use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_echo_server() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();

    stream.write_all(b"hello\n").await.unwrap();
    let mut buf = vec![0u8; 64];
    let n = stream.read(&mut buf).await.unwrap();

    let response = String::from_utf8_lossy(&buf[..n]);
    assert_eq!(response, "hello\n");
}
