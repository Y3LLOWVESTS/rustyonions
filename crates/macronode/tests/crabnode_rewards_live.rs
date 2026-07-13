//! RO:WHAT — End-to-end `crabnode rewards *` against a spawned macronode.
//! RO:WHY — Proves Phase 5B CLI commands talk to real local macronode handlers.
//! RO:INVARIANTS — runtime-local only; no wallet mutation, no ledger mutation, no confirmed ROC.

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::{
    net::TcpListener,
    process::{Child, Command, Output, Stdio},
    time::Duration,
};
use tokio::time::sleep;

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
        .context("read loopback local addr")?
        .port())
}

async fn spawn_macronode() -> Result<(MacronodeGuard, String)> {
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
    let base = format!("http://127.0.0.1:{admin_port}");

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("build healthcheck client")?;

    for _ in 0..50 {
        match client.get(format!("{base}/healthz")).send().await {
            Ok(resp) if resp.status().is_success() => return Ok((MacronodeGuard { child }, base)),
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
        .context("run crabnode")
}

fn successful_json(output: Output) -> Result<Value> {
    if !output.status.success() {
        return Err(anyhow!(
            "crabnode failed: status={:?} stderr={} stdout={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        ));
    }

    serde_json::from_slice(&output.stdout).context("parse crabnode stdout as json")
}

#[tokio::test(flavor = "multi_thread")]
async fn crabnode_rewards_live_show_bind_rotate_flow() -> Result<()> {
    let (_node, base) = spawn_macronode().await?;

    let initial = successful_json(run_crabnode(&base, &["rewards", "show"])?)?;
    assert_eq!(initial["state"], "unbound");
    assert_eq!(initial["walletMutation"], false);
    assert_eq!(initial["ledgerMutation"], false);
    assert!(initial["confirmedRoc"].is_null());

    let bound = successful_json(run_crabnode(&base, &["rewards", "bind", "@operator"])?)?;
    assert_eq!(bound["status"], "binding request recorded");
    assert_eq!(bound["state"], "bound");
    assert_eq!(bound["rewardRecipientDisplayAddress"], "@operator");
    assert_eq!(bound["walletMutation"], false);
    assert_eq!(bound["ledgerMutation"], false);
    assert!(bound["confirmedRoc"].is_null());

    let after_bind = successful_json(run_crabnode(&base, &["rewards", "show"])?)?;
    assert_eq!(after_bind["state"], "bound");
    assert_eq!(after_bind["rewardRecipientDisplayAddress"], "@operator");
    assert!(after_bind["pendingRotationDisplayAddress"].is_null());

    let rotated = successful_json(run_crabnode(
        &base,
        &["rewards", "rotate", "@new-operator"],
    )?)?;
    assert_eq!(rotated["status"], "rotation request recorded");
    assert_eq!(rotated["state"], "pending_rotation");
    assert_eq!(rotated["rewardRecipientDisplayAddress"], "@operator");
    assert_eq!(rotated["pendingRotationDisplayAddress"], "@new-operator");
    assert_eq!(rotated["walletMutation"], false);
    assert_eq!(rotated["ledgerMutation"], false);
    assert!(rotated["confirmedRoc"].is_null());

    let after_rotate = successful_json(run_crabnode(&base, &["rewards", "show"])?)?;
    assert_eq!(after_rotate["state"], "pending_rotation");
    assert_eq!(after_rotate["rewardRecipientDisplayAddress"], "@operator");
    assert_eq!(
        after_rotate["pendingRotationDisplayAddress"],
        "@new-operator"
    );
    assert_eq!(after_rotate["walletMutation"], false);
    assert_eq!(after_rotate["ledgerMutation"], false);
    assert!(after_rotate["confirmedRoc"].is_null());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn crabnode_rewards_live_rejects_bad_bind_and_unbound_rotate() -> Result<()> {
    let (_node, base) = spawn_macronode().await?;

    let bad_bind = run_crabnode(&base, &["rewards", "bind", "operator"])?;
    assert!(!bad_bind.status.success());

    let bad_bind_stderr = String::from_utf8_lossy(&bad_bind.stderr);
    assert!(bad_bind_stderr.contains("@ address"));

    let unbound_rotate = run_crabnode(&base, &["rewards", "rotate", "@new-operator"])?;
    assert!(!unbound_rotate.status.success());

    let stderr = String::from_utf8_lossy(&unbound_rotate.stderr);
    let stdout = String::from_utf8_lossy(&unbound_rotate.stdout);
    assert!(
        (stderr.contains("409") && stderr.contains("cannot rotate before"))
            || stdout.contains("cannot rotate before"),
        "expected unbound rotation rejection, stderr={stderr}, stdout={stdout}"
    );

    Ok(())
}
