//! RO:WHAT — Integration tests for macronode reward-recipient HTTP endpoints.
//! RO:WHY — Proves `crabnode rewards *` has real local macronode endpoints behind it.
//! RO:INVARIANTS — runtime-local only; no wallet mutation, no ledger mutation, no confirmed ROC.

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    time::Duration,
};
use tokio::time::sleep;

fn macronode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_macronode")
}

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
        .context("read loopback local addr")?
        .port())
}

async fn spawn_macronode() -> Result<(MacronodeGuard, Client, String)> {
    let admin_port = free_loopback_port()?;
    let gateway_port = free_loopback_port()?;

    let mut cmd = Command::new(macronode_bin());
    cmd.arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{admin_port}"))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{gateway_port}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn().context("failed to spawn macronode binary")?;
    let guard = MacronodeGuard { child };

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("failed to build reqwest client")?;

    let base = format!("http://127.0.0.1:{admin_port}");

    for _ in 0..50 {
        match client.get(format!("{base}/healthz")).send().await {
            Ok(resp) if resp.status().is_success() => return Ok((guard, client, base)),
            _ => sleep(Duration::from_millis(200)).await,
        }
    }

    Err(anyhow!("macronode did not become healthy in time"))
}

#[tokio::test(flavor = "multi_thread")]
async fn reward_recipient_http_status_bind_and_rotate_are_runtime_local() -> Result<()> {
    let (_node, client, base) = spawn_macronode().await?;

    let initial: Value = client
        .get(format!("{base}/api/v1/rewards/status"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(initial["state"], "unbound");
    assert_eq!(initial["walletMutation"], false);
    assert_eq!(initial["ledgerMutation"], false);
    assert!(initial["confirmedRoc"].is_null());

    let bound: Value = client
        .post(format!("{base}/api/v1/rewards/bind"))
        .json(&serde_json::json!({
            "rewardRecipientDisplayAddress": "@operator",
            "note": "test bind request"
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(bound["status"], "binding request recorded");
    assert_eq!(bound["state"], "bound");
    assert_eq!(bound["rewardRecipientDisplayAddress"], "@operator");
    assert_eq!(bound["walletMutation"], false);
    assert_eq!(bound["ledgerMutation"], false);
    assert!(bound["confirmedRoc"].is_null());

    let rotated: Value = client
        .post(format!("{base}/api/v1/rewards/rotate"))
        .json(&serde_json::json!({
            "newRewardRecipientDisplayAddress": "@new-operator",
            "note": "test rotation request"
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(rotated["status"], "rotation request recorded");
    assert_eq!(rotated["state"], "pending_rotation");
    assert_eq!(rotated["rewardRecipientDisplayAddress"], "@operator");
    assert_eq!(rotated["pendingRotationDisplayAddress"], "@new-operator");
    assert_eq!(rotated["walletMutation"], false);
    assert_eq!(rotated["ledgerMutation"], false);
    assert!(rotated["confirmedRoc"].is_null());

    let after: Value = client
        .get(format!("{base}/api/v1/rewards/status"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(after["state"], "pending_rotation");
    assert_eq!(after["rewardRecipientDisplayAddress"], "@operator");
    assert_eq!(after["pendingRotationDisplayAddress"], "@new-operator");
    assert_eq!(after["walletMutation"], false);
    assert_eq!(after["ledgerMutation"], false);
    assert!(after["confirmedRoc"].is_null());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn reward_recipient_http_rejects_bad_address_and_unbound_rotation() -> Result<()> {
    let (_node, client, base) = spawn_macronode().await?;

    let bad_bind = client
        .post(format!("{base}/api/v1/rewards/bind"))
        .json(&serde_json::json!({
            "rewardRecipientDisplayAddress": "operator"
        }))
        .send()
        .await?;

    assert_eq!(bad_bind.status(), reqwest::StatusCode::BAD_REQUEST);

    let unbound_rotate = client
        .post(format!("{base}/api/v1/rewards/rotate"))
        .json(&serde_json::json!({
            "newRewardRecipientDisplayAddress": "@new-operator"
        }))
        .send()
        .await?;

    assert_eq!(unbound_rotate.status(), reqwest::StatusCode::CONFLICT);

    let body: Value = unbound_rotate.json().await?;
    assert_eq!(body["walletMutation"], false);
    assert_eq!(body["ledgerMutation"], false);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("cannot rotate before"));

    Ok(())
}
