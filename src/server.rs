use std::io::ErrorKind;
use std::net::{SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore};
use crate::{Shutdown,Connection};
use anyhow::{anyhow, Result};
use rand::RngExt;
use socket2::{Domain, Protocol, Type};
use tokio::task::JoinSet;
use tokio::time;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use crate::error::TCPSocketError;

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

pub async fn run(addr:SocketAddr,shutdown: impl Future) {
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
    async fn run(&mut self) -> Result<()> {
        info!("accepting inbound connections");
        loop {
            let permit = self.limit_connections
                .clone()
                .acquire_owned()
                .await?;

            let socket = self.accept().await?;
            let child_token = self.token.clone();

            let mut handler = Handle {
                connection: Connection::new(socket),
                shutdown: Shutdown::new(child_token),
            };

            tokio::spawn(async move {
                if let Err(err) = handler.run().await {
                    if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
                        match io_err.kind() {
                            ErrorKind::ConnectionAborted |
                            ErrorKind::BrokenPipe |
                            ErrorKind::ConnectionReset => {
                                info!(cause = ?io_err, "listener closed, shutting down");
                            }

                            _ => error!(cause = ?io_err, "unexpected io error"),
                        }
                    }else {
                        error!(cause = ?err, "unexpected error in listener");
                    }
                }

                drop(permit);
            });
        }
    }

    async fn accept(&mut self) -> Result<tokio::net::TcpStream> {
        let mut backoff = Duration::from_millis(1).as_secs_f32();
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

            let max_delay = Duration::from_millis(120).as_secs_f32();

            if backoff >= max_delay {
                retries -= 1;
                time::sleep(Duration::from_millis(max_delay as u64)).await;
            }

            backoff = (backoff * 1.1).min(max_delay);
            let mut rng = rand::rng();
            let jittered = backoff * (1.0 + 0.2 * rng.random::<f32>());
            let sleep_time = jittered.min(max_delay);

            retries -= 1;
            time::sleep(Duration::from_millis(sleep_time as u64)).await;
        }
    }
}

pub fn setup_tcp( addr: SocketAddr) -> Result<TcpListener> {
    let socket = socket2::Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP)).map_err(|_| TCPSocketError::SocketConfig("failed to create socket".to_string()))?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;
    socket.set_tcp_nodelay(true)?;
    socket.bind(&addr.into())?;
    socket.listen(128)?;

    let std_listen: std::net::TcpListener = socket.into();
    Ok(TcpListener::from_std(std_listen)?)
}


impl Handle {
    pub async fn run(&mut self) -> Result<()>{
        while !self.shutdown.is_shutdown() {

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

            self.connection.write_frame(&frame).await?;
        }

        Ok(())
    }
}