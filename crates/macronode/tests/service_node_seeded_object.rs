//! RO:WHAT — Live service-node seeded-object and OAP fetch acceptance test.
//! RO:WHY — Prove a headless macronode serves and verifies a preloaded b3 object over OAP/1.
//! RO:INTERACTS — macronode binary, embedded svc-storage, OAP wire flow, admin status/shutdown.
//! RO:INVARIANTS — opt-in seed; bounded OAP; exact b3 bytes; no economic authority claim.
//! RO:CONFIG — dedicated admin/gateway/storage/index ports and seed-object enable switch.
//! RO:SECURITY — loopback-only test; no provider publication, wallet, ledger, or quorum.
//! RO:TEST — cargo test -p macronode --test service_node_seeded_object.

use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client, StatusCode};
use ron_proto::ContentId;
use serde_json::Value;
use svc_storage::oap_object::{build_obj_get_request, encode_frame_wire, verify_obj_stream_wire};
use tokio::time::sleep;

const ADMIN_PORT: u16 = 18280;
const GATEWAY_PORT: u16 = 18290;
const STORAGE_PORT: u16 = 18300;
const INDEX_PORT: u16 = 18310;

const SEED_BYTES: &[u8] = b"abc";
const SEED_CID: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn child_mut(&mut self) -> &mut Child {
        self.child.as_mut().expect("child should still be present")
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

fn spawn_seeded_service_node() -> Result<ChildGuard> {
    let bin = env!("CARGO_BIN_EXE_macronode");

    let child = Command::new(bin)
        .arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{ADMIN_PORT}"))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{GATEWAY_PORT}"))
        .env("RON_STORAGE_ADDR", format!("127.0.0.1:{STORAGE_PORT}"))
        .env("INDEX_BIND", format!("127.0.0.1:{INDEX_PORT}"))
        .env("RON_SERVICE_NODE_SEED_OBJECT", "1")
        .env_remove("RON_SERVICE_NODE_MODERATION_POLICY_PATH")
        .env_remove("RON_SERVICE_NODE_SIGNED_MODERATION_POLICY_PATH")
        .env_remove("RON_SERVICE_NODE_MODERATION_TRUSTED_SIGNER_ID")
        .env_remove("RON_SERVICE_NODE_MODERATION_TRUSTED_PUBLIC_KEY_HEX")
        .env_remove("RON_SERVICE_NODE_MODERATION_ACCEPTED_STATE_PATH")
        .env_remove("MACRONODE_DEV_READY")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to spawn seeded macronode")?;

    Ok(ChildGuard::new(child))
}

async fn wait_for_truthful_readiness(client: &Client, admin_base: &str) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(20);

    loop {
        if let Ok(response) = client.get(format!("{admin_base}/readyz")).send().await {
            if response.status() == StatusCode::OK {
                let body: Value = response
                    .json()
                    .await
                    .context("failed to decode /readyz body")?;

                if body["ready"] == true && body["mode"] == "truthful" {
                    return Ok(body);
                }
            }
        }

        if Instant::now() >= deadline {
            return Err(anyhow!(
                "seeded service node did not reach truthful readiness in time"
            ));
        }

        sleep(Duration::from_millis(100)).await;
    }
}

async fn shutdown_node(guard: &mut ChildGuard, client: &Client, admin_base: &str) -> Result<()> {
    let response = client
        .post(format!("{admin_base}/api/v1/shutdown"))
        .send()
        .await
        .context("failed to call macronode shutdown")?;

    assert!(
        response.status().is_success(),
        "shutdown endpoint returned {}",
        response.status()
    );

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
            return Err(anyhow!(
                "seeded macronode did not exit after shutdown request"
            ));
        }

        sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn service_node_serves_opt_in_seeded_object() -> Result<()> {
    let mut guard = spawn_seeded_service_node()?;

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("failed to build HTTP client")?;

    let admin_base = format!("http://127.0.0.1:{ADMIN_PORT}");
    let storage_base = format!("http://127.0.0.1:{STORAGE_PORT}");

    let ready = wait_for_truthful_readiness(&client, &admin_base).await?;

    assert_eq!(ready["deps"]["storage"], "ok");
    assert_eq!(ready["deps"]["index"], "ok");
    assert_eq!(ready["deps"]["gateway"], "ok");

    // No upload is made to the live node. This GET succeeds only if startup
    // seeded the object before truthful readiness became available.
    let response = client
        .get(format!("{storage_base}/o/{SEED_CID}"))
        .send()
        .await
        .context("failed to fetch pre-seeded object")?;

    assert_eq!(response.status(), StatusCode::OK);

    let headers = response.headers().clone();
    let body = response
        .bytes()
        .await
        .context("failed to read seeded object body")?;

    assert_eq!(body.as_ref(), SEED_BYTES);
    assert_eq!(
        headers
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok()),
        Some("3")
    );
    assert_eq!(
        headers
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok()),
        Some("\"6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85\"")
    );

    // Fetch the same startup-seeded object through the real binary OAP route.
    let seed_cid: ContentId = SEED_CID.parse().context("seed CID should parse")?;

    let request_wire = encode_frame_wire(
        build_obj_get_request(seed_cid.clone(), 0xCAFE, 9001).context("build live OBJ_GET")?,
    )
    .context("encode live OBJ_GET")?;

    let oap_response = client
        .post(format!("{storage_base}/oap/obj-get"))
        .header(header::CONTENT_TYPE, "application/oap")
        .body(request_wire)
        .send()
        .await
        .context("live OAP OBJ_GET request failed")?;

    assert_eq!(oap_response.status(), StatusCode::OK);
    assert_eq!(
        oap_response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/oap")
    );

    let response_wire = oap_response
        .bytes()
        .await
        .context("read live OAP response stream")?;

    let verified = verify_obj_stream_wire(&seed_cid, &response_wire)
        .context("live OAP stream failed mandatory b3 verification")?;

    assert_eq!(verified.as_ref(), SEED_BYTES);

    let status: Value = client
        .get(format!("{admin_base}/api/v1/status"))
        .send()
        .await
        .context("failed to query service-node status")?
        .error_for_status()
        .context("service-node status was not successful")?
        .json()
        .await
        .context("failed to decode service-node status")?;

    assert_eq!(status["node_role"], "service_node");
    assert_eq!(status["node_profile"], "macronode");
    assert_eq!(status["headless_mode"], true);
    assert_eq!(status["admin_ui_enabled"], false);
    assert_eq!(status["admin_ui_runtime_required"], false);
    assert_eq!(status["services"]["svc-storage"], "ok");

    // OAP object fetch and mandatory full-digest verification are now real.
    // Provider advertisement, quorum, and economic authority remain inactive.
    assert_eq!(status["oap"]["runtime_state"], "active_local_http_oap");
    assert_eq!(status["oap"]["object_fetch_active"], true);
    assert_eq!(status["oap"]["full_digest_verification_active"], true);
    assert_eq!(status["policy"]["oap_serve_policy_enforced"], true);
    assert_eq!(status["policy"]["serve_policy_enforced"], true);
    assert_eq!(status["provider"]["advertisement_active"], false);
    assert_eq!(status["provider"]["provider_records_published"], 0);
    assert_eq!(status["service_quorum_enabled"], false);
    assert_eq!(status["wallet_execution_participant"], false);
    assert_eq!(status["ledger_replay_enabled"], false);

    shutdown_node(&mut guard, &client, &admin_base).await?;

    Ok(())
}
