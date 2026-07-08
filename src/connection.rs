use crate::parse::parser::{HeaderMap, Request, Status};
use bytes::{Buf, BytesMut};
use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;

trait AsyncStream: AsyncRead + AsyncWrite + Unpin {}
impl AsyncStream for TcpStream {}

struct BufferStream<S: AsyncStream> {
    stream: S,
    buffer: BytesMut,
}

impl<S: AsyncStream> BufferStream<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            buffer: BytesMut::with_capacity(1024 * 8),
        }
    }

    pub fn poll_fill_buf(&mut self, cx: &mut Context<'_>) -> Poll<std::io::Result<usize>> {
        let mut tmp = [0u8; 4096];
        let mut read_buf = ReadBuf::new(&mut tmp);

        match Pin::new(&mut self.stream).poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                let n = read_buf.filled().len();
                self.buffer.extend_from_slice(&read_buf.filled());
                Poll::Ready(Ok(n))
            }
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Pending => Poll::Pending,
        }
    }

    pub fn buf(&self) -> &[u8] {
        self.buffer.as_ref()
    }

    pub fn consume(&mut self, n: usize) {
        let _ = self.buffer.split_to(n);
    }

    pub fn write(
        &mut self,
        cx: &mut Context<'_>,
        buf: &mut BytesMut,
    ) -> Poll<std::io::Result<usize>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        match Pin::new(&mut self.stream).poll_write(cx, buf.as_ref()) {
            Poll::Ready(Ok(n)) => {
                buf.advance(n);
                Poll::Ready(Ok(n))
            }
            other => other,
        }
    }

    pub fn flush(&mut self, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    pub fn shutdown(&mut self, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

pub struct Connection<S: AsyncStream> {
    state: State,
    buf: BufferStream<S>,
    last_parse_pos: usize,
    write_buf: BytesMut,
}

#[derive(Debug, PartialEq)]
enum State {
    Idle,
    ReadingHead,
    ParsingHead,
    WritingResponse,
    Flushing,
    Closing,
    Closed,
}

#[derive(Debug, PartialEq)]
enum ParseState {
    Complete(usize),
    Partial,
    Error,
}

impl<S: AsyncStream> Connection<S> {
    pub fn new(stream: S) -> Self {
        Self {
            state: State::Idle,
            buf: BufferStream::new(stream),
            last_parse_pos: 0,
            write_buf: BytesMut::with_capacity(1024),
        }
    }
    fn try_parse(&self) -> ParseState {
        let mut header = HeaderMap::new();
        let mut req = Request::new(&mut header);

        match req.parse_header(self.buf.buf()) {
            Ok(Status::Complete(n)) => ParseState::Complete(n),
            Ok(Status::Partial) => ParseState::Partial,
            Err(_) => ParseState::Error,
        }
    }
}

impl<S: AsyncStream> Future for Connection<S> {
    type Output = anyhow::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            match this.state {
                State::Idle => this.state = State::ReadingHead,
                State::ReadingHead => match this.buf.poll_fill_buf(cx) {
                    Poll::Ready(Ok(0)) | Poll::Ready(Err(_)) => this.state = State::Closing,
                    Poll::Ready(Ok(n)) => {
                        this.last_parse_pos = n;
                        this.state = State::ParsingHead
                    }
                    Poll::Pending => return Poll::Pending,
                },
                State::ParsingHead => match this.try_parse() {
                    ParseState::Complete(n) => {
                        this.buf.consume(n);
                        this.state = State::WritingResponse;
                    }
                    ParseState::Partial => this.state = State::ReadingHead,
                    ParseState::Error => this.state = State::Closing,
                },
                State::WritingResponse => {
                    match this.buf.write(cx, &mut this.write_buf) {
                        Poll::Ready(Ok(n)) => {
                            if this.write_buf.is_empty() {
                                this.last_parse_pos = n;
                                this.state = State::Flushing;
                            }
                        }
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(_)) => this.state = State::Closing,
                    }
                }
                State::Flushing => match this.buf.flush(cx) {
                    Poll::Ready(Ok(())) => {
                        this.state = State::Idle;
                    }
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Err(_)) => this.state = State::Closing,
                },
                State::Closing => match this.buf.shutdown(cx) {
                    Poll::Ready(_) => this.state = State::Closed,
                    Poll::Pending => return Poll::Pending,
                },
                State::Closed => return Poll::Ready(Ok(())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::poll_fn;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

    impl AsyncStream for DuplexStream {}

    async fn poll_once<F: Future + Unpin>(f: &mut F) -> Poll<F::Output> {
        poll_fn(|cx| Poll::Ready(Pin::new(&mut *f).poll(cx))).await
    }

    #[tokio::test]
    async fn test_connection_state_machine_with_real_io() {
        let (mut client, server) = tokio::io::duplex(1024);

        let mut conn = Connection::new(server);

        assert_eq!(conn.state, State::Idle);

        let poll_result = poll_once(&mut conn).await;
        assert!(poll_result.is_pending());
        assert_eq!(conn.state, State::ReadingHead);

        client.write_all(b"GET / HTTP/1.1\r\n").await.unwrap();

        let poll_result = poll_once(&mut conn).await;
        assert!(poll_result.is_pending());
        assert_eq!(conn.state, State::ReadingHead);

        client.write_all(b"Host: localhost\r\n\r\n").await.unwrap();

        let poll_result = poll_once(&mut conn).await;
        assert!(poll_result.is_pending());
        assert_eq!(conn.state, State::ReadingHead,);

        // let mut response = [0u8; 1024];
        // let n = client.read(&mut response).await.unwrap();
        // assert!(n > 0);

        drop(client);

        let poll_result = poll_once(&mut conn).await;

        assert!(matches!(poll_result, Poll::Ready(Ok(()))));
        assert_eq!(
            conn.state,
            State::Closed,
            "Client 斷線，完美進入 Closed 狀態"
        );
    }
}
