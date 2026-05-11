use std::net::SocketAddr;
use std::sync::Arc;
use socket2::{Domain, Protocol, Type};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info_span, instrument, Instrument};
use anyhow::{Context, Result};
use tokio::sync::Semaphore;
use http::error::TCPSocketError;
use bytes::{BytesMut};


/*
NOW

wrk -t8 -c1000 -d60s --latency http://localhost:8080/                                          took 1m0s
Running 1m test @ http://localhost:8080/
  8 threads and 1000 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     5.23ms    3.99ms  88.03ms   86.54%
    Req/Sec    18.44k     2.73k   49.36k    71.61%
  Latency Distribution
     50%    4.34ms
     75%    6.68ms
     90%    9.42ms
     99%   17.35ms
  8804283 requests in 1.00m, 428.22MB read
Requests/sec: 146497.03
Transfer/sec:      7.13MB
*/

#[instrument]
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let permit = Arc::new(Semaphore::new(4096));

    let addr : SocketAddr = "0.0.0.0:8080".parse().context("failed to parse socket address")?;
    let listener = setup_tcp(addr).context("failed to setup tcp listener")?;

    // TODO: graceful shutdown
    let token = CancellationToken::new();
    let (tx, mut rx) = mpsc::channel::<()>(1);

    loop {
        let permit = Arc::clone(&permit);

        match listener.accept().await.context("failed to accept connection") {

            Ok((stream,addr)) => {
                tokio::spawn(async move {
                    let _permit = permit.acquire().await.unwrap();

                    handle_connection(stream)
                            .instrument(info_span!("handle_connection", client_addr = %addr))
                            .await
                }
                );
            },
            Err(e) => {
                error!("{}", e);
            }
        }
    }
}

fn setup_tcp(addr: SocketAddr) -> Result<tokio::net::TcpListener> {
    let socket = socket2::Socket::new(Domain::IPV4,Type::STREAM,Some(Protocol::TCP)).map_err(|_| TCPSocketError::SocketConfig("failed to create socket".to_string()))?;
    socket.set_reuse_address(true).map_err(|_| TCPSocketError::SocketConfig("failed to set SO_REUSEADDR".to_string()))?;
    socket.set_nonblocking(true).map_err(|_| TCPSocketError::SocketConfig("failed to set SO_NONBLOCK".to_string()))?;
    socket.set_tcp_nodelay(true).map_err(|_| TCPSocketError::SocketConfig("failed to set TCP_NODELAY".to_string()))?;
    socket.bind(&addr.into()).context("failed to bind socket")?;
    socket.listen(4096).context("failed to set listen backlog")?;

    let std_listen: std::net::TcpListener = socket.into();
    Ok(tokio::net::TcpListener::from_std(std_listen)?)
}

#[instrument(skip(stream))]
async fn handle_connection(mut stream: tokio::net::TcpStream) {
    let mut buffer= BytesMut::with_capacity(1024);
    loop {
         match stream.read_buf(&mut buffer).await {
             Ok(0) => break,
            Ok(_) => {
                let response = "HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\nHello World!";
                stream.write_all(response.as_bytes()).await.ok();
                buffer.clear();
            },
            Err(err) => {
                error!("failed to read from socket; err={:?}", err);
                break
            }
        }
    }
}