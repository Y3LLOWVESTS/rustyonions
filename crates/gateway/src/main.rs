// crates/gateway/src/main.rs
#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use clap::Parser;

mod routes;
mod state;
mod utils;
mod pay_enforce; // you already created this in the previous step

use routes::build_router;
use state::AppState;

/// Gateway CLI args
#[derive(Parser, Debug, Clone)]
#[command(name = "gateway", version)]
struct Args {
    /// Path to the index database (Sled dir)
    #[arg(long, default_value = ".data/index")]
    pub index_db: std::path::PathBuf,

    /// Bind address (use 127.0.0.1:0 for ephemeral port)
    #[arg(long, default_value = "127.0.0.1:0")]
    pub bind: SocketAddr,

    /// If set, enforce `[payment].required = true` with HTTP 402
    #[arg(long)]
    pub enforce_payments: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Build shared state
    let state = Arc::new(AppState::new(args.index_db.clone(), args.enforce_payments));

    // Router
    let app: Router = build_router(state);

    // Bind + run
    let listener = tokio::net::TcpListener::bind(args.bind).await?;
    let local = listener.local_addr()?;
    println!("gateway listening on http://{}", local);

    axum::serve(listener, app).await?;
    Ok(())
}
