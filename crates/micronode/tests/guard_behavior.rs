//! RO:WHAT — Integration tests for Micronode guard behavior (decode + body cap).
//! RO:WHY  — Assert that `DecodeGuard` and `BodyCapLayer` behave as specified
//!           on real HTTP routes (no mock services).
//!
//! RO:INVARIANTS —
//!   - Any `Content-Encoding` on guarded routes yields 415.
//!   - Payloads over `HTTP_BODY_CAP_BYTES` yield 413.
//!
//! These tests exercise `/dev/echo`, which is wired with:
//!   DecodeGuard -> BodyCapLayer -> ConcurrencyLayer -> handler.

use std::{net::SocketAddr, time::Duration};

use micronode::app::build_router;
use micronode::config::schema::{Config, Server};
use micronode::limits::HTTP_BODY_CAP_BYTES;
use reqwest::StatusCode;
use tokio::task::JoinHandle;

/// Spawn an in-process Micronode instance on an ephemeral port, with
/// dev routes enabled and readiness probes flipped to "healthy".
async fn spawn_micronode() -> (SocketAddr, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind test listener");
    let addr = listener.local_addr().expect("get local address for test listener");

    let cfg = Config { server: Server { bind: addr, dev_routes: true }, ..Config::default() };

    let (router, state) = build_router(cfg);

    state.probes.set_cfg_loaded(true);
    state.probes.set_listeners_bound(true);
    state.probes.set_metrics_bound(true);
    state.probes.set_deps_ok(true);

    let handle = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, router).await {
            eprintln!("[micronode-guard-test] server error: {err}");
        }
    });

    (addr, handle)
}

#[tokio::test]
async fn decode_guard_rejects_any_content_encoding() {
    let (addr, _handle) = spawn_micronode().await;
    let base = format!("http://{}", addr);
    let url = format!("{base}/dev/echo");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("build reqwest client");

    let resp = client
        .post(&url)
        .header("content-type", "application/json")
        .header("content-encoding", "gzip")
        .body(r#"{"message":"hi"}"#)
        .send()
        .await
        .expect("POST /dev/echo with content-encoding");

    assert_eq!(
        resp.status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "expected 415 from DecodeGuard on any Content-Encoding, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn body_cap_enforces_max_payload_size() {
    let (addr, _handle) = spawn_micronode().await;
    let base = format!("http://{}", addr);
    let url = format!("{base}/dev/echo");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("build reqwest client");

    // Construct a payload that is one byte over the configured cap.
    let over_cap_len = (HTTP_BODY_CAP_BYTES as usize).saturating_add(1);
    let payload = vec![b'a'; over_cap_len];

    let resp = client
        .post(&url)
        .header("content-type", "application/json")
        .body(payload)
        .send()
        .await
        .expect("POST /dev/echo with over-cap payload");

    assert_eq!(
        resp.status(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "expected 413 from BodyCapLayer for payload > HTTP_BODY_CAP_BYTES, got {}",
        resp.status()
    );
}
