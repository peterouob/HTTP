use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use crate::parse::parser::{HeaderMap, Request, Status};

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
        let mut tmp = [0u8;4096];
        let mut read_buf = ReadBuf::new(&mut tmp);

        match Pin::new(&mut self.stream).poll_read(cx,&mut read_buf) {
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

    pub fn consume(&mut self, n:usize) {
        let _ = self.buffer.split_to(n);
    }

    pub fn write(&mut self, cx: &mut Context<'_>, buf: &mut BytesMut) -> Poll<std::io::Result<usize>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0))
        }

        match Pin::new(&mut self.stream).poll_write(cx,buf.as_ref()) {
            Poll::Ready(Ok(n)) => {
                buf.advance(n);
                Poll::Ready(Ok(n))
            },
            other => other
        }
    }

    pub fn flush(&mut self, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    pub fn shutdown(&mut self, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

pub struct Connection<S:AsyncStream> {
    state: State,
    buf: BufferStream<S>,
    last_parse_pos: usize,
}

enum State {
    Idle,
    ReadingHead,
    ParsingHead,
    WritingResponse,
    Flushing,
    Closing,
    Closed,
}

enum ParseState{
    Complete(usize),
    Partial,
    Error,
}

impl<S:AsyncStream> Connection<S> {
    pub fn new(stream: S) -> Self {
        Self {
            state: State::Idle,
            buf: BufferStream::new(stream),
            last_parse_pos: 0,
        }
    }
    fn try_parse(&self) -> ParseState{
        let mut header = HeaderMap::new();
        let mut req = Request::new(&mut header);

        match req.parse_header(self.buf.buf()) {
            Ok(Status::Complete(n)) => ParseState::Complete(n),
            Ok(Status::Partial) => ParseState::Partial,
            Err(_) => ParseState::Error,
        }
    }
}

impl<S:AsyncStream> Future for Connection<S> {
    type Output = anyhow::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.state {
                State::Idle => self.state = State::ReadingHead,
                State::ReadingHead => {
                    match self.buf.poll_fill_buf(cx) {
                        Poll::Ready(Ok(0)) | Poll::Ready(Err(_)) => self.state = State::Closing,
                        Poll::Ready(Ok(n)) => {
                            self.last_parse_pos = n;
                            self.state = State::ParsingHead
                        },
                        Poll::Pending => return Poll::Pending,
                    }
                },
                State::ParsingHead => {
                    match self.try_parse() {
                        ParseState::Complete(n) => {
                            self.buf.consume(n);
                            self.state = State::WritingResponse;
                        },
                        ParseState::Partial => self.state = State::ReadingHead,
                        ParseState::Error => self.state = State::Closing
                    }
                },
                State::WritingResponse => {
                    let mut write_buf = BytesMut::with_capacity(1024);
                    match self.buf.write(cx, &mut write_buf) {
                        Poll::Ready(Ok(n)) => {
                            if write_buf.is_empty() {
                                self.last_parse_pos = n;
                                self.state = State::Flushing;
                                drop(write_buf);
                            }
                        },
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(_)) => self.state = State::Closing,
                    }
                },
                State::Flushing => {
                    match self.buf.flush(cx) {
                        Poll::Ready(Ok(())) => {
                            self.state = State::Idle;
                        },
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(_)) => self.state = State::Closing,
                    }
                },
                State::Closing => {
                    match self.buf.shutdown(cx) {
                        Poll::Ready(_) => self.state = State::Closed,
                        Poll::Pending => return Poll::Pending,
                    }
                },
                State::Closed => return Poll::Ready(Ok(())),
            }
        }
    }
}