//! RO:WHAT — svc-wallet binary entrypoint for the Phase 2 HTTP service shell.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES/DX. Runs the wallet API boundary around the ron-ledger adapter.
//! RO:INTERACTS — supervisor, routes, readiness, metrics, tokio/axum runtime.
//! RO:INVARIANTS — no durable wallet truth; dev mode uses RAM/amnesia-safe state; graceful shutdown.
//! RO:METRICS — exposes /metrics through routes.
//! RO:CONFIG — SVC_WALLET_ADDR override; default 127.0.0.1:8088.
//! RO:SECURITY — dev verifier requires nonempty bearer token; production verifier plugs into auth module later.
//! RO:TEST — cargo run -p svc-wallet; HTTP smoke in Phase 3.

use std::{net::SocketAddr, str::FromStr};

use svc_wallet::{routes, supervisor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "svc_wallet=info,tower_http=info".into()),
        )
        .try_init();

    let addr = std::env::var("SVC_WALLET_ADDR")
        .ok()
        .and_then(|value| SocketAddr::from_str(&value).ok())
        .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8088)));

    let state = supervisor::build_dev_state()?;
    let app = routes::router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;

    tracing::info!(%local_addr, "svc-wallet listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(supervisor::shutdown_signal())
        .await?;

    Ok(())
}
