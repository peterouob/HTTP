use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tcp2http::server::run;
use tokio::signal;
use tcp2http::router::engine::Engine;

#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8081));
    let mut e = Engine::new();
    e.get(b"/", |ctx|{
        ctx.html("<h1>Hello, world!</h1>");
        ctx.status(200);
    });
    e.post(b"/", |ctx| {
        ctx.html("<h1>Hello, world! POST</h1>");
        ctx.status(200);
    });

    run(addr, signal::ctrl_c(), Arc::from(e)).await;
    Ok(())
}
