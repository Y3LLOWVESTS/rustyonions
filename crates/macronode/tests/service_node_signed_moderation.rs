//! Live signed-moderation startup and rollback acceptance tests.
//!
//! Authenticated global state must compose with canonical local state before
//! storage readiness. The accepted epoch survives restart and rejects rollback.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, StatusCode};
use ron_kms::backends::ed25519;
use ron_policy::{
    encode_ed25519_signature, ModerationPolicy, SignedModerationPolicyV1,
    SIGNED_MODERATION_POLICY_VERSION,
};
use serde_json::Value;
use tokio::time::sleep;

const SEED_CID: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";
const SIGNER_ID: &str = "global-policy-root-1";

#[derive(Clone, Copy)]
struct Ports {
    admin: u16,
    gateway: u16,
    storage: u16,
    index: u16,
}

struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.stop();
    }
}

struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    fn new(label: &str) -> Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock was before Unix epoch")?
            .as_nanos();

        let root = std::env::temp_dir().join(format!(
            "macronode-signed-moderation-{label}-{}-{nonce}",
            std::process::id()
        ));

        fs::create_dir_all(&root).context("failed to create signed-moderation workspace")?;

        Ok(Self { root })
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

struct TestSigner {
    public_key: [u8; 32],
    secret_seed: [u8; 32],
}

impl TestSigner {
    fn generate() -> Self {
        let (public_key, secret_seed) = ed25519::generate();

        Self {
            public_key,
            secret_seed,
        }
    }

    fn public_key_hex(&self) -> String {
        hex_encode(&self.public_key)
    }
}

fn write_signed_snapshot(
    path: &Path,
    signer: &TestSigner,
    epoch: u64,
    valid_for_seconds: u64,
) -> Result<String> {
    let object = SEED_CID
        .parse()
        .expect("seed CID must be a canonical B3 identifier");

    let mut policy = ModerationPolicy::default();
    assert!(policy.insert_global_deny(object));

    let now = now_unix_s();

    let mut snapshot = SignedModerationPolicyV1 {
        version: SIGNED_MODERATION_POLICY_VERSION,
        signer_id: SIGNER_ID.to_owned(),
        epoch,
        issued_at_unix_s: now.saturating_sub(60),
        expires_at_unix_s: now.saturating_add(valid_for_seconds),
        policy,
        signature_hex: String::new(),
    };

    let payload = snapshot
        .signing_payload()
        .context("failed to encode signed moderation payload")?;
    let signature = ed25519::sign(&signer.secret_seed, &payload);
    snapshot.signature_hex = encode_ed25519_signature(&signature);

    let bytes = serde_json::to_vec_pretty(&snapshot)
        .context("failed to encode signed moderation snapshot")?;
    fs::write(path, bytes).context("failed to write signed moderation snapshot")?;

    Ok(snapshot.signature_hex)
}

fn write_local_allow(path: &Path) -> Result<()> {
    let object = SEED_CID
        .parse()
        .expect("seed CID must be a canonical B3 identifier");

    let mut policy = ModerationPolicy::default();
    assert!(policy.insert_local_allow(object));

    let bytes = serde_json::to_vec_pretty(&policy).context("failed to encode local policy")?;

    fs::write(path, bytes).context("failed to write local policy")
}

fn spawn_node(
    ports: Ports,
    snapshot_path: &Path,
    accepted_state_path: &Path,
    index_path: &Path,
    signer: &TestSigner,
    local_policy_path: Option<&Path>,
) -> Result<ChildGuard> {
    let bin = env!("CARGO_BIN_EXE_macronode");

    let mut command = Command::new(bin);
    command
        .arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{}", ports.admin))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{}", ports.gateway))
        .env("RON_STORAGE_ADDR", format!("127.0.0.1:{}", ports.storage))
        .env("INDEX_BIND", format!("127.0.0.1:{}", ports.index))
        .env("RON_INDEX_DB", index_path)
        .env("RON_SERVICE_NODE_SEED_OBJECT", "1")
        .env(
            "RON_SERVICE_NODE_SIGNED_MODERATION_POLICY_PATH",
            snapshot_path,
        )
        .env("RON_SERVICE_NODE_MODERATION_TRUSTED_SIGNER_ID", SIGNER_ID)
        .env(
            "RON_SERVICE_NODE_MODERATION_TRUSTED_PUBLIC_KEY_HEX",
            signer.public_key_hex(),
        )
        .env(
            "RON_SERVICE_NODE_MODERATION_ACCEPTED_STATE_PATH",
            accepted_state_path,
        )
        .env_remove("MACRONODE_DEV_READY")
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    match local_policy_path {
        Some(path) => {
            command.env("RON_SERVICE_NODE_MODERATION_POLICY_PATH", path);
        }
        None => {
            command.env_remove("RON_SERVICE_NODE_MODERATION_POLICY_PATH");
        }
    }

    let child = command
        .spawn()
        .context("failed to spawn signed-moderation macronode")?;

    Ok(ChildGuard::new(child))
}

