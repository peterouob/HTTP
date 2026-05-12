use bytes::{BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;

#[derive(Debug)]
pub struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(4 * 1024),
        }
    }

    pub async fn read_frame(&mut self) -> anyhow::Result<Option<String>>{
        loop {
            if self.stream.read_buf(&mut self.buffer).await? == 0 {
                return Ok(None);
            }

            if !self.buffer.is_empty() {
                let res = "HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\nHello World!";
                let frame = res.to_string();
                self.buffer.clear();
                return Ok(Some(frame));
            }
        }
    }

    pub async fn write_frame(&mut self, frame: &str) -> anyhow::Result<()> {
        self.stream.write_all(frame.as_bytes()).await?;
        self.stream.flush().await?;
        Ok(())
    }
}