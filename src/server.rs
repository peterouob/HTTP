use crate::error::TCPSocketError;
use crate::parse::parser::Status::Complete;
use crate::parse::parser::{HeaderMap, Request, Response, Status};
use crate::{Connection, Shutdown};
use anyhow::{Result, anyhow};
use rand::RngExt;
use socket2::{Domain, Protocol, Type};
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use bytes::BytesMut;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

#[derive(Debug)]
pub struct Listener {
    listener: TcpListener,
    limit_connections: Arc<Semaphore>,
    token: CancellationToken,
    join_set: JoinSet<()>,
}

#[derive(Debug)]
pub struct Handle {
    connection: Connection,
    shutdown: Shutdown,
}

const MAX_CONNECTIONS: usize = 4096;

pub async fn run(addr: SocketAddr, shutdown: impl Future) {
    let token = CancellationToken::new();
    let join_set = JoinSet::new();

    let listener = setup_tcp(addr).unwrap();

    let mut server = Listener {
        listener,
        limit_connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        token: token.clone(),
        join_set,
    };

    tokio::select! {
        res = server.run() => {
            if let Err(err) = res {
                error!("server exited with error: {}", err);
            }
        }

        _ = shutdown => {
            info!("shutting down");
        }
    }

    token.cancel();

    tokio::select! {
        _ = server.join_set.join_all() => {
            info!("all connections closed, shutdown complete");
        }

        _ = time::sleep(Duration::from_secs(30)) => {
            error!("timeout waiting for connections to close, forcing shutdown");
        }
    }
}

impl Listener {
    // TODO: use FSM mange the state of server, now is the very simple example to show the req/res can work well
    // SO NOW the represent page is from claud and when the router done then i will fix here
    // i want to use router like the golang package which name is "gin" so i need some time to learning
    async fn run(&mut self) -> Result<()> {
        info!("accepting inbound connections");
        loop {
            let permit = self.limit_connections.clone().acquire_owned().await?;

            let socket = match self.accept().await {
                Ok(socket) => socket,
                Err(err) => {
                    error!(cause = ?err, "failed to accept connection");
                    continue;
                }
            };

            let child_token = self.token.clone();

            let mut handler = Handle {
                connection: Connection::new(socket),
                shutdown: Shutdown::new(child_token),
            };

            self.join_set.spawn(async move {
                if let Err(err) = handler.run().await {
                    if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
                        match io_err.kind() {
                            ErrorKind::ConnectionAborted
                            | ErrorKind::BrokenPipe
                            | ErrorKind::ConnectionReset => {
                                info!(cause = ?io_err, "listener closed, shutting down");
                            }

                            _ => error!(cause = ?io_err, "unexpected io error"),
                        }
                    } else {
                        error!(cause = ?err, "unexpected error in listener");
                    }
                }

                drop(permit);
            });
        }
    }

    async fn accept(&mut self) -> Result<tokio::net::TcpStream> {
        let mut backoff = 1.0_f32;
        let max_delay = 120.0_f32;
        let mut retries = 5;
        loop {
            match self.listener.accept().await {
                Ok((socket, _)) => return Ok(socket),
                Err(err) => {
                    if retries == 0 {
                        return Err(anyhow!(err));
                    }
                }
            }

            let mut rng = rand::rng();
            let jittered = backoff * (1.0 + 0.2 * rng.random::<f32>());
            let sleep_time = jittered.min(max_delay);

            time::sleep(Duration::from_millis(sleep_time as u64)).await;
            backoff = (backoff * 1.1).min(max_delay);

            retries -= 1;
        }
    }
}

pub fn setup_tcp(addr: SocketAddr) -> Result<TcpListener> {
    let socket = socket2::Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))
        .map_err(|_| TCPSocketError::SocketConfig("failed to create socket".to_string()))?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;
    socket.set_tcp_nodelay(true)?;
    socket.bind(&addr.into())?;
    socket.listen(128)?;

    let std_listen: std::net::TcpListener = socket.into();
    Ok(TcpListener::from_std(std_listen)?)
}

impl Handle {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            let maybe_frame = tokio::select! {
                res = self.connection.read_frame() => res?,
                _ = self.shutdown.recv() => {
                    info!("shutdown signal received, shutting down connection");
                    return Ok(());
                }
            };

            // TODO: use http frame instead currently hardcode message
            let frame = match maybe_frame {
                Some(frame) => frame,
                None => return Ok(()),
            };

            let mut headers = HeaderMap::new();
            let mut req = Request::new(&mut headers);

            let result = req.parse_header(frame.as_ref());

            let response_bytes = match result {
                Ok(Complete(())) => {
                    build_hello_response(&req)
                }
                Ok(Status::Partial) => {
                    build_400_response()
                }
                Err(_) => {
                    build_400_response()
                }
            };

            drop(req);
            drop(headers);
            self.connection.write_frame(&response_bytes).await?;
        }
    }
}

// HACK: THESE CODE IS GENERATE FROM AI SO WE NEED TO REWRITE LATER WHEN THE ROUTER WORK DONE
fn build_hello_response(req: &Request) -> Vec<u8> {
    let body = format!(
        "<!DOCTYPE html>\n\
         <html>\n\
         <head><title>Hello</title></head>\n\
         <body>\n\
           <h1>Hello from Rust!</h1>\n\
           <p>Method: {}</p>\n\
           <p>Path: {}</p>\n\
         </body>\n\
         </html>\n",
        req.method.unwrap_or("?"),
        req.uri.unwrap_or("?"),
    );
    let body_bytes = body.into_bytes();

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", b"text/html; charset=utf-8");
    let len = body_bytes.len().to_string();
    headers.insert("Content-Length", len.as_bytes());
    headers.insert("Connection", b"close");

    let resp = Response {
        version: Some(1),
        status_code: Some(200),
        reason: Some("OK"),
        headers: &mut headers,
    };

    let mut out = BytesMut::with_capacity(512);
    resp.write_to(&mut out);
    out.extend_from_slice(&body_bytes);
    out.to_vec()
}

fn build_400_response() -> Vec<u8> {
    let body = b"<h1>400 Bad Request</h1>";

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", b"text/html");
    let len = body.len().to_string();
    headers.insert("Content-Length", len.as_bytes());
    headers.insert("Connection", b"close");

    let resp = Response {
        version: Some(1),
        status_code: Some(400),
        reason: Some("Bad Request"),
        headers: &mut headers,
    };

    let mut out = BytesMut::with_capacity(512);
    resp.write_to(&mut out);
    out.extend_from_slice(body);
    out.to_vec()
}