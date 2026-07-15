//! RO:WHAT — Live operator-admin and Phase 23 setup-credential chaos tests.
//! RO:WHY — Proves optional UI controls and expired setup credentials fail
//! closed without breaking the headless Service Node.
//! RO:INTERACTS — macronode binary, /api/v1/admin/*, /api/v1/status,
//! /healthz, Config::admin_setup_token_ttl.
//! RO:INVARIANTS — setup tokens are short-lived and one-use; admin UI remains
//! optional; expiration creates no admin, wallet, ledger, payout, or finality.
//! RO:SECURITY — loopback-only test listeners; no durable credential storage.
//! RO:TEST — cargo test -p macronode --test operator_admin.

use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::sleep;

struct MacronodeGuard {
    child: Child,
}

impl Drop for MacronodeGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_loopback_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("bind free loopback port")?;

    Ok(listener
        .local_addr()
        .context("read free loopback address")?
        .port())
}

async fn spawn_macronode(setup_token_ttl: &str) -> Result<(MacronodeGuard, Client, String)> {
    let bin = env!("CARGO_BIN_EXE_macronode");
    let admin_port = free_loopback_port()?;
    let gateway_port = free_loopback_port()?;

    let mut command = Command::new(bin);

    command
        .arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{admin_port}"))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{gateway_port}"))
        .env("RON_ADMIN_SETUP_TOKEN_TTL", setup_token_ttl)
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = command
        .spawn()
        .context("failed to spawn macronode binary")?;

    let guard = MacronodeGuard { child };

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("failed to build reqwest client")?;

    let base = format!("http://127.0.0.1:{admin_port}");

    for _ in 0..50 {
        match client.get(format!("{base}/version")).send().await {
            Ok(response) if response.status().is_success() => {
                return Ok((guard, client, base));
            }
            _ => sleep(Duration::from_millis(200)).await,
        }
    }

    Err(anyhow!("macronode did not expose /version in time"))
}

async fn shutdown_macronode(mut guard: MacronodeGuard, client: &Client, base: &str) -> Result<()> {
    let _ = client.post(format!("{base}/api/v1/shutdown")).send().await;

    for _ in 0..50 {
        if let Ok(Some(_status)) = guard.child.try_wait() {
            return Ok(());
        }

        sleep(Duration::from_millis(200)).await;
    }

    let _ = guard.child.kill();

    Err(anyhow!(
        "macronode did not exit cleanly after /api/v1/shutdown"
    ))
}

fn assert_headless_non_authoritative_status(body: &Value) {
    assert_eq!(body["headless_mode"], true);
    assert_eq!(body["admin_ui_enabled"], false);
    assert_eq!(body["admin_ui_runtime_required"], false);

    assert_eq!(body["reward_binding"]["registry_finality"], false);
    assert_eq!(body["reward_binding"]["wallet_mutation"], false);
    assert_eq!(body["reward_binding"]["ledger_mutation"], false);
    assert!(
        body["reward_binding"]["confirmed_roc_minor_units"].is_null(),
        "operator setup state must never fabricate confirmed ROC",
    );

    assert_eq!(body["service_quorum_enabled"], false);
    assert_eq!(body["wallet_execution_participant"], false);
    assert_eq!(body["ledger_replay_enabled"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_ui_toggle_and_setup_token_are_real_runtime_controls() -> Result<()> {
    let (node, client, base) = spawn_macronode("15m").await?;

    let body: Value = client
        .get(format!("{base}/api/v1/status"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_headless_non_authoritative_status(&body);
    assert_eq!(body["setup_token_active"], false);

    let body: Value = client
        .post(format!("{base}/api/v1/admin/ui/enable"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(body["status"], "admin UI enabled");
    assert_eq!(body["admin_ui_enabled"], true);
    assert_eq!(body["admin_ui_runtime_required"], false);

    let body: Value = client
        .get(format!("{base}/api/v1/status"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(body["admin_ui_enabled"], true);

    let body: Value = client
        .post(format!("{base}/api/v1/admin/ui/disable"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(body["status"], "admin UI disabled");
    assert_eq!(body["admin_ui_enabled"], false);

    let body: Value = client
        .post(format!("{base}/api/v1/admin/setup-token"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(body["status"], "setup token issued");
    assert!(body["token"].as_str().unwrap_or_default().len() >= 32);
    assert!(body["setup_url"]
        .as_str()
        .unwrap_or_default()
        .contains("127.0.0.1:5300/setup?token="));
    assert_eq!(body["expires_in_seconds"], 15 * 60);

    let token = body["token"]
        .as_str()
        .expect("setup token string")
        .to_string();

    let body: Value = client
        .get(format!("{base}/api/v1/status"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(body["setup_token_active"], true);

    let response = client
        .post(format!("{base}/api/v1/admin/setup-token/consume"))
        .json(&json!({ "token": token }))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "first setup token consumption should succeed",
    );

    let body: Value = response.json().await?;
    assert_eq!(body["consumed"], true);

    let response = client
        .post(format!("{base}/api/v1/admin/setup-token/consume"))
        .json(&json!({ "token": token }))
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

    shutdown_macronode(node, &client, &base).await
}

#[tokio::test(flavor = "multi_thread")]
async fn expired_setup_token_fails_closed_while_headless_node_stays_healthy() -> Result<()> {
    let (node, client, base) = spawn_macronode("1s").await?;

    let issued: Value = client
        .post(format!("{base}/api/v1/admin/setup-token"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(issued["status"], "setup token issued");
    assert_eq!(issued["expires_in_seconds"], 1);

    let token = issued["token"]
        .as_str()
        .expect("setup token string")
        .to_string();

    let active: Value = client
        .get(format!("{base}/api/v1/status"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(active["setup_token_active"], true);
    assert_headless_non_authoritative_status(&active);

    sleep(Duration::from_millis(1_200)).await;

    let expired: Value = client
        .get(format!("{base}/api/v1/status"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(
        expired["setup_token_active"], false,
        "status must clear an expired runtime credential",
    );
    assert_headless_non_authoritative_status(&expired);

    let response = client
        .post(format!("{base}/api/v1/admin/setup-token/consume"))
        .json(&json!({ "token": token }))
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

    let rejected: Value = response.json().await?;
    assert_eq!(rejected["status"], "setup token rejected");
    assert_eq!(rejected["consumed"], false);

    let health = client.get(format!("{base}/healthz")).send().await?;

    assert!(
        health.status().is_success(),
        "expired setup credentials must not stop the headless Service Node",
    );

    println!(
        "Phase 23A passed: an expired setup token was rejected and cleared, \
         the optional admin UI remained disabled, the headless Service Node \
         stayed healthy, and no economic or finality authority was created."
    );

    shutdown_macronode(node, &client, &base).await
}
