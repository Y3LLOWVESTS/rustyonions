//! RO:WHAT — Integration tests for Phase 4A local operator admin controls.
//! RO:WHY — Proves crabnode-targeted setup-token and admin-UI toggle endpoints are real.
//! RO:INTERACTS — macronode binary, /api/v1/admin/*, /api/v1/status.
//! RO:INVARIANTS — setup tokens are one-use; admin UI toggle is runtime-local; no fake user creation.
//! RO:TEST — cargo test -p macronode --test operator_admin.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::sleep;

const ADMIN_PORT: u16 = 18180;
const GATEWAY_PORT: u16 = 18190;

async fn spawn_macronode() -> Result<(Child, Client, String)> {
    let bin = env!("CARGO_BIN_EXE_macronode");

    let mut cmd = Command::new(bin);
    cmd.arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{ADMIN_PORT}"))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{GATEWAY_PORT}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn().context("failed to spawn macronode binary")?;

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("failed to build reqwest client")?;

    let base = format!("http://127.0.0.1:{ADMIN_PORT}");

    for _ in 0..50 {
        match client.get(format!("{base}/version")).send().await {
            Ok(resp) if resp.status().is_success() => return Ok((child, client, base)),
            _ => sleep(Duration::from_millis(200)).await,
        }
    }

    Err(anyhow!("macronode did not expose /version in time"))
}

async fn shutdown_macronode(mut child: Child, client: &Client, base: &str) -> Result<()> {
    let _ = client.post(format!("{base}/api/v1/shutdown")).send().await;

    for _ in 0..50 {
        if let Ok(Some(_status)) = child.try_wait() {
            return Ok(());
        }
        sleep(Duration::from_millis(200)).await;
    }

    let _ = child.kill();
    Err(anyhow!("macronode did not exit cleanly after /shutdown"))
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_ui_toggle_and_setup_token_are_real_runtime_controls() -> Result<()> {
    let (child, client, base) = spawn_macronode().await?;

    let body: Value = client
        .get(format!("{base}/api/v1/status"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(body["admin_ui_enabled"], false);
    assert_eq!(body["setup_token_active"], false);

    let body: Value = client
        .post(format!("{base}/api/v1/admin/ui/enable"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(body["status"], "admin UI enabled");
    assert_eq!(body["admin_ui_enabled"], true);
    assert_eq!(body["admin_ui_runtime_required"], false);

    let body: Value = client
        .get(format!("{base}/api/v1/status"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(body["admin_ui_enabled"], true);

    let body: Value = client
        .post(format!("{base}/api/v1/admin/ui/disable"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(body["status"], "admin UI disabled");
    assert_eq!(body["admin_ui_enabled"], false);

    let body: Value = client
        .post(format!("{base}/api/v1/admin/setup-token"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(body["status"], "setup token issued");
    assert!(body["token"].as_str().unwrap_or_default().len() >= 32);
    assert!(body["setup_url"]
        .as_str()
        .unwrap_or_default()
        .contains("127.0.0.1:5300/setup?token="));
    assert_eq!(body["expires_in_seconds"], 900);

    let token = body["token"]
        .as_str()
        .expect("setup token string")
        .to_string();

    let body: Value = client
        .get(format!("{base}/api/v1/status"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(body["setup_token_active"], true);

    let resp = client
        .post(format!("{base}/api/v1/admin/setup-token/consume"))
        .json(&json!({ "token": token }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "first setup token consume should succeed"
    );
    let body: Value = resp.json().await?;
    assert_eq!(body["consumed"], true);

    let resp = client
        .post(format!("{base}/api/v1/admin/setup-token/consume"))
        .json(&json!({ "token": token }))
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);

    shutdown_macronode(child, &client, &base).await?;
    Ok(())
}
