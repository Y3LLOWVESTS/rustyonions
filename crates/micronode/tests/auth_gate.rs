//! RO:WHAT — Integration tests for Micronode auth enforcement (MVP layer).
//! RO:WHY  — Prove deny-by-default semantics and DX-friendly dev_allow mode.
//! RO:TESTS —
//!   1) deny_all without header -> 401 + WWW-Authenticate
//!   2) deny_all with header    -> 403
//!   3) dev_allow without header -> 201/200 on KV

use std::{net::SocketAddr, time::Duration};

use micronode::app::build_router;
use micronode::config::schema::{Config, SecurityCfg, SecurityMode, Server};
use reqwest::StatusCode;
use tokio::task::JoinHandle;

/// Spawn an in-process Micronode instance on an ephemeral port with the given security mode.
async fn spawn_with_security(mode: SecurityMode) -> (SocketAddr, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind test listener");
    let addr = listener.local_addr().expect("local address");

    let cfg = Config {
        server: Server { bind: addr, dev_routes: false },
        security: SecurityCfg { mode },
        ..Config::default()
    };

    let (router, state) = build_router(cfg);

    // Make readiness truthful for the in-memory engine.
    state.probes.set_cfg_loaded(true);
    state.probes.set_listeners_bound(true);
    state.probes.set_metrics_bound(true);
    state.probes.set_deps_ok(true);

    let handle = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, router).await {
            eprintln!("[micronode-auth-test] server error: {err}");
        }
    });

    (addr, handle)
}

#[tokio::test]
async fn deny_all_without_header_yields_401_www_authenticate() {
    let (addr, _handle) = spawn_with_security(SecurityMode::DenyAll).await;
    let base = format!("http://{}", addr);
    let key_url = format!("{base}/v1/kv/a/k");

    let client =
        reqwest::Client::builder().timeout(Duration::from_secs(2)).build().expect("client");

    // Attempt a PUT without Authorization: expect 401 + WWW-Authenticate
    let put = client
        .put(&key_url)
        .header("content-type", "application/octet-stream")
        .body("hello")
        .send()
        .await
        .expect("PUT missing auth");
    assert_eq!(put.status(), StatusCode::UNAUTHORIZED, "expected 401, got {}", put.status());
    let hdr = put.headers().get("www-authenticate").and_then(|v| v.to_str().ok()).unwrap_or("");
    assert!(
        hdr.to_ascii_lowercase().contains("macro"),
        "expected Macro scheme in WWW-Authenticate, got {hdr:?}"
    );
}

#[tokio::test]
async fn deny_all_with_header_yields_403() {
    let (addr, _handle) = spawn_with_security(SecurityMode::DenyAll).await;
    let base = format!("http://{}", addr);
    let key_url = format!("{base}/v1/kv/a/k");

    let client =
        reqwest::Client::builder().timeout(Duration::from_secs(2)).build().expect("client");

    // With a dummy Macro token we still deny in MVP (no external auth yet)
    let put = client
        .put(&key_url)
        .header("content-type", "application/octet-stream")
        .header("authorization", "Macro dummy-token")
        .body("hello")
        .send()
        .await
        .expect("PUT with dummy auth");
    assert_eq!(put.status(), StatusCode::FORBIDDEN, "expected 403, got {}", put.status());
}

#[tokio::test]
async fn dev_allow_permits_kv_without_header() {
    let (addr, _handle) = spawn_with_security(SecurityMode::DevAllow).await;
    let base = format!("http://{}", addr);
    let key_url = format!("{base}/v1/kv/a/k");

    let client =
        reqwest::Client::builder().timeout(Duration::from_secs(2)).build().expect("client");

    // 1) PUT without Authorization should succeed in dev_allow.
    let put = client
        .put(&key_url)
        .header("content-type", "application/octet-stream")
        .body("hello")
        .send()
        .await
        .expect("PUT /v1/kv/a/k");
    assert_eq!(put.status(), StatusCode::CREATED, "expected 201, got {}", put.status());

    // 2) GET should return the value.
    let get = client.get(&key_url).send().await.expect("GET");
    assert_eq!(get.status(), StatusCode::OK, "expected 200, got {}", get.status());
    let body = get.bytes().await.expect("read body");
    assert_eq!(&body[..], b"hello");
}
