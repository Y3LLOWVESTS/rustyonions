// crates/svc-passport/src/bootstrap.rs
//! RO:WHAT   TCP listener + Axum server bootstrap.
//! RO:WHY    Keep signature expected by main.rs (bind, admin, cfg) -> (JoinHandle, addr).
//!           Print the *actual* bound addr so curl targets the right port.
//!           Router is unit-state so `axum::serve(listener, app)` compiles cleanly.

use crate::{config::Config, health::Health, http::router::build_router};
use anyhow::Result;
use std::net::SocketAddr;
use tokio::{net::TcpListener, task::JoinHandle};

pub async fn run(
    bind: SocketAddr,
    _admin: SocketAddr,
    cfg: Config,
) -> Result<(JoinHandle<()>, SocketAddr)> {
    // 1) Bind TCP listener (explicit address from main.rs)
    let listener = TcpListener::bind(bind).await?;
    let addr = listener.local_addr()?; // in case port=0, this tells you what you actually got

    // 2) Loud startup so you can't miss it (works even if tracing isn't configured)
    println!("svc-passport: listening on http://{addr}");
    eprintln!("svc-passport: listening on http://{addr}");

    // 3) Health (Default ok)
    let health: Health = Default::default();

    // 4) Build Router<()> (IssuerState is carried via Extension inside build_router)
    let app = build_router(cfg.clone(), health);

    // 5) Serve (await inside a task so main can hold the JoinHandle)
    let task = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("svc-passport server error: {e}");
        }
    });

    Ok((task, addr))
}
