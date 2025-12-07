// crates/svc-admin/tests/auth_me_http.rs

use std::time::Duration;

use reqwest::Client;
use svc_admin::{config::Config, server};
use tokio::task::JoinHandle;

/// Spin up svc-admin with the given config, wait for /healthz on the
/// metrics port to pass, and return a join handle plus the base UI URL.
async fn spawn_svc_admin(config: Config) -> (JoinHandle<()>, String) {
    let ui_addr = config.server.bind_addr.clone();
    let metrics_addr = config.server.metrics_addr.clone();

    let handle = tokio::spawn(async move {
        // If this panics, the test should fail loudly.
        server::run(config).await.expect("svc-admin server exited with error");
    });

    wait_for_healthz(&metrics_addr).await;

    (handle, format!("http://{}", ui_addr))
}

async fn wait_for_healthz(metrics_addr: &str) {
    let client = Client::new();
    let url = format!("http://{}/healthz", metrics_addr);

    for _ in 0..50 {
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => return,
            _ => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    panic!("svc-admin /healthz did not become ready on {}", metrics_addr);
}

#[tokio::test]
async fn me_returns_dev_identity_in_none_mode() {
    // Start from the crate's default config so we inherit all the right
    // defaults (nodes, polling, etc.) and only tweak what we need.
    let mut config = Config::default();
    config.server.bind_addr = "127.0.0.1:5400".to_string();
    config.server.metrics_addr = "127.0.0.1:5410".to_string();
    config.auth.mode = "none".to_string();

    let (handle, base_url) = spawn_svc_admin(config).await;
    let client = Client::new();

    let resp = client
        .get(format!("{}/api/me", base_url))
        .send()
        .await
        .expect("request to /api/me failed");

    assert!(
        resp.status().is_success(),
        "expected 2xx from /api/me in none mode, got {}",
        resp.status()
    );

    let body: serde_json::Value = resp
        .json()
        .await
        .expect("failed to deserialize /api/me body as JSON");

    // Shape is defined by dto::me::MeResponse (camelCase via serde).
    assert_eq!(body["subject"], "dev-operator");
    assert_eq!(body["displayName"], "Dev Operator");
    assert_eq!(body["authMode"], "none");

    let roles = body["roles"]
        .as_array()
        .expect("roles should be an array");
    assert!(
        roles.iter().any(|r| r == "admin"),
        "roles should contain 'admin', got {:?}",
        roles
    );

    // In none mode we expect no login URL (serialized as JSON null).
    assert!(
        body["loginUrl"].is_null(),
        "loginUrl should be null in none mode, got {:?}",
        body["loginUrl"]
    );

    // We don't care about the metrics sampler loop for this test, so just
    // tear down the server task.
    handle.abort();
}

#[tokio::test]
async fn me_uses_ingress_headers_when_mode_is_ingress() {
    let mut config = Config::default();
    // Use different ports from the previous test to avoid any flakiness
    // around port reuse if the OS keeps them in TIME_WAIT briefly.
    config.server.bind_addr = "127.0.0.1:5401".to_string();
    config.server.metrics_addr = "127.0.0.1:5411".to_string();
    config.auth.mode = "ingress".to_string();

    let (handle, base_url) = spawn_svc_admin(config).await;
    let client = Client::new();

    // These header names follow the ingress backend defaults (X-User / X-Groups).
    let resp = client
        .get(format!("{}/api/me", base_url))
        .header("X-User", "alice@example.com")
        .header("X-Groups", "admin,ops")
        .send()
        .await
        .expect("request to /api/me failed");

    assert!(
        resp.status().is_success(),
        "expected 2xx from /api/me in ingress mode, got {}",
        resp.status()
    );

    let body: serde_json::Value = resp
        .json()
        .await
        .expect("failed to deserialize /api/me body as JSON");

    assert_eq!(body["subject"], "alice@example.com");
    assert_eq!(body["displayName"], "alice@example.com");
    assert_eq!(body["authMode"], "ingress");

    let roles = body["roles"]
        .as_array()
        .expect("roles should be an array");
    assert_eq!(roles.len(), 2, "expected exactly 2 roles, got {:?}", roles);
    assert!(
        roles.iter().any(|r| r == "admin"),
        "roles should contain 'admin', got {:?}",
        roles
    );
    assert!(
        roles.iter().any(|r| r == "ops"),
        "roles should contain 'ops', got {:?}",
        roles
    );

    // In ingress mode we still don't expect a loginUrl from svc-admin itself;
    // the upstream ingress is responsible for auth flows.
    assert!(
        body["loginUrl"].is_null(),
        "loginUrl should be null in ingress mode, got {:?}",
        body["loginUrl"]
    );

    handle.abort();
}
