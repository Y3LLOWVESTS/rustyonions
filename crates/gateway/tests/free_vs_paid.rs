// FILE: crates/gateway/tests/free_vs_paid.rs
#![forbid(unsafe_code)]

use anyhow::Result;
use std::net::SocketAddr;

use axum::{extract::Extension, Router};
use tokio::task::JoinHandle;

use gateway::index_client::IndexClient;
use gateway::overlay_client::OverlayClient;
use gateway::routes::router;
use gateway::state::AppState;

async fn spawn_gateway(enforce_payments: bool) -> Result<(JoinHandle<()>, SocketAddr)> {
    // Clients pull sockets from env, with sensible fallbacks.
    let index = IndexClient::from_env_or("/tmp/ron/svc-index.sock");
    let overlay = OverlayClient::from_env_or("/tmp/ron/svc-overlay.sock");
    let state = AppState::new(index, overlay, enforce_payments);

    // Build stateless router and attach state via Extension layer (not with_state).
    let app: Router = router().layer(Extension(state));

    // Bind to ephemeral port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    Ok((handle, addr))
}

#[tokio::test]
async fn free_bundle_returns_200() -> Result<()> {
    // assumes svc-index/overlay have a mapping loaded in CI step
    std::env::set_var("RON_INDEX_SOCK", "/tmp/ron/svc-index.sock");
    std::env::set_var("RON_OVERLAY_SOCK", "/tmp/ron/svc-overlay.sock");

    let (_h, addr) = spawn_gateway(false).await?;
    let url = format!("http://{}/o/{}/payload.bin", addr, "b3:freehash.text");

    let resp = reqwest::get(url).await?;
    assert!(resp.status().is_success(), "expected 200 for free bundle");
    Ok(())
}

#[tokio::test]
async fn paid_bundle_returns_402() -> Result<()> {
    std::env::set_var("RON_INDEX_SOCK", "/tmp/ron/svc-index.sock");
    std::env::set_var("RON_OVERLAY_SOCK", "/tmp/ron/svc-overlay.sock");

    let (_h, addr) = spawn_gateway(true).await?;
    let url = format!("http://{}/o/{}/payload.bin", addr, "b3:paidhash.text");

    let resp = reqwest::get(url).await?;
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::PAYMENT_REQUIRED,
        "expected 402 for paid bundle"
    );
    Ok(())
}
