// crates/gateway/src/main.rs
#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use axum::Router;
use clap::Parser;

mod index_client; // svc-index client (kept for other calls if needed)
mod overlay_client; // svc-overlay client (new)
mod pay_enforce; // manifest payment guard (402)
mod routes; // HTTP routes -> uses OverlayClient (+ optional pay guard)
mod state; // AppState holds IndexClient + OverlayClient + flag
mod utils; // basic_headers etc.

use crate::index_client::IndexClient;
use crate::overlay_client::OverlayClient;
use crate::routes::router;
use crate::state::AppState;

#[derive(Parser, Debug)]
struct Args {
    /// Bind address for the HTTP server (use 127.0.0.1:0 for any free port).
    #[arg(long, default_value = "127.0.0.1:54087")]
    bind: SocketAddr,

    /// Path to old index DB (kept for compat in CLI; not used in overlay mode).
    #[arg(long, default_value = ".data/index")]
    #[allow(dead_code)]
    index_db: PathBuf,

    /// Enforce payment requirements from Manifest.toml (returns 402).
    #[arg(long, default_value_t = false)]
    enforce_payments: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Build clients from env sockets (with sensible fallbacks).
    let index_client = IndexClient::from_env_or("/tmp/ron/svc-index.sock");
    let overlay_client = OverlayClient::from_env_or("/tmp/ron/svc-overlay.sock");

    // State includes both clients (index kept for future use).
    let state = AppState::new(index_client, overlay_client, args.enforce_payments);

    // Build router
    let app: Router = router(state);

    // Bind + run
    let listener = tokio::net::TcpListener::bind(args.bind).await?;
    let local = listener.local_addr()?;
    println!("gateway listening on http://{}", local);

    axum::serve(listener, app).await?;
    Ok(())
}
