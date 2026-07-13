//! RO:WHAT — Live HTTP proof for macronode persistence-review controls.
//!
//! RO:WHY — Phase 11F requires operator commands to reach one real process
//! catalog rather than returning disconnected or fabricated success.
//!
//! RO:INTERACTS — macronode binary, guarded admin router, RuntimeStatus, and
//! svc-storage persistence catalog.
//!
//! RO:INVARIANTS — exact B3 only; duplicate registration is idempotent;
//! pending lists reflect real transitions; rejection claims no durability.
//!
//! RO:SECURITY — loopback test only; no durable byte, provider, reward, wallet,
//! or ledger mutation.
//!
//! RO:TEST — cargo test -p macronode --test persistence_http.

use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use tokio::time::sleep;

const OBJECT: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const APPROVED_OBJECT: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

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
        .context("read free loopback address")?
        .port())
}

async fn spawn_macronode() -> Result<(MacronodeGuard, Client, String)> {
    let admin_port = free_loopback_port()?;
    let gateway_port = free_loopback_port()?;

    let mut command = Command::new(macronode_bin());

    command
        .arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{admin_port}"))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{gateway_port}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = command
        .spawn()
        .context("failed to spawn macronode binary")?;

    let guard = MacronodeGuard { child };

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("failed to build HTTP client")?;

    let base = format!("http://127.0.0.1:{admin_port}");

    for _ in 0..50 {
        match client.get(format!("{base}/healthz")).send().await {
            Ok(response) if response.status().is_success() => {
                return Ok((guard, client, base));
            }
            _ => sleep(Duration::from_millis(200)).await,
        }
    }

    Err(anyhow!("macronode did not become healthy in time"))
}

async fn wait_for_ready(client: &Client, base: &str) -> Result<()> {
    for _ in 0..50 {
        match client.get(format!("{base}/readyz")).send().await {
            Ok(response) if response.status().is_success() => {
                return Ok(());
            }
            _ => sleep(Duration::from_millis(200)).await,
        }
    }

    Err(anyhow!(
        "macronode did not become ready for persistence approval"
    ))
}

fn assert_non_authoritative(response: &Value) {
    assert_eq!(response["durableBytesWritten"], false);
    assert_eq!(response["walletMutation"], false);
    assert_eq!(response["ledgerMutation"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn persistence_http_controls_share_real_runtime_state() -> Result<()> {
    let (_node, client, base) = spawn_macronode().await?;
    wait_for_ready(&client, &base).await?;

    let registered: Value = client
        .post(format!("{base}/api/v1/persistence/register"))
        .json(&json!({
            "object": OBJECT,
            "assetKind": "image"
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(registered["action"], "register");
    assert_eq!(registered["changed"], true);
    assert_eq!(registered["candidate"]["state"], "ephemeral_unvetted");
    assert_eq!(registered["candidate"]["durableStorageEligible"], false);
    assert_non_authoritative(&registered);

    let duplicate: Value = client
        .post(format!("{base}/api/v1/persistence/register"))
        .json(&json!({
            "object": OBJECT,
            "assetKind": "video"
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(duplicate["changed"], false);
    assert_eq!(
        duplicate["candidate"]["assetKind"], "image",
        "duplicate registration must not reset canonical metadata"
    );

    let pending: Value = client
        .get(format!("{base}/api/v1/persistence/pending?limit=10"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(pending["count"], 1);
    assert_eq!(pending["items"][0]["object"], OBJECT);
    assert_eq!(pending["items"][0]["state"], "ephemeral_unvetted");
    assert_non_authoritative(&pending);

    let submitted: Value = client
        .post(format!("{base}/api/v1/persistence/submit"))
        .json(&json!({ "object": OBJECT }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(submitted["changed"], true);
    assert_eq!(submitted["candidate"]["state"], "pending_review");
    assert_non_authoritative(&submitted);

    let status: Value = client
        .get(format!("{base}/api/v1/persistence/status/{OBJECT}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(status["candidate"]["object"], OBJECT);
    assert_eq!(status["candidate"]["state"], "pending_review");
    assert_non_authoritative(&status);

    let rejected: Value = client
        .post(format!("{base}/api/v1/persistence/reject"))
        .json(&json!({ "object": OBJECT }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(rejected["changed"], true);
    assert_eq!(rejected["candidate"]["state"], "operator_blocked");
    assert_eq!(rejected["candidate"]["durableStorageEligible"], false);
    assert_non_authoritative(&rejected);

    let after_reject: Value = client
        .get(format!("{base}/api/v1/persistence/pending?limit=10"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(after_reject["count"], 0);

    let invalid = client
        .post(format!("{base}/api/v1/persistence/register"))
        .json(&json!({
            "object": "b3:not-valid",
            "assetKind": "image"
        }))
        .send()
        .await?;

    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    let invalid_body: Value = invalid.json().await?;
    assert_eq!(invalid_body["status"], "invalid_object");
    assert_non_authoritative(&invalid_body);

    let approval_candidate: Value = client
        .post(format!("{base}/api/v1/persistence/register"))
        .json(&json!({
            "object": APPROVED_OBJECT,
            "assetKind": "image"
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(
        approval_candidate["candidate"]["state"],
        "ephemeral_unvetted"
    );
    assert_non_authoritative(&approval_candidate);

    let approved: Value = client
        .post(format!("{base}/api/v1/persistence/approve"))
        .json(&json!({ "object": APPROVED_OBJECT }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(approved["action"], "approve");
    assert_eq!(approved["changed"], true);
    assert_eq!(approved["candidate"]["state"], "verified_persistent");
    assert_eq!(approved["candidate"]["durableStorageEligible"], true);
    assert_non_authoritative(&approved);

    let repeated: Value = client
        .post(format!("{base}/api/v1/persistence/approve"))
        .json(&json!({ "object": APPROVED_OBJECT }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(repeated["changed"], false);
    assert_eq!(repeated["candidate"]["state"], "verified_persistent");
    assert_non_authoritative(&repeated);

    Ok(())
}
