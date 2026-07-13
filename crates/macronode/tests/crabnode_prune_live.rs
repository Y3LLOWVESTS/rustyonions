//! RO:WHAT — Live Phase 10 prune exit-gate test.
//!
//! RO:WHY — Prove the operator CLI reaches the real macronode coordinator,
//! deletes exact local bytes, evaluates the registered DHT/index surfaces,
//! and becomes idempotent on a repeated request.
//!
//! RO:INTERACTS — crabnode binary, macronode admin plane, embedded
//! svc-storage, embedded svc-dht, embedded svc-index.
//!
//! RO:INVARIANTS — loopback only; opt-in deterministic object; no
//! network-wide deletion; no resolve-cache or manifest-pointer deletion;
//! no wallet, ledger, or reward-finality claim.

use std::{
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use tokio::time::sleep;

const ADMIN_PORT: u16 = 18680;
const GATEWAY_PORT: u16 = 18690;
const STORAGE_PORT: u16 = 18700;
const INDEX_PORT: u16 = 18710;
const DHT_PORT: u16 = 18720;

const SEED_CID: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn child_mut(&mut self) -> &mut Child {
        self.child
            .as_mut()
            .expect("macronode child must still be present")
    }

    fn mark_exited(&mut self) {
        self.child = None;
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn macronode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_macronode")
}

fn crabnode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_crabnode")
}

fn spawn_seeded_node() -> Result<ChildGuard> {
    let child = Command::new(macronode_bin())
        .arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{ADMIN_PORT}"))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{GATEWAY_PORT}"))
        .env("RON_STORAGE_ADDR", format!("127.0.0.1:{STORAGE_PORT}"))
        .env("INDEX_BIND", format!("127.0.0.1:{INDEX_PORT}"))
        .env("RON_DHT_ADDR", format!("127.0.0.1:{DHT_PORT}"))
        .env("RON_SERVICE_NODE_SEED_OBJECT", "1")
        .env_remove("RON_ADMIN_TOKEN")
        .env_remove("MACRONODE_DEV_INSECURE")
        .env_remove("MACRONODE_DEV_READY")
        .env_remove("RON_SERVICE_NODE_MODERATION_POLICY_PATH")
        .env_remove("RON_SERVICE_NODE_SIGNED_MODERATION_POLICY_PATH")
        .env_remove("RON_SERVICE_NODE_MODERATION_TRUSTED_SIGNER_ID")
        .env_remove("RON_SERVICE_NODE_MODERATION_TRUSTED_PUBLIC_KEY_HEX")
        .env_remove("RON_SERVICE_NODE_MODERATION_ACCEPTED_STATE_PATH")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to spawn seeded macronode")?;

    Ok(ChildGuard::new(child))
}

async fn wait_for_truthful_readiness(client: &Client, admin_base: &str) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(20);

    loop {
        if let Ok(response) = client.get(format!("{admin_base}/readyz")).send().await {
            if response.status() == StatusCode::OK {
                let body: Value = response.json().await.context("decode /readyz body")?;

                if body["ready"] == true
                    && body["mode"] == "truthful"
                    && body["deps"]["storage"] == "ok"
                    && body["deps"]["index"] == "ok"
                    && body["deps"]["dht"] == "ok"
                {
                    return Ok(());
                }
            }
        }

        if Instant::now() >= deadline {
            return Err(anyhow!(
                "macronode did not reach truthful storage/index/DHT readiness"
            ));
        }

        sleep(Duration::from_millis(100)).await;
    }
}

fn run_crabnode_prune(admin_base: &str) -> Result<Value> {
    let output = Command::new(crabnode_bin())
        .args(["--admin-url", admin_base, "prune", SEED_CID])
        .output()
        .context("failed to run crabnode prune")?;

    let stdout = String::from_utf8(output.stdout).context("crabnode stdout was not UTF-8")?;

    let stderr = String::from_utf8(output.stderr).context("crabnode stderr was not UTF-8")?;

    if !output.status.success() {
        return Err(anyhow!(
            "crabnode prune failed: status={}; stdout={stdout:?}; stderr={stderr:?}",
            output.status
        ));
    }

    serde_json::from_str(stdout.trim()).context("failed to decode crabnode prune JSON")
}

async fn shutdown_node(guard: &mut ChildGuard, client: &Client, admin_base: &str) -> Result<()> {
    let response = client
        .post(format!("{admin_base}/api/v1/shutdown"))
        .send()
        .await
        .context("failed to request macronode shutdown")?;

    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        if guard
            .child_mut()
            .try_wait()
            .context("failed to poll macronode child")?
            .is_some()
        {
            guard.mark_exited();
            return Ok(());
        }

        if Instant::now() >= deadline {
            return Err(anyhow!("macronode did not exit after shutdown"));
        }

        sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn crabnode_prunes_live_local_runtime_and_is_idempotent() -> Result<()> {
    let mut guard = spawn_seeded_node()?;

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("failed to build test HTTP client")?;

    let admin_base = format!("http://127.0.0.1:{ADMIN_PORT}");

    let storage_base = format!("http://127.0.0.1:{STORAGE_PORT}");

    wait_for_truthful_readiness(&client, &admin_base).await?;

    // The deterministic object exists before the operator action.
    let before = client
        .get(format!("{storage_base}/o/{SEED_CID}"))
        .send()
        .await
        .context("failed to read seeded object before prune")?;

    assert_eq!(before.status(), StatusCode::OK);

    let first = run_crabnode_prune(&admin_base)?;

    assert_eq!(first["status"], "pruned");
    assert_eq!(first["complete"], true);
    assert_eq!(first["changed"], true);
    assert_eq!(first["scope"], "local_storage_provider_and_index_cache");

    assert_eq!(first["local_bytes"]["status"], "removed");
    assert_eq!(first["local_bytes"]["bytes"], 3);

    // These authoritative surfaces are registered and were checked. They
    // truthfully report absence because this seed does not advertise itself.
    assert_eq!(first["provider"]["status"], "not_found");
    assert_eq!(first["index_cache_invalidation"], "not_found");

    assert_eq!(first["network_propagation"], false);
    assert_eq!(first["resolve_cache_invalidation"], false);
    assert_eq!(first["manifest_pointer_removal"], false);
    assert_eq!(first["wallet_mutation"], false);
    assert_eq!(first["ledger_mutation"], false);
    assert_eq!(first["reward_finality"], false);

    // Physical local bytes are no longer served.
    let after = client
        .get(format!("{storage_base}/o/{SEED_CID}"))
        .send()
        .await
        .context("failed to read object after prune")?;

    assert_eq!(after.status(), StatusCode::NOT_FOUND);

    // Repeating the same operator request is a successful no-op, not fake
    // mutation or an error.
    let second = run_crabnode_prune(&admin_base)?;

    assert_eq!(second["status"], "already_absent");
    assert_eq!(second["complete"], true);
    assert_eq!(second["changed"], false);
    assert_eq!(second["local_bytes"]["status"], "not_found");
    assert_eq!(second["provider"]["status"], "not_found");
    assert_eq!(second["index_cache_invalidation"], "not_found");

    shutdown_node(&mut guard, &client, &admin_base).await
}
