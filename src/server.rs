use crate::error::TCPSocketError;
use crate::parse::parser::Status::Complete;
use crate::parse::parser::{HeaderMap, Request, Response, Status};
use crate::router::engine::Engine;
use crate::{Connection, Shutdown};
use anyhow::{Result, anyhow};
use rand::RngExt;
use socket2::{Domain, Protocol, Type};
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub struct Listener {
    listener: TcpListener,
    limit_connections: Arc<Semaphore>,
    token: CancellationToken,
    join_set: JoinSet<()>,
    engine: Arc<Engine<'static>>,
}

const MAX_CONNECTIONS: usize = 4096;

pub async fn run(addr: SocketAddr, shutdown: impl Future, engine: Arc<Engine<'static>>) {
    let token = CancellationToken::new();
    let join_set = JoinSet::new();

    let listener = setup_tcp(addr).unwrap();

    let mut server = Listener {
        listener,
        limit_connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        token: token.clone(),
        join_set,
        engine,
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
            let engine = Arc::clone(&self.engine);
            let mut shutdown = Shutdown::new(child_token);

            let conn = Connection::new(socket);

            self.join_set.spawn(async move {
                conn.await;
                drop(permit);
            });

            tokio::select! {
                _ = self.token.cancelled() => {
                    info!("shutdown signal received, shutting down connection");
                    return Ok(())
                }
                _ = shutdown.recv() => {
                    info!("shutdown signal received, shutting down connection");
                }
            }
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
