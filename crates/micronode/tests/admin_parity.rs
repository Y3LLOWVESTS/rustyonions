//! RO:WHAT — Integration tests for Micronode admin plane.
//! RO:WHY  — Ensure `/healthz`, `/readyz`, `/version`, `/metrics` are wired
//!           and behave sanely in-process (no external binaries needed).
//! RO:HOW  — Spin up an ephemeral axum server using `build_router` and
//!           hit it with `reqwest`.
//!
//! These are intentionally high-level smoke tests:
//!   - If they fail, the node is not “basically alive”.
//!   - They double as a template for future KV / guard tests.

use std::{net::SocketAddr, time::Duration};

use micronode::app::build_router;
use micronode::config::schema::{Config, Server};
use reqwest::StatusCode;
use tokio::task::JoinHandle;

/// Spawn an in-process Micronode instance on an ephemeral port.
///
/// This mirrors the main binary’s bootstrap pattern but avoids config
/// files and uses a synthetic `Config` pointing at `127.0.0.1:0`.
async fn spawn_micronode() -> (SocketAddr, JoinHandle<()>) {
    // Bind an ephemeral port first so we know where to hit the server.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind test listener");
    let addr = listener.local_addr().expect("get local address for test listener");

    // Minimal config: bind address + dev routes (handy for future tests).
    let cfg = Config { server: Server { bind: addr, dev_routes: true } };

    // Build router + state the same way the binary does.
    let (router, state) = build_router(cfg);

    // Flip readiness probes to a “healthy” state.
    //
    // This matches what the main binary does after binding listeners
    // and wiring metrics. For now we treat deps_ok as true because the
    // in-memory store has no external failure mode.
    state.probes.set_cfg_loaded(true);
    state.probes.set_listeners_bound(true);
    state.probes.set_metrics_bound(true);
    state.probes.set_deps_ok(true);

    // Run the server in the background. Dropping the handle will cancel it.
    let handle = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, router).await {
            eprintln!("[micronode-test] server error: {err}");
        }
    });

    (addr, handle)
}

#[tokio::test]
async fn admin_endpoints_are_healthy_and_observable() {
    let (addr, _handle) = spawn_micronode().await;
    let base = format!("http://{}", addr);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("build reqwest client");

    // /healthz — must be 200 and JSON { "ok": true }.
    let health = client.get(format!("{base}/healthz")).send().await.expect("GET /healthz");
    assert!(health.status().is_success(), "expected 2xx from /healthz, got {}", health.status());
    let health_body: serde_json::Value = health.json().await.expect("parse /healthz json");
    assert_eq!(
        health_body["ok"],
        serde_json::Value::Bool(true),
        "expected /healthz.ok == true, got {health_body}"
    );

    // /readyz — must be 200 and JSON { "ready": true, ... } in truthful mode.
    let ready = client.get(format!("{base}/readyz")).send().await.expect("GET /readyz");
    assert_eq!(ready.status(), StatusCode::OK, "expected 200 from /readyz, got {}", ready.status());
    let ready_body: serde_json::Value = ready.json().await.expect("parse /readyz json");
    assert_eq!(
        ready_body["ready"],
        serde_json::Value::Bool(true),
        "expected /readyz.ready == true, got {ready_body}"
    );

    // /version — must be 200 and at least contain `name: "micronode"`.
    let version = client.get(format!("{base}/version")).send().await.expect("GET /version");
    assert!(version.status().is_success(), "expected 2xx from /version, got {}", version.status());
    let version_body: serde_json::Value = version.json().await.expect("parse /version json");
    assert_eq!(
        version_body["name"],
        serde_json::Value::String("micronode".to_string()),
        "expected /version.name == \"micronode\", got {version_body}"
    );

    // /metrics — must be 200 and contain at least the micronode HTTP series.
    let metrics = client.get(format!("{base}/metrics")).send().await.expect("GET /metrics");
    assert!(metrics.status().is_success(), "expected 2xx from /metrics, got {}", metrics.status());
    let metrics_text = metrics.text().await.expect("read /metrics text");

    assert!(
        metrics_text.contains("micronode_http_requests_total"),
        "expected /metrics to contain micronode_http_requests_total; got:\n{}",
        metrics_text
    );
    assert!(
        metrics_text.contains("micronode_request_latency_seconds"),
        "expected /metrics to contain micronode_request_latency_seconds; got:\n{}",
        metrics_text
    );
}
