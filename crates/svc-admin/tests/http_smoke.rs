// crates/svc-admin/tests/http_smoke.rs

use std::collections::BTreeMap;
use std::time::Duration;

use reqwest::Client;
use svc_admin::config::{
    ActionsCfg, AuthCfg, Config, LogCfg, NodeCfg, NodesCfg, PollingCfg,
    ServerCfg, TlsCfg, UiCfg, UiDevCfg,
};
use svc_admin::server;

#[tokio::test]
async fn healthz_and_metrics_smoke() {
    // Build a minimal but explicit config so we don't depend on env.
    let server = ServerCfg {
        bind_addr: "127.0.0.1:5300".to_string(),
        metrics_addr: "127.0.0.1:5310".to_string(),
        max_conns: 128,
        read_timeout: Duration::from_secs(5),
        write_timeout: Duration::from_secs(5),
        idle_timeout: Duration::from_secs(60),
        tls: TlsCfg::default(),
    };

    let auth = AuthCfg::default();

    let ui = UiCfg {
        default_theme: "light".to_string(),
        default_language: "en-US".to_string(),
        read_only: true,
        dev: UiDevCfg::default(),
    };

    let mut nodes: NodesCfg = BTreeMap::new();
    nodes.insert(
        "example-node".to_string(),
        NodeCfg {
            base_url: "http://127.0.0.1:9000".to_string(),
            display_name: Some("Example Node".to_string()),
            environment: "dev".to_string(),
            insecure_http: true,
            forced_profile: Some("macronode".to_string()),
            macaroon_path: None,
            default_timeout: Some(Duration::from_secs(2)),
        },
    );

    let cfg = Config {
        server,
        auth,
        ui,
        nodes,
        polling: PollingCfg::default(),
        log: LogCfg::default(),
        actions: ActionsCfg::default(),
    };

    // Spawn svc-admin in the background.
    let _handle = tokio::spawn(server::run(cfg));

    // Give it a moment to bind listeners. If this flakes later we can
    // replace it with a proper readiness probe.
    tokio::time::sleep(Duration::from_millis(250)).await;

    let client = Client::new();

    // /healthz on metrics port
    let health = client
        .get("http://127.0.0.1:5310/healthz")
        .send()
        .await
        .expect("healthz request should succeed");
    assert!(health.status().is_success());
    let body = health.text().await.expect("healthz body must be text");
    assert_eq!(body, "ok");

    // /metrics on metrics port
    let metrics = client
        .get("http://127.0.0.1:5310/metrics")
        .send()
        .await
        .expect("metrics request should succeed");
    assert!(metrics.status().is_success());
    let metrics_body = metrics
        .text()
        .await
        .expect("metrics body must be text");

    // Sanity checks: we should see our node inventory gauges.
    assert!(
        metrics_body.contains("ron_svc_admin_nodes_total"),
        "metrics output should contain ron_svc_admin_nodes_total\n{}",
        metrics_body
    );
    assert!(
        metrics_body.contains("ron_svc_admin_nodes_by_env"),
        "metrics output should contain ron_svc_admin_nodes_by_env\n{}",
        metrics_body
    );
}
