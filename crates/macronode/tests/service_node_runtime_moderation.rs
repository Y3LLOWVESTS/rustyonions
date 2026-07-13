//! Live macronode runtime-moderation acceptance tests.
//!
//! A configured bounded policy must reach the embedded OAP route. Invalid
//! configured policy must leave truthful storage readiness false.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client, StatusCode};
use ron_proto::ContentId;
use serde_json::{json, Value};
use svc_storage::oap_object::{build_obj_get_request, encode_frame_wire};
use tokio::time::sleep;

const SEED_CID: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
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

struct PolicyFile {
    path: PathBuf,
}

impl PolicyFile {
    fn write(label: &str, bytes: &[u8]) -> Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock was before Unix epoch")?
            .as_nanos();

        let path = std::env::temp_dir().join(format!(
            "macronode-runtime-moderation-{label}-{}-{nonce}.json",
            std::process::id(),
        ));

        fs::write(&path, bytes).context("failed to write temporary policy")?;

        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PolicyFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_dir_all(self.path.with_extension("index.sled"));
    }
}

struct Ports {
    admin: u16,
    gateway: u16,
    storage: u16,
    index: u16,
}

fn spawn_node(ports: &Ports, policy_path: &Path) -> Result<ChildGuard> {
    let bin = env!("CARGO_BIN_EXE_macronode");

    let child = Command::new(bin)
        .arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{}", ports.admin))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{}", ports.gateway))
        .env("RON_STORAGE_ADDR", format!("127.0.0.1:{}", ports.storage))
        .env("INDEX_BIND", format!("127.0.0.1:{}", ports.index))
        .env("RON_INDEX_DB", policy_path.with_extension("index.sled"))
        .env("RON_SERVICE_NODE_SEED_OBJECT", "1")
        .env("RON_SERVICE_NODE_MODERATION_POLICY_PATH", policy_path)
        .env_remove("RON_SERVICE_NODE_SIGNED_MODERATION_POLICY_PATH")
        .env_remove("RON_SERVICE_NODE_MODERATION_TRUSTED_SIGNER_ID")
        .env_remove("RON_SERVICE_NODE_MODERATION_TRUSTED_PUBLIC_KEY_HEX")
        .env_remove("RON_SERVICE_NODE_MODERATION_ACCEPTED_STATE_PATH")
        .env_remove("MACRONODE_DEV_READY")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to spawn moderated macronode")?;

    Ok(ChildGuard::new(child))
}

