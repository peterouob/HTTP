use std::net::SocketAddr;
use anyhow::{Result};
use tokio::signal;
use tcp2http::server::run;

#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() -> Result<()> {
    // console_subscriber::init();
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    run(addr,signal::ctrl_c()).await;
    Ok(())
}