async fn wait_for_admin(client: &Client, admin_port: u16) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let url = format!("http://127.0.0.1:{admin_port}/version");

    loop {
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => return Ok(()),
            _ => {}
        }

        if Instant::now() >= deadline {
            return Err(anyhow!("macronode admin plane did not start"));
        }

        sleep(Duration::from_millis(100)).await;
    }
}

async fn wait_for_ready(client: &Client, admin_port: u16) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let url = format!("http://127.0.0.1:{admin_port}/readyz");

    loop {
        if let Ok(response) = client.get(&url).send().await {
            let status = response.status();

            if let Ok(body) = response.json::<Value>().await {
                if status == StatusCode::OK && body["ready"] == true {
                    return Ok(body);
                }
            }
        }

        if Instant::now() >= deadline {
            return Err(anyhow!("signed-moderation node never became ready"));
        }

        sleep(Duration::from_millis(100)).await;
    }
}

async fn status_body(client: &Client, admin_port: u16) -> Result<Value> {
    client
        .get(format!("http://127.0.0.1:{admin_port}/api/v1/status"))
        .send()
        .await
        .context("GET /api/v1/status failed")?
        .error_for_status()
        .context("/api/v1/status returned an error")?
        .json::<Value>()
        .await
        .context("failed to decode status body")
}

async fn wait_for_signed_failure(client: &Client, admin_port: u16) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        let readiness = client
            .get(format!("http://127.0.0.1:{admin_port}/readyz"))
            .send()
            .await;

        if let Ok(response) = readiness {
            let response_status = response.status();

            if let Ok(body) = response.json::<Value>().await {
                if response_status == StatusCode::SERVICE_UNAVAILABLE
                    && body["ready"] == false
                    && body["deps"]["storage"] == "pending"
                {
                    if let Ok(status) = status_body(client, admin_port).await {
                        if status["policy"]["moderation_state"] == "load_failed" {
                            return Ok(status);
                        }
                    }
                }
            }
        }

        if Instant::now() >= deadline {
            return Err(anyhow!(
                "signed moderation failure did not keep storage unready"
            ));
        }

        sleep(Duration::from_millis(100)).await;
    }
}