async fn wait_for_admin(client: &Client, admin_port: u16) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let url = format!("http://127.0.0.1:{admin_port}/version");

    loop {
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                return Ok(());
            }
            _ => {}
        }

        if Instant::now() >= deadline {
            return Err(anyhow!("macronode admin plane did not start"));
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
            return Err(anyhow!("macronode never reached truthful readiness"));
        }

        sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn configured_policy_blocks_seed_over_live_oap() -> Result<()> {
    let policy = PolicyFile::write(
        "blocked-seed",
        format!(r#"{{"local_block":["{SEED_CID}"]}}"#).as_bytes(),
    )?;

    let ports = Ports {
        admin: 18480,
        gateway: 18490,
        storage: 18500,
        index: 18510,
    };

    let _node = spawn_node(&ports, policy.path())?;

    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .context("failed to build HTTP client")?;

    wait_for_admin(&client, ports.admin).await?;
    let ready = wait_for_ready(&client, ports.admin).await?;

    assert_eq!(ready["deps"]["storage"], "ok");

    let status = status_body(&client, ports.admin).await?;

    assert_eq!(status["policy"]["operator_moderation_active"], true);
    assert_eq!(status["policy"]["moderation_configured"], true);
    assert_eq!(status["policy"]["moderation_state"], "active");
    assert_eq!(status["policy"]["moderation_source"], "unsigned_local");
    assert_eq!(status["policy"]["global_moderation_active"], false);
    assert_eq!(status["policy"]["signed_policy_verified"], false);
    assert!(status["policy"]["signed_policy_epoch"].is_null());
    assert!(status["policy"]["signed_policy_expires_at_unix_s"].is_null());
    assert_eq!(status["policy"]["rollback_guard_persisted"], false);
    assert_eq!(
        status["policy"]["moderation_activation"],
        "startup_snapshot"
    );
    assert_eq!(status["policy"]["moderation_hot_reload"], false);
    assert_eq!(status["policy"]["moderation_load_failed"], false);
    assert_eq!(status["policy"]["moderation_entries"]["total"], 1);
    assert_eq!(status["policy"]["moderation_entries"]["local_block"], 1);
    assert_eq!(status["policy"]["moderation_entries"]["global_deny"], 0);
    assert_eq!(status["policy"]["moderation_entries"]["local_allow"], 0);
    assert_eq!(status["policy"]["moderation_entries"]["owner_tombstone"], 0);
    assert_eq!(status["policy"]["moderation_entries"]["quarantine"], 0);

    let rendered_status = status.to_string();
    assert!(
        !rendered_status.contains(policy.path().to_string_lossy().as_ref()),
        "status must not expose the moderation policy path"
    );
    assert!(
        !rendered_status.contains(SEED_CID),
        "status must not expose exact moderated object IDs"
    );

    assert_eq!(status["policy"]["serve_policy_enforced"], true);
    assert_eq!(status["policy"]["oap_serve_policy_enforced"], true);

    let base = format!("http://127.0.0.1:{}", ports.admin);

    let registered: Value = client
        .post(format!("{base}/api/v1/persistence/register"))
        .json(&json!({
            "object": SEED_CID,
            "assetKind": "image"
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(registered["candidate"]["state"], "ephemeral_unvetted");

    let moderated_approval: Value = client
        .post(format!("{base}/api/v1/persistence/approve"))
        .json(&json!({ "object": SEED_CID }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(moderated_approval["action"], "approve");
    assert_eq!(moderated_approval["changed"], true);
    assert_eq!(
        moderated_approval["candidate"]["state"], "operator_blocked",
        "configured local moderation must defeat operator persistence approval"
    );
    assert_eq!(
        moderated_approval["candidate"]["durableStorageEligible"],
        false
    );
    assert_eq!(moderated_approval["durableBytesWritten"], false);
    assert_eq!(moderated_approval["walletMutation"], false);
    assert_eq!(moderated_approval["ledgerMutation"], false);

    let legacy_url = format!("http://127.0.0.1:{}/o/{SEED_CID}", ports.storage,);

    let legacy_get = client
        .get(&legacy_url)
        .send()
        .await
        .context("live legacy GET moderation request failed")?;

    assert_eq!(legacy_get.status(), StatusCode::FORBIDDEN);

    let legacy_head = client
        .head(&legacy_url)
        .send()
        .await
        .context("live legacy HEAD moderation request failed")?;

    assert_eq!(legacy_head.status(), StatusCode::FORBIDDEN);

    let cid: ContentId = SEED_CID.parse().expect("seed CID must be canonical");

    let wire = encode_frame_wire(build_obj_get_request(cid, 42, 99).expect("OBJ_GET should build"))
        .expect("OBJ_GET should encode");

    let response = client
        .post(format!("http://127.0.0.1:{}/oap/obj-get", ports.storage,))
        .header(header::CONTENT_TYPE, "application/oap")
        .body(wire)
        .send()
        .await
        .context("live OAP moderation request failed")?;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_configured_policy_keeps_storage_not_ready() -> Result<()> {
    let policy = PolicyFile::write("invalid", br#"{"local_block":["not-a-b3"]}"#)?;

    let ports = Ports {
        admin: 18580,
        gateway: 18590,
        storage: 18600,
        index: 18610,
    };

    let _node = spawn_node(&ports, policy.path())?;

    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .context("failed to build HTTP client")?;

    wait_for_admin(&client, ports.admin).await?;

    let deadline = Instant::now() + Duration::from_secs(10);
    let url = format!("http://127.0.0.1:{}/readyz", ports.admin);

    loop {
        if let Ok(response) = client.get(&url).send().await {
            let status = response.status();

            if let Ok(body) = response.json::<Value>().await {
                if status == StatusCode::SERVICE_UNAVAILABLE
                    && body["ready"] == false
                    && body["deps"]["storage"] == "pending"
                {
                    if let Ok(runtime_status) = status_body(&client, ports.admin).await {
                        if runtime_status["policy"]["moderation_state"] == "load_failed" {
                            assert_eq!(
                                runtime_status["policy"]["operator_moderation_active"],
                                false
                            );
                            assert_eq!(runtime_status["policy"]["moderation_configured"], true);
                            assert_eq!(
                                runtime_status["policy"]["moderation_source"],
                                "unsigned_local"
                            );
                            assert_eq!(runtime_status["policy"]["signed_policy_verified"], false);
                            assert_eq!(runtime_status["policy"]["rollback_guard_persisted"], false);
                            assert_eq!(runtime_status["policy"]["moderation_load_failed"], true);
                            assert_eq!(runtime_status["policy"]["moderation_entries"]["total"], 0);

                            let rendered_status = runtime_status.to_string();

                            assert!(
                                !rendered_status.contains(policy.path().to_string_lossy().as_ref()),
                                "status must not expose the rejected policy path"
                            );

                            return Ok(());
                        }
                    }
                }
            }
        }

        if Instant::now() >= deadline {
            return Err(anyhow!(
                "invalid moderation policy did not leave storage readiness false"
            ));
        }

        sleep(Duration::from_millis(100)).await;
    }
}
