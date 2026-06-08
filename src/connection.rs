use bytes::BytesMut;
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

    // read_frame TODO: loop read and parse http frame
    pub async fn read_frame(&mut self) -> anyhow::Result<Option<String>> {
        let n = self.stream.read_buf(&mut self.buffer).await?;

        if n == 0 {
            return Ok(None);
        }

        let data = String::from_utf8(self.buffer.to_vec())?;
        self.buffer.clear();
        Ok(Some(data))
    }

    pub async fn write_frame(&mut self, frame: &[u8]) -> anyhow::Result<()> {
        self.stream.write_all(frame).await?;
        self.stream.flush().await?;
        Ok(())
    }
}