async fn wait_for_signed_expiration(client: &Client, admin_port: u16) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(15);

    loop {
        let readiness = client
            .get(format!("http://127.0.0.1:{admin_port}/readyz"))
            .send()
            .await;

        if let Ok(response) = readiness {
            let response_status = response.status();

            if let Ok(body) = response.json::<Value>().await {
                if response_status == StatusCode::SERVICE_UNAVAILABLE
                    && body["ready"] == false
                    && body["deps"]["storage"] == "pending"
                {
                    if let Ok(status) = status_body(client, admin_port).await {
                        if status["policy"]["moderation_state"] == "expired" {
                            return Ok(status);
                        }
                    }
                }
            }
        }

        if Instant::now() >= deadline {
            return Err(anyhow!(
                "signed moderation expiration did not stop storage readiness"
            ));
        }

        sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn signed_global_composes_with_local_and_survives_restart() -> Result<()> {
    let workspace = TestWorkspace::new("activation")?;
    let signer = TestSigner::generate();

    let snapshot_path = workspace.path("signed-policy.json");
    let accepted_state_path = workspace.path("accepted-state.json");
    let local_policy_path = workspace.path("local-policy.json");

    let signature = write_signed_snapshot(&snapshot_path, &signer, 7, 3_600)?;
    write_local_allow(&local_policy_path)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .context("failed to build HTTP client")?;

    let first_ports = Ports {
        admin: 18680,
        gateway: 18690,
        storage: 18700,
        index: 18710,
    };

    let mut first = spawn_node(
        first_ports,
        &snapshot_path,
        &accepted_state_path,
        &workspace.path("first-index.sled"),
        &signer,
        Some(&local_policy_path),
    )?;

    wait_for_admin(&client, first_ports.admin).await?;
    wait_for_ready(&client, first_ports.admin).await?;

    let status = status_body(&client, first_ports.admin).await?;

    assert_eq!(status["policy"]["moderation_state"], "active");
    assert_eq!(
        status["policy"]["moderation_source"],
        "signed_global_plus_local"
    );
    assert_eq!(status["policy"]["operator_moderation_active"], true);
    assert_eq!(status["policy"]["global_moderation_active"], true);
    assert_eq!(status["policy"]["signed_policy_verified"], true);
    assert_eq!(status["policy"]["signed_policy_epoch"], 7);
    assert!(status["policy"]["signed_policy_expires_at_unix_s"]
        .as_u64()
        .is_some_and(|expires| expires > now_unix_s()));
    assert_eq!(status["policy"]["rollback_guard_persisted"], true);
    assert_eq!(status["policy"]["moderation_entries"]["global_deny"], 1);
    assert_eq!(status["policy"]["moderation_entries"]["local_allow"], 1);
    assert_eq!(status["policy"]["moderation_entries"]["total"], 2);

    let rendered = status.to_string();
    let snapshot_path_text = snapshot_path.to_string_lossy().into_owned();
    let accepted_state_path_text = accepted_state_path.to_string_lossy().into_owned();
    let local_policy_path_text = local_policy_path.to_string_lossy().into_owned();
    let public_key_hex = signer.public_key_hex();

    for forbidden in [
        snapshot_path_text.as_str(),
        accepted_state_path_text.as_str(),
        local_policy_path_text.as_str(),
        signature.as_str(),
        public_key_hex.as_str(),
        SEED_CID,
    ] {
        assert!(
            !rendered.contains(forbidden),
            "status must not expose policy paths, exact IDs, signatures, or trust material"
        );
    }

    let response = client
        .get(format!(
            "http://127.0.0.1:{}/o/{SEED_CID}",
            first_ports.storage
        ))
        .send()
        .await
        .context("signed-policy legacy GET failed")?;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(accepted_state_path.is_file());

    first.stop();

    let restart_ports = Ports {
        admin: 18780,
        gateway: 18790,
        storage: 18800,
        index: 18810,
    };

    let _restart = spawn_node(
        restart_ports,
        &snapshot_path,
        &accepted_state_path,
        &workspace.path("restart-index.sled"),
        &signer,
        Some(&local_policy_path),
    )?;

    wait_for_admin(&client, restart_ports.admin).await?;
    wait_for_ready(&client, restart_ports.admin).await?;

    let restarted_status = status_body(&client, restart_ports.admin).await?;

    assert_eq!(restarted_status["policy"]["moderation_state"], "active");
    assert_eq!(restarted_status["policy"]["signed_policy_epoch"], 7);
    assert_eq!(restarted_status["policy"]["rollback_guard_persisted"], true);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn signed_policy_expiration_stops_storage_fail_closed() -> Result<()> {
    let workspace = TestWorkspace::new("expiration")?;
    let signer = TestSigner::generate();

    let snapshot_path = workspace.path("signed-policy.json");
    let accepted_state_path = workspace.path("accepted-state.json");

    let _signature = write_signed_snapshot(&snapshot_path, &signer, 5, 10)?;

    let ports = Ports {
        admin: 19080,
        gateway: 19090,
        storage: 19100,
        index: 19110,
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .context("failed to build HTTP client")?;

    let _node = spawn_node(
        ports,
        &snapshot_path,
        &accepted_state_path,
        &workspace.path("expiration-index.sled"),
        &signer,
        None,
    )?;

    wait_for_admin(&client, ports.admin).await?;
    wait_for_ready(&client, ports.admin).await?;

    let status = wait_for_signed_expiration(&client, ports.admin).await?;

    assert_eq!(status["policy"]["moderation_source"], "signed_global");
    assert_eq!(status["policy"]["moderation_configured"], true);
    assert_eq!(status["policy"]["operator_moderation_active"], false);
    assert_eq!(status["policy"]["global_moderation_active"], false);
    assert_eq!(status["policy"]["signed_policy_verified"], true);
    assert_eq!(status["policy"]["signed_policy_epoch"], 5);
    assert_eq!(status["policy"]["rollback_guard_persisted"], true);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn persisted_epoch_rejects_signed_policy_rollback() -> Result<()> {
    let workspace = TestWorkspace::new("rollback")?;
    let signer = TestSigner::generate();

    let snapshot_path = workspace.path("signed-policy.json");
    let accepted_state_path = workspace.path("accepted-state.json");

    let _accepted_signature = write_signed_snapshot(&snapshot_path, &signer, 9, 3_600)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .context("failed to build HTTP client")?;

    let accepted_ports = Ports {
        admin: 18880,
        gateway: 18890,
        storage: 18900,
        index: 18910,
    };

    let mut accepted = spawn_node(
        accepted_ports,
        &snapshot_path,
        &accepted_state_path,
        &workspace.path("accepted-index.sled"),
        &signer,
        None,
    )?;

    wait_for_admin(&client, accepted_ports.admin).await?;
    wait_for_ready(&client, accepted_ports.admin).await?;
    accepted.stop();

    let _rollback_signature = write_signed_snapshot(&snapshot_path, &signer, 8, 3_600)?;

    let rollback_ports = Ports {
        admin: 18980,
        gateway: 18990,
        storage: 19000,
        index: 19010,
    };

    let _rollback = spawn_node(
        rollback_ports,
        &snapshot_path,
        &accepted_state_path,
        &workspace.path("rollback-index.sled"),
        &signer,
        None,
    )?;

    wait_for_admin(&client, rollback_ports.admin).await?;
    let status = wait_for_signed_failure(&client, rollback_ports.admin).await?;

    assert_eq!(status["policy"]["moderation_configured"], true);
    assert_eq!(status["policy"]["moderation_source"], "signed_global");
    assert_eq!(status["policy"]["operator_moderation_active"], false);
    assert_eq!(status["policy"]["global_moderation_active"], false);
    assert_eq!(status["policy"]["signed_policy_verified"], false);
    assert!(status["policy"]["signed_policy_epoch"].is_null());
    assert_eq!(status["policy"]["rollback_guard_persisted"], false);
    assert_eq!(status["policy"]["moderation_entries"]["total"], 0);

    Ok(())
}

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::with_capacity(bytes.len() * 2);

    for &byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }

    encoded
}
