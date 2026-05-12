use anyhow::Result;
use std::net::SocketAddr;
use tcp2http::server::run;
use tokio::signal;

#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() -> Result<()> {
    // console_subscriber::init();
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    run(addr, signal::ctrl_c()).await;
    Ok(())
}
