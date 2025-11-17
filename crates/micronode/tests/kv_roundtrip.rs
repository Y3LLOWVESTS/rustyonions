//! RO:WHAT — KV roundtrip test (PUT/GET/DELETE) under dev_allow.
//! RO:WHY  — Default security mode is deny_all; for this behavioral test we
//!           exercise the storage surface in DX mode.
//! RO:INVARIANTS — Ensures status codes and body echo match expectations.

use std::{net::SocketAddr, time::Duration};

use micronode::app::build_router;
use micronode::config::schema::{Config, SecurityCfg, SecurityMode, Server};
use reqwest::StatusCode;
use tokio::task::JoinHandle;

async fn spawn_dev_allow() -> (SocketAddr, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind test listener");
    let addr = listener.local_addr().expect("local address");

    let cfg = Config {
        server: Server { bind: addr, dev_routes: false },
        security: SecurityCfg { mode: SecurityMode::DevAllow },
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
            eprintln!("[micronode-kv-roundtrip] server error: {err}");
        }
    });

    (addr, handle)
}

#[tokio::test]
async fn kv_put_get_delete_roundtrip() {
    let (addr, _handle) = spawn_dev_allow().await;
    let base = format!("http://{}", addr);
    let key_url = format!("{base}/v1/kv/demo/k");

    let client =
        reqwest::Client::builder().timeout(Duration::from_secs(2)).build().expect("client");

    // PUT
    let put = client
        .put(&key_url)
        .header("content-type", "application/octet-stream")
        .body("hello")
        .send()
        .await
        .expect("PUT /v1/kv/demo/k");
    assert_eq!(put.status(), StatusCode::CREATED, "expected 201 Created, got {}", put.status());

    // GET
    let get = client.get(&key_url).send().await.expect("GET /v1/kv/demo/k");
    assert_eq!(get.status(), StatusCode::OK, "expected 200 OK, got {}", get.status());
    let body = get.bytes().await.expect("read body");
    assert_eq!(&body[..], b"hello");

    // DELETE
    let del = client.delete(&key_url).send().await.expect("DELETE /v1/kv/demo/k");
    assert_eq!(
        del.status(),
        StatusCode::NO_CONTENT,
        "expected 204 No Content, got {}",
        del.status()
    );
}
