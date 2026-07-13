//! RO:WHAT — End-to-end `crabnode persistence *` against a spawned macronode.
//!
//! RO:WHY — Phase 11F must prove separate CLI processes reach one real shared
//! macronode persistence catalog rather than isolated fake HTTP fixtures.
//!
//! RO:INTERACTS — real crabnode binary, real macronode admin router,
//! RuntimeStatus, and svc-storage PersistenceCatalog.
//!
//! RO:INVARIANTS — exact B3 only; registration begins amnesia-first;
//! submission and rejection persist across CLI invocations; pending listing
//! reflects canonical runtime state.
//!
//! RO:SECURITY — loopback only; metadata and eligibility workflow only;
//! no durable byte write, provider mutation, reward, wallet, or ledger
//! authority.
//!
//! RO:TEST — cargo test -p macronode --test crabnode_persistence_live.

use std::{
    net::TcpListener,
    process::{Child, Command, Output, Stdio},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::Value;
use tokio::time::sleep;

const CID: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn macronode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_macronode")
}

fn crabnode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_crabnode")
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

async fn spawn_macronode() -> Result<(MacronodeGuard, String)> {
    let admin_port = free_loopback_port()?;
    let gateway_port = free_loopback_port()?;

    let child = Command::new(macronode_bin())
        .arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{admin_port}"))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{gateway_port}"))
        .env_remove("RON_ADMIN_TOKEN")
        .env_remove("MACRONODE_DEV_INSECURE")
        .env_remove("RON_SERVICE_NODE_MODERATION_POLICY_PATH")
        .env_remove("RON_SERVICE_NODE_SIGNED_MODERATION_POLICY_PATH")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to spawn macronode binary")?;

    let base = format!("http://127.0.0.1:{admin_port}");

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("build macronode health client")?;

    for _ in 0..50 {
        match client.get(format!("{base}/healthz")).send().await {
            Ok(response) if response.status().is_success() => {
                return Ok((MacronodeGuard { child }, base));
            }
            _ => sleep(Duration::from_millis(200)).await,
        }
    }

    Err(anyhow!("macronode did not become healthy in time"))
}

fn run_crabnode(base: &str, args: &[&str]) -> Result<Output> {
    Command::new(crabnode_bin())
        .arg("--admin-url")
        .arg(base)
        .args(args)
        .output()
        .context("run crabnode persistence command")
}

fn successful_json(output: Output) -> Result<Value> {
    if !output.status.success() {
        return Err(anyhow!(
            "crabnode failed: status={:?}; stderr={}; stdout={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        ));
    }

    serde_json::from_slice(&output.stdout).context("parse crabnode stdout as JSON")
}

fn assert_non_authoritative(value: &Value) {
    assert_eq!(value["durableBytesWritten"], false);
    assert_eq!(value["walletMutation"], false);
    assert_eq!(value["ledgerMutation"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn crabnode_persistence_commands_share_live_runtime_state() -> Result<()> {
    let (_node, base) = spawn_macronode().await?;

    let registered = successful_json(run_crabnode(
        &base,
        &["persistence", "register", CID, "IMAGE"],
    )?)?;

    assert_eq!(registered["action"], "register");
    assert_eq!(registered["changed"], true);
    assert_eq!(registered["candidate"]["object"], CID);
    assert_eq!(registered["candidate"]["assetKind"], "image");
    assert_eq!(registered["candidate"]["state"], "ephemeral_unvetted");
    assert_eq!(registered["candidate"]["durableStorageEligible"], false);
    assert_non_authoritative(&registered);

    // A separate crabnode process reads the same macronode runtime catalog.
    let status = successful_json(run_crabnode(&base, &["persistence", "status", CID])?)?;

    assert_eq!(status["candidate"]["object"], CID);
    assert_eq!(status["candidate"]["assetKind"], "image");
    assert_eq!(status["candidate"]["state"], "ephemeral_unvetted");
    assert_non_authoritative(&status);

    let pending = successful_json(run_crabnode(&base, &["persistence", "pending", "10"])?)?;

    assert_eq!(pending["limit"], 10);
    assert_eq!(pending["count"], 1);
    assert_eq!(pending["items"][0]["object"], CID);
    assert_eq!(pending["items"][0]["state"], "ephemeral_unvetted");
    assert_non_authoritative(&pending);

    let submitted = successful_json(run_crabnode(&base, &["persistence", "submit", CID])?)?;

    assert_eq!(submitted["action"], "submit_for_review");
    assert_eq!(submitted["changed"], true);
    assert_eq!(submitted["candidate"]["state"], "pending_review");
    assert_non_authoritative(&submitted);

    let after_submit = successful_json(run_crabnode(&base, &["persistence", "pending", "10"])?)?;

    assert_eq!(after_submit["count"], 1);
    assert_eq!(after_submit["items"][0]["state"], "pending_review");
    assert_non_authoritative(&after_submit);

    let rejected = successful_json(run_crabnode(&base, &["persistence", "reject", CID])?)?;

    assert_eq!(rejected["action"], "reject");
    assert_eq!(rejected["changed"], true);
    assert_eq!(rejected["candidate"]["state"], "operator_blocked");
    assert_eq!(rejected["candidate"]["durableStorageEligible"], false);
    assert_non_authoritative(&rejected);

    let final_status = successful_json(run_crabnode(&base, &["persistence", "status", CID])?)?;

    assert_eq!(final_status["candidate"]["state"], "operator_blocked");
    assert_non_authoritative(&final_status);

    let final_pending = successful_json(run_crabnode(&base, &["persistence", "pending", "10"])?)?;

    assert_eq!(final_pending["count"], 0);
    assert_non_authoritative(&final_pending);

    // Repeating an already-applied local rejection is a truthful no-op.
    let repeated = successful_json(run_crabnode(&base, &["persistence", "reject", CID])?)?;

    assert_eq!(repeated["action"], "reject");
    assert_eq!(repeated["changed"], false);
    assert_eq!(repeated["candidate"]["state"], "operator_blocked");
    assert_non_authoritative(&repeated);

    Ok(())
}
