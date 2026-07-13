// crates/svc-admin/tests/setup_http.rs

use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use reqwest::{Client, StatusCode};
use serde_json::json;
use svc_admin::{config::Config, server};
use tokio::task::JoinHandle;

async fn spawn_svc_admin(config: Config) -> (JoinHandle<()>, String) {
    let ui_addr = config.server.bind_addr.clone();
    let metrics_addr = config.server.metrics_addr.clone();

    let handle = tokio::spawn(async move {
        server::run(config)
            .await
            .expect("svc-admin server exited with error");
    });

    wait_for_healthz(&metrics_addr).await;

    (handle, format!("http://{ui_addr}"))
}

async fn wait_for_healthz(metrics_addr: &str) {
    let client = Client::new();
    let url = format!("http://{metrics_addr}/healthz");

    for _ in 0..50 {
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => return,
            _ => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }

    panic!("svc-admin /healthz did not become ready on {metrics_addr}");
}

fn spawn_fake_macronode_setup_verifier(port: u16) -> String {
    let listener =
        TcpListener::bind(("127.0.0.1", port)).expect("fake macronode setup verifier should bind");

    thread::spawn(move || {
        for _ in 0..8 {
            let Ok((mut stream, _addr)) = listener.accept() else {
                continue;
            };

            let mut buf = [0_u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]);

            let is_consume = request.starts_with("POST /api/v1/admin/setup-token/consume ");
            let accepted = is_consume && request.contains("setup-token-good");

            let (status, body) = if accepted {
                (
                    "HTTP/1.1 200 OK",
                    r#"{"status":"setup token consumed","consumed":true}"#,
                )
            } else if is_consume {
                (
                    "HTTP/1.1 401 Unauthorized",
                    r#"{"status":"setup token rejected","consumed":false}"#,
                )
            } else {
                ("HTTP/1.1 404 Not Found", r#"{"status":"not_found"}"#)
            };

            let response = format!(
                "{status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );

            let _ = stream.write_all(response.as_bytes());
        }
    });

    format!("http://127.0.0.1:{port}")
}

fn temp_rbac_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "svc-admin-{label}-{}-{nanos}.rbac.json",
        std::process::id()
    ))
}

fn local_config(
    label: &str,
    ui_port: u16,
    metrics_port: u16,
    macronode_url: &str,
) -> (Config, PathBuf) {
    let rbac_path = temp_rbac_path(label);

    let mut config = Config::default();
    config.server.bind_addr = format!("127.0.0.1:{ui_port}");
    config.server.metrics_addr = format!("127.0.0.1:{metrics_port}");
    config.auth.mode = "local".to_string();
    config.auth.rbac_path = Some(rbac_path.to_string_lossy().to_string());
    config.auth.bootstrap_admin_password_env =
        Some(format!("SVC_ADMIN_TEST_UNUSED_BOOTSTRAP_PASSWORD_{label}"));

    let macronode = config
        .nodes
        .get_mut("macronode")
        .expect("default config should include local macronode");
    macronode.base_url = macronode_url.to_string();
    macronode.insecure_http = true;

    (config, rbac_path)
}

#[tokio::test]
async fn setup_http_creates_first_admin_and_then_locks_setup() {
    let macronode_url = spawn_fake_macronode_setup_verifier(18444);
    let (config, rbac_path) = local_config("setup-http", 5420, 5430, &macronode_url);
    let (handle, base_url) = spawn_svc_admin(config).await;
    let client = Client::new();

    let status_before_resp = client
        .get(format!("{base_url}/api/setup/status"))
        .send()
        .await
        .expect("setup status request should complete");

    assert_eq!(
        status_before_resp.status(),
        StatusCode::OK,
        "setup status must be reachable before login"
    );

    let status_before: serde_json::Value = status_before_resp
        .json()
        .await
        .expect("setup status should be JSON");

    assert_eq!(status_before["localAuthEnabled"], true);
    assert_eq!(status_before["hasLocalUsers"], false);
    assert_eq!(status_before["setupRequired"], true);

    let anonymous_nodes = client
        .get(format!("{base_url}/api/nodes"))
        .send()
        .await
        .expect("anonymous /api/nodes request should complete");

    assert_eq!(
        anonymous_nodes.status(),
        StatusCode::UNAUTHORIZED,
        "setup allowlist must not expose the node registry"
    );

    let missing_token_resp = client
        .post(format!("{base_url}/api/setup/create-admin"))
        .json(&json!({
            "username": "admin",
            "password": "correct horse battery staple"
        }))
        .send()
        .await
        .expect("missing-token setup request should complete");

    assert_eq!(missing_token_resp.status(), StatusCode::UNAUTHORIZED);

    let bad_token_resp = client
        .post(format!("{base_url}/api/setup/create-admin"))
        .json(&json!({
            "username": "admin",
            "password": "correct horse battery staple",
            "setupToken": "setup-token-bad"
        }))
        .send()
        .await
        .expect("bad-token setup request should complete");

    assert_eq!(bad_token_resp.status(), StatusCode::UNAUTHORIZED);

    let create_resp = client
        .post(format!("{base_url}/api/setup/create-admin"))
        .json(&json!({
            "username": "admin",
            "password": "correct horse battery staple",
            "setupToken": "setup-token-good"
        }))
        .send()
        .await
        .expect("setup create-admin request should complete");

    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let create_body: serde_json::Value = create_resp
        .json()
        .await
        .expect("create-admin response should be JSON");

    assert_eq!(create_body["status"], "admin_created");
    assert_eq!(create_body["setupRequired"], false);
    assert_eq!(create_body["user"]["username"], "admin");

    let status_after: serde_json::Value = client
        .get(format!("{base_url}/api/setup/status"))
        .send()
        .await
        .expect("setup status after create should complete")
        .json()
        .await
        .expect("setup status after create should be JSON");

    assert_eq!(status_after["hasLocalUsers"], true);
    assert_eq!(status_after["setupRequired"], false);

    let duplicate_resp = client
        .post(format!("{base_url}/api/setup/create-admin"))
        .json(&json!({
            "username": "admin2",
            "password": "another correct horse battery staple"
        }))
        .send()
        .await
        .expect("duplicate setup request should complete");

    assert_eq!(duplicate_resp.status(), StatusCode::CONFLICT);

    let login_resp = client
        .post(format!("{base_url}/api/auth/login"))
        .json(&json!({
            "username": "admin",
            "password": "correct horse battery staple"
        }))
        .send()
        .await
        .expect("login after setup should complete");

    assert!(
        login_resp.status().is_success(),
        "login after setup should succeed, got {}",
        login_resp.status()
    );

    handle.abort();
    let _ = std::fs::remove_file(rbac_path);
}
