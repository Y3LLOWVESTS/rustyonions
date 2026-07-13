//! RO:WHAT — Integration tests for Micronode admin plane.
//! RO:WHY  — Ensure `/healthz`, `/readyz`, `/version`, `/api/v1/status`, and `/metrics`
//!           are wired and behave sanely in-process.
//! RO:HOW  — Spin up an ephemeral axum server using `build_router` and hit it with `reqwest`.
//!
//! These tests intentionally match the current endpoint contract:
//!   - `/healthz`, `/readyz`, and `/version` are plain text.
//!   - `/api/v1/status` is JSON for svc-admin/node truth.
//!   - `/metrics` is Prometheus text.

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
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind test listener");
    let addr = listener.local_addr().expect("get local address for test listener");

    let cfg = Config { server: Server { bind: addr, dev_routes: true }, ..Config::default() };

    let (router, state) = build_router(cfg);

    // Match main.rs readiness sequencing after listener bind.
    state.probes.set_cfg_loaded(true);
    state.probes.set_listeners_bound(true);
    state.probes.set_metrics_bound(true);
    state.probes.set_deps_ok(true);

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

    // /healthz — current contract is plain text: "ok".
    let health = client.get(format!("{base}/healthz")).send().await.expect("GET /healthz");
    assert!(health.status().is_success(), "expected 2xx from /healthz, got {}", health.status());
    let health_text = health.text().await.expect("read /healthz text");
    assert_eq!(health_text, "ok");

    // /readyz — current contract is plain text: "ready".
    let ready = client.get(format!("{base}/readyz")).send().await.expect("GET /readyz");
    assert_eq!(ready.status(), StatusCode::OK, "expected 200 from /readyz, got {}", ready.status());
    let ready_text = ready.text().await.expect("read /readyz text");
    assert_eq!(ready_text, "ready");

    // /version — current contract is the crate version as plain text.
    let version = client.get(format!("{base}/version")).send().await.expect("GET /version");
    assert!(version.status().is_success(), "expected 2xx from /version, got {}", version.status());
    let version_text = version.text().await.expect("read /version text");
    assert_eq!(version_text, env!("CARGO_PKG_VERSION"));

    // /api/v1/status — svc-admin contract endpoint for node truth.
    let status =
        client.get(format!("{base}/api/v1/status")).send().await.expect("GET /api/v1/status");
    assert!(
        status.status().is_success(),
        "expected 2xx from /api/v1/status, got {}",
        status.status()
    );
    let status_text = status.text().await.expect("read /api/v1/status text");
    let status_body: serde_json::Value =
        serde_json::from_str(&status_text).expect("parse /api/v1/status json");

    assert_eq!(status_body["profile"], "micronode");
    assert_eq!(status_body["node_role"], "user_node");
    assert_eq!(status_body["node_profile"], "micronode");
    assert_eq!(status_body["amnesia_mode"], true);
    assert_eq!(status_body["privacy_mode"], true);
    assert_eq!(status_body["public_inbound_enabled"], false);
    assert_eq!(status_body["verification_enabled"], true);
    assert_eq!(status_body["content_serving_enabled"], false);
    assert_eq!(status_body["economic_replay_enabled"], true);
    assert_eq!(status_body["service_quorum_enabled"], false);
    assert_eq!(status_body["wallet_execution_participant"], false);
    assert_eq!(status_body["ledger_replay_enabled"], false);
    assert_eq!(status_body["user_ip_publication"], "forbidden");
    assert_eq!(status_body["peer_ip_display"], "forbidden");
    assert_eq!(status_body["admin_bind_loopback_only"], true);

    let passive = &status_body["passive_runtime"];
    assert_eq!(passive["enabled"], true);
    assert_eq!(passive["lifecycle_state"], "active");
    assert_eq!(passive["resource_mode"], "balanced");
    assert_eq!(passive["max_cpu_percent"], 5);
    assert_eq!(passive["max_background_kbps"], 64);
    assert_eq!(passive["pending_evidence_limit"], 1024);
    assert_eq!(passive["privacy_mode"], true);
    assert_eq!(passive["public_inbound_enabled"], false);
    assert_eq!(passive["peer_ip_display"], "forbidden");
    assert_eq!(passive["verification_queue"]["enabled"], true);
    assert_eq!(passive["verification_queue"]["status"], "stubbed");
    assert_eq!(passive["verification_queue"]["pending_items"], 0);
    assert_eq!(passive["verification_queue"]["mutates_wallet"], false);
    assert_eq!(passive["verification_queue"]["mutates_ledger"], false);
    assert_eq!(passive["economic_replay_worker"]["enabled"], true);
    assert_eq!(passive["economic_replay_worker"]["status"], "stubbed");
    assert_eq!(passive["economic_replay_worker"]["pending_items"], 0);
    assert_eq!(passive["economic_replay_worker"]["mutates_wallet"], false);
    assert_eq!(passive["economic_replay_worker"]["mutates_ledger"], false);
    assert_eq!(passive["confirmed_roc_minor_units"], serde_json::Value::Null);
    assert_eq!(passive["confirmed_roc_source"], "wallet_ledger_receipt_only");
    assert_eq!(passive["wallet_mutation"], false);
    assert_eq!(passive["ledger_mutation"], false);

    let capabilities = status_body["capabilities"].as_array().expect("capabilities array");
    assert!(
        capabilities.iter().any(|v| v == "user_node_status_v1"),
        "expected user_node_status_v1 capability, got {status_body}"
    );
    assert!(
        capabilities.iter().any(|v| v == "passive_user_node_runtime_v1"),
        "expected passive_user_node_runtime_v1 capability, got {status_body}"
    );
    assert!(
        capabilities.iter().any(|v| v == "verification_queue_stub_v1"),
        "expected verification_queue_stub_v1 capability, got {status_body}"
    );
    assert!(
        capabilities.iter().any(|v| v == "economic_replay_stub_v1"),
        "expected economic_replay_stub_v1 capability, got {status_body}"
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
    assert!(
        metrics_text.contains("amnesia_mode 1"),
        "expected /metrics to report amnesia_mode 1 for default mem storage; got:\n{}",
        metrics_text
    );
}
