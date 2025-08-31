#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::path::PathBuf;

use axum::Router;
use tokio::task::JoinHandle;

use gateway::index_client::IndexClient;
use gateway::routes::router;
use gateway::state::AppState;

async fn spawn_gateway(enforce_payments: bool) -> (JoinHandle<()>, SocketAddr) {
    // IndexClient pulls socket from RON_INDEX_SOCK (set in CI), fallback used locally.
    let index = IndexClient::from_env_or("/tmp/ron/svc-index.sock");
    let state = AppState::new(index, enforce_payments);
    let app: Router = router(state);

    // Bind to ephemeral port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (handle, addr)
}

#[tokio::test]
async fn free_bundle_returns_200() {
    // assumes svc-index has a mapping loaded in CI step: addr -> dir with payload.bin
    std::env::set_var("RON_INDEX_SOCK", "/tmp/ron/svc-index.sock");

    let (_h, addr) = spawn_gateway(false).await;
    let url = format!("http://{}/o/{}/payload.bin", addr, "b3:freehash.text");

    let resp = reqwest::get(url).await.unwrap();
    assert!(resp.status().is_success(), "expected 200 for free bundle");
}

#[tokio::test]
async fn paid_bundle_returns_402() {
    std::env::set_var("RON_INDEX_SOCK", "/tmp/ron/svc-index.sock");

    let (_h, addr) = spawn_gateway(true).await;
    let url = format!("http://{}/o/{}/payload.bin", addr, "b3:paidhash.text");

    let resp = reqwest::get(url).await.unwrap();
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::PAYMENT_REQUIRED,
        "expected 402 for paid bundle"
    );
}
