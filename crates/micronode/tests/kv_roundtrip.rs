//! RO:WHAT — Integration tests for Micronode KV v1 HTTP API.
//! RO:WHY  — Assert that PUT/GET/DELETE /v1/kv/{bucket}/{key} behave as
//!           documented, using the same router wiring as the binary.
//! RO:INVARIANTS —
//!   - In-memory storage engine behaves like a simple KV store.
//!   - Readiness probes are flipped to "healthy" for the duration of the test.

use std::{net::SocketAddr, time::Duration};

use micronode::app::build_router;
use micronode::config::schema::{Config, Server};
use reqwest::StatusCode;
use tokio::task::JoinHandle;

/// Spawn an in-process Micronode instance on an ephemeral port.
///
/// Mirrors the binary bootstrap pattern:
///   - Bind 127.0.0.1:0
///   - Build router from Config
///   - Flip readiness probes so /readyz is truthfully "ready"
async fn spawn_micronode() -> (SocketAddr, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind test listener");
    let addr = listener.local_addr().expect("get local address for test listener");

    // Minimal config: bind address + dev routes enabled (harmless for KV tests).
    let cfg = Config { server: Server { bind: addr, dev_routes: true } };

    let (router, state) = build_router(cfg);

    // Make readiness truthful for the in-memory engine.
    state.probes.set_cfg_loaded(true);
    state.probes.set_listeners_bound(true);
    state.probes.set_metrics_bound(true);
    state.probes.set_deps_ok(true);

    let handle = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, router).await {
            eprintln!("[micronode-kv-test] server error: {err}");
        }
    });

    (addr, handle)
}

#[tokio::test]
async fn kv_put_get_delete_roundtrip() {
    let (addr, _handle) = spawn_micronode().await;
    let base = format!("http://{}", addr);
    let key_url = format!("{base}/v1/kv/a/k");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("build reqwest client");

    // 1) PUT — create key with "hello" payload.
    let put = client
        .put(&key_url)
        .header("content-type", "application/octet-stream")
        .body("hello")
        .send()
        .await
        .expect("PUT /v1/kv/a/k");
    assert_eq!(put.status(), StatusCode::CREATED, "expected 201 from PUT, got {}", put.status());

    // 2) GET — verify payload echoes back verbatim.
    let get = client.get(&key_url).send().await.expect("GET /v1/kv/a/k (after PUT)");
    assert_eq!(
        get.status(),
        StatusCode::OK,
        "expected 200 from GET after PUT, got {}",
        get.status()
    );
    let body = get.bytes().await.expect("read GET body");
    assert_eq!(&body[..], b"hello", "expected GET body == b\"hello\"");

    // 3) DELETE — remove key.
    let del = client.delete(&key_url).send().await.expect("DELETE /v1/kv/a/k");
    assert_eq!(
        del.status(),
        StatusCode::NO_CONTENT,
        "expected 204 from DELETE, got {}",
        del.status()
    );

    // 4) GET again — now we expect 404.
    let get_missing = client.get(&key_url).send().await.expect("GET /v1/kv/a/k (after DELETE)");
    assert_eq!(
        get_missing.status(),
        StatusCode::NOT_FOUND,
        "expected 404 from GET after DELETE, got {}",
        get_missing.status()
    );
}
