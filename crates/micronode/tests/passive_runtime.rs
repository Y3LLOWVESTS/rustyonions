//! RO:WHAT — Phase 6A tests for Micronode passive user-node runtime posture.
//! RO:WHY — Locks loopback-only privacy, bounded resource config, and stubbed
//!          verification/economic replay status without fake wallet/ledger truth.
//! RO:TEST — `cargo test -p micronode --test passive_runtime`.

use std::{net::SocketAddr, time::Duration};

use micronode::{
    app::build_router,
    config::{
        schema::{Config, ResourceMode, Server, UserNodeCfg},
        validate::validate,
    },
};
use reqwest::StatusCode;
use tokio::task::JoinHandle;

async fn spawn_with_user_node_cfg(user_node: UserNodeCfg) -> (SocketAddr, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind test listener");
    let addr = listener.local_addr().expect("get local address");

    let cfg =
        Config { server: Server { bind: addr, dev_routes: false }, user_node, ..Config::default() };

    let (router, state) = build_router(cfg);
    state.probes.set_cfg_loaded(true);
    state.probes.set_listeners_bound(true);
    state.probes.set_metrics_bound(true);
    state.probes.set_deps_ok(true);

    let handle = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, router).await {
            eprintln!("[micronode-passive-runtime-test] server error: {err}");
        }
    });

    (addr, handle)
}

#[test]
fn user_node_rejects_public_inbound_config() {
    let cfg = Config {
        server: Server { bind: "0.0.0.0:5310".parse().unwrap(), dev_routes: false },
        ..Config::default()
    };

    let err = validate(&cfg).expect_err("public bind must be rejected for passive user node");
    assert!(
        err.to_string().contains("loopback-only"),
        "expected loopback-only validation error, got {err}"
    );
}

#[test]
fn user_node_resource_budgets_are_bounded() {
    let zero_cpu = Config {
        user_node: UserNodeCfg { max_cpu_percent: 0, ..UserNodeCfg::default() },
        ..Config::default()
    };
    assert!(validate(&zero_cpu).is_err(), "zero cpu budget must be rejected");

    let too_much_cpu = Config {
        user_node: UserNodeCfg { max_cpu_percent: 101, ..UserNodeCfg::default() },
        ..Config::default()
    };
    assert!(validate(&too_much_cpu).is_err(), "cpu budget above 100 must be rejected");

    let zero_bandwidth = Config {
        user_node: UserNodeCfg { max_background_kbps: 0, ..UserNodeCfg::default() },
        ..Config::default()
    };
    assert!(validate(&zero_bandwidth).is_err(), "zero background bandwidth must be rejected");

    let zero_queue = Config {
        user_node: UserNodeCfg { pending_evidence_limit: 0, ..UserNodeCfg::default() },
        ..Config::default()
    };
    assert!(validate(&zero_queue).is_err(), "zero pending evidence queue must be rejected");
}

#[tokio::test]
async fn user_node_passive_runtime_reports_private_loopback_posture() {
    let user_node = UserNodeCfg {
        resource_mode: ResourceMode::Low,
        max_cpu_percent: 3,
        max_background_kbps: 32,
        pending_evidence_limit: 256,
        ..UserNodeCfg::default()
    };
    let (addr, _handle) = spawn_with_user_node_cfg(user_node).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("build reqwest client");

    let status = client
        .get(format!("http://{addr}/api/v1/status"))
        .send()
        .await
        .expect("GET /api/v1/status");
    assert_eq!(status.status(), StatusCode::OK);

    let status_text = status.text().await.expect("read status body");
    let body: serde_json::Value = serde_json::from_str(&status_text).expect("parse status json");
    assert_eq!(body["node_role"], "user_node");
    assert_eq!(body["privacy_mode"], true);
    assert_eq!(body["public_inbound_enabled"], false);
    assert_eq!(body["admin_bind_loopback_only"], true);
    assert_eq!(body["user_ip_publication"], "forbidden");
    assert_eq!(body["peer_ip_display"], "forbidden");

    let passive = &body["passive_runtime"];
    assert_eq!(passive["enabled"], true);
    assert_eq!(passive["resource_mode"], "low");
    assert_eq!(passive["max_cpu_percent"], 3);
    assert_eq!(passive["max_background_kbps"], 32);
    assert_eq!(passive["pending_evidence_limit"], 256);
    assert_eq!(passive["public_inbound_enabled"], false);
    assert_eq!(passive["peer_ip_display"], "forbidden");
}

#[tokio::test]
async fn user_node_status_exposes_stubs_without_claiming_rewards() {
    let (addr, _handle) = spawn_with_user_node_cfg(UserNodeCfg::default()).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("build reqwest client");

    let status = client
        .get(format!("http://{addr}/api/v1/status"))
        .send()
        .await
        .expect("GET /api/v1/status");
    assert_eq!(status.status(), StatusCode::OK);

    let status_text = status.text().await.expect("read status body");
    let body: serde_json::Value = serde_json::from_str(&status_text).expect("parse status json");
    assert_eq!(body["verification_enabled"], true);
    assert_eq!(body["economic_replay_enabled"], true);
    assert_eq!(body["wallet_execution_participant"], false);
    assert_eq!(body["ledger_replay_enabled"], false);

    let passive = &body["passive_runtime"];
    assert_eq!(passive["verification_queue"]["enabled"], true);
    assert_eq!(passive["verification_queue"]["status"], "stubbed");
    assert_eq!(passive["verification_queue"]["mutates_wallet"], false);
    assert_eq!(passive["verification_queue"]["mutates_ledger"], false);
    assert_eq!(passive["economic_replay_worker"]["enabled"], true);
    assert_eq!(passive["economic_replay_worker"]["status"], "stubbed");
    assert_eq!(passive["economic_replay_worker"]["mutates_wallet"], false);
    assert_eq!(passive["economic_replay_worker"]["mutates_ledger"], false);
    assert_eq!(passive["confirmed_roc_minor_units"], serde_json::Value::Null);
    assert_eq!(passive["confirmed_roc_source"], "wallet_ledger_receipt_only");
    assert_eq!(passive["wallet_mutation"], false);
    assert_eq!(passive["ledger_mutation"], false);
}
