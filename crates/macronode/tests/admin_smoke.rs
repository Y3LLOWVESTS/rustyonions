//! RO:WHAT — End-to-end smoke test for the Macronode admin plane.
//! RO:WHY  — Prove that `/version`, `/healthz`, `/readyz`, `/metrics`,
//!           `/api/v1/status`, and `/api/v1/shutdown` all behave sanely.
//!
//! This test boots the real `macronode` binary via `CARGO_BIN_EXE_macronode`,
//! waits for it to come up, hits the core admin endpoints, and then shuts the
//! node down via the HTTP control surface.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::Value;
use tokio::time::sleep;

const ADMIN_PORT: u16 = 18080;
const GATEWAY_PORT: u16 = 18090;
const STORAGE_PORT: u16 = 18100;
const INDEX_PORT: u16 = 18110;

/// Spawn the macronode binary and wait until the **full admin HTTP stack** is
/// available by polling `/version`, not just `/healthz`.
///
/// `/healthz` only proves that the event loop is alive; `/version` requires
/// the admin listener, router, and middleware stack to be bound and serving.
async fn spawn_macronode() -> Result<(Child, Client, String)> {
    let bin = env!("CARGO_BIN_EXE_macronode");

    let mut cmd = Command::new(bin);
    cmd.arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        // Per-test ports to avoid collisions when tests run in parallel.
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{ADMIN_PORT}"))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{GATEWAY_PORT}"))
        .env("RON_STORAGE_ADDR", format!("127.0.0.1:{STORAGE_PORT}"))
        .env("INDEX_BIND", format!("127.0.0.1:{INDEX_PORT}"))
        .env_remove("RON_SERVICE_NODE_MODERATION_POLICY_PATH")
        .env_remove("RON_SERVICE_NODE_SIGNED_MODERATION_POLICY_PATH")
        .env_remove("RON_SERVICE_NODE_MODERATION_TRUSTED_SIGNER_ID")
        .env_remove("RON_SERVICE_NODE_MODERATION_TRUSTED_PUBLIC_KEY_HEX")
        .env_remove("RON_SERVICE_NODE_MODERATION_ACCEPTED_STATE_PATH")
        // Keep test output quiet by default.
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn().context("failed to spawn macronode binary")?;

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("failed to build reqwest client")?;

    let base = format!("http://127.0.0.1:{ADMIN_PORT}");

    // Wait for `/version` to go green, which implies the full HTTP stack is up.
    for _ in 0..50 {
        match client.get(format!("{base}/version")).send().await {
            Ok(resp) if resp.status().is_success() => return Ok((child, client, base)),
            _ => sleep(Duration::from_millis(200)).await,
        }
    }

    Err(anyhow!("macronode did not expose /version in time"))
}

async fn shutdown_macronode(mut child: Child, client: &Client, base: &str) -> Result<()> {
    let resp = client
        .post(format!("{base}/api/v1/shutdown"))
        .send()
        .await
        .context("failed to call /api/v1/shutdown")?;

    // Log status/body when tests are run with --nocapture or RUST_LOG on.
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    eprintln!("[admin_smoke] /shutdown status={status} body={body_text}");

    // Give the process a few seconds to exit cleanly.
    for _ in 0..50 {
        if let Ok(Some(_status)) = child.try_wait() {
            return Ok(());
        }
        sleep(Duration::from_millis(200)).await;
    }

    // If it is still running, kill it to avoid hanging tests.
    let _ = child.kill();
    Err(anyhow!("macronode did not exit cleanly after /shutdown"))
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_plane_smoke() -> Result<()> {
    let (child, client, base) = spawn_macronode().await?;

    // /version
    let resp = client
        .get(format!("{base}/version"))
        .send()
        .await
        .context("GET /version failed")?;
    assert!(resp.status().is_success());
    let body: Value = resp.json().await.context("decode /version body")?;
    // /version contract: includes `service: "macronode"` plus build info.
    assert_eq!(body["service"], "macronode");
    assert!(body["version"].is_string());
    assert!(body["git_sha"].is_string());
    assert!(body["api"]["http"].is_string());

    // /healthz
    let resp = client
        .get(format!("{base}/healthz"))
        .send()
        .await
        .context("GET /healthz failed")?;
    assert!(resp.status().is_success());
    let body: Value = resp.json().await.context("decode /healthz body")?;
    assert_eq!(body["ok"], true);

    // /readyz
    let resp = client
        .get(format!("{base}/readyz"))
        .send()
        .await
        .context("GET /readyz failed")?;
    assert!(
        resp.status().is_success(),
        "expected /readyz 200 when node is up"
    );
    let body: Value = resp.json().await.context("decode /readyz body")?;
    assert_eq!(body["ready"], true);
    // Basic sanity on deps.
    assert_eq!(body["deps"]["config"], "loaded");
    assert_eq!(body["deps"]["network"], "ok");
    assert_eq!(body["deps"]["gateway"], "ok");
    assert_eq!(body["deps"]["storage"], "ok");
    assert_eq!(body["deps"]["index"], "ok");

    // /metrics
    let resp = client
        .get(format!("{base}/metrics"))
        .send()
        .await
        .context("GET /metrics failed")?;
    assert!(resp.status().is_success(), "/metrics must return 200 OK");

    let headers = resp.headers().clone();
    let text = resp.text().await.context("decode /metrics body")?;

    // Content-type should be text/plain; charset=utf-8 (Axum default for String).
    if let Some(ct) = headers.get(reqwest::header::CONTENT_TYPE) {
        let ct = ct.to_str().unwrap_or_default();
        assert!(
            ct.starts_with("text/plain"),
            "expected text/plain content-type for /metrics, got {ct}"
        );
    }

    // We don't yet enforce that the metrics body is non-empty, only that it is
    // reasonably small and successfully returned as text.
    assert!(
        text.len() < 1024 * 1024,
        "/metrics body should not exceed 1 MiB in tests"
    );

    // /api/v1/status
    let resp = client
        .get(format!("{base}/api/v1/status"))
        .send()
        .await
        .context("GET /api/v1/status failed")?;
    assert!(resp.status().is_success());
    let body: Value = resp.json().await.context("decode /api/v1/status body")?;
    // Status contract: uses `profile: "macronode"` (not `service`).
    assert_eq!(body["profile"], "macronode");
    assert_eq!(body["node_role"], "service_node");
    assert_eq!(body["node_profile"], "macronode");
    assert_eq!(body["content_serving_enabled"], true);
    assert_eq!(body["headless_mode"], true);
    assert_eq!(body["admin_ui_enabled"], false);
    assert_eq!(body["admin_ui_bind"], "127.0.0.1:5300");
    assert_eq!(body["operator_ui_profile"], "service_node_local");
    assert_eq!(body["admin_ui_runtime_required"], false);
    let capabilities = body["capabilities"]
        .as_array()
        .expect("capabilities array present");
    assert!(capabilities
        .iter()
        .any(|v| v.as_str() == Some("headless_operator_status_v1")));
    assert!(capabilities
        .iter()
        .any(|v| v.as_str() == Some("optional_admin_ui_v1")));
    assert!(capabilities
        .iter()
        .any(|v| v.as_str() == Some("oap_foundation_status_v1")));
    assert!(capabilities
        .iter()
        .any(|v| v.as_str() == Some("oap_object_fetch_v1")));
    assert!(capabilities
        .iter()
        .any(|v| v.as_str() == Some("provider_status_v1")));
    assert!(capabilities
        .iter()
        .any(|v| v.as_str() == Some("policy_status_v1")));
    assert!(capabilities
        .iter()
        .any(|v| v.as_str() == Some("signed_moderation_policy_v1")));
    assert!(capabilities
        .iter()
        .any(|v| v.as_str() == Some("reward_binding_status_v1")));
    assert!(capabilities
        .iter()
        .any(|v| v.as_str() == Some("service_evidence_outbox_v1")));

    // Phase 9 OAP runtime truth: the embedded storage listener now accepts
    // a bounded binary OBJ_GET request and emits a verified frame stream.
    assert_eq!(body["oap"]["protocol"], "oap/1");
    assert_eq!(body["oap"]["version"], 1);
    assert_eq!(body["oap"]["runtime_state"], "active_local_http_oap");
    assert_eq!(body["oap"]["max_frame_bytes"], 1_048_576);
    assert_eq!(body["oap"]["stream_chunk_bytes"], 65_536);
    assert_eq!(body["oap"]["object_fetch_active"], true);
    assert_eq!(body["oap"]["full_digest_verification_active"], true);

    // The embedded DHT router and local provider store are active, but
    // network advertisement and provider publication remain disabled.
    assert_eq!(
        body["provider"]["state"],
        "local_provider_store_active_not_advertising"
    );
    assert_eq!(
        body["provider"]["dht_worker_status"],
        "active_embedded_router"
    );
    assert_eq!(body["provider"]["advertisement_active"], false);
    assert_eq!(body["provider"]["provider_records_published"], 0);
    assert_eq!(
        body["provider"]["public_node_uri_format"],
        "crab://node/<node-id>"
    );
    assert_eq!(body["provider"]["residential_ip_publication"], false);

    // Legacy GET/HEAD and OAP OBJ_GET now share the same moderation
    // snapshot before storage access.
    assert_eq!(body["policy"]["state"], "all_object_read_policy_active");
    assert_eq!(body["policy"]["serve_policy_enforced"], true);
    assert_eq!(body["policy"]["oap_serve_policy_enforced"], true);
    assert_eq!(body["policy"]["operator_moderation_active"], false);
    assert_eq!(body["policy"]["global_moderation_active"], false);
    assert_eq!(body["policy"]["moderation_configured"], false);
    assert_eq!(body["policy"]["moderation_state"], "not_configured");
    assert_eq!(body["policy"]["moderation_source"], "none");
    assert_eq!(body["policy"]["moderation_load_failed"], false);
    assert_eq!(body["policy"]["signed_policy_verified"], false);
    assert!(body["policy"]["signed_policy_epoch"].is_null());
    assert!(body["policy"]["signed_policy_expires_at_unix_s"].is_null());
    assert_eq!(body["policy"]["rollback_guard_persisted"], false);
    assert_eq!(body["policy"]["moderation_activation"], "startup_snapshot");
    assert_eq!(body["policy"]["moderation_hot_reload"], false);
    assert_eq!(
        body["policy"]["unvetted_persistence_posture"],
        "amnesia_first"
    );
    assert_eq!(
        body["policy"]["serve_gate_phase"],
        "phase_10_all_object_reads_active"
    );
    assert_eq!(body["policy"]["moderation_phase"], "phase_10");

    // Reward binding starts unbound and remains explicitly non-economic.
    assert_eq!(body["reward_binding"]["state"], "unbound");
    assert!(body["reward_binding"]["reward_recipient_display_address"].is_null());
    assert!(body["reward_binding"]["pending_rotation_display_address"].is_null());
    assert_eq!(body["reward_binding"]["registry_finality"], false);
    assert_eq!(body["reward_binding"]["wallet_mutation"], false);
    assert_eq!(body["reward_binding"]["ledger_mutation"], false);
    assert!(body["reward_binding"]["confirmed_roc_minor_units"].is_null());

    // Phase 13 service evidence is a bounded process-local stream.
    // An empty outbox is truthful at startup and cannot imply reward,
    // accounting, payout, wallet, or ledger acceptance.
    assert_eq!(
        body["service_evidence"]["state"],
        "bounded_process_local_outbox"
    );
    assert_eq!(body["service_evidence"]["queued_records"], 0);
    assert_eq!(body["service_evidence"]["signature_required"], true);
    assert_eq!(
        body["service_evidence"]["replay_scope"],
        "bounded_process_local"
    );
    assert_eq!(body["service_evidence"]["durable"], false);
    assert_eq!(body["service_evidence"]["accounting_accepted"], false);
    assert_eq!(body["service_evidence"]["reward_eligible"], false);
    assert_eq!(body["service_evidence"]["reward_truth"], false);
    assert_eq!(body["service_evidence"]["payout_authority"], false);
    assert_eq!(body["service_evidence"]["wallet_mutation"], false);
    assert_eq!(body["service_evidence"]["ledger_mutation"], false);

    assert_eq!(body["service_quorum_enabled"], false);
    assert_eq!(body["wallet_execution_participant"], false);
    assert_eq!(body["user_ip_publication"], "not_applicable_service_node");
    assert!(body["uptime_seconds"].as_f64().unwrap_or(0.0) >= 0.0);
    // We expect a services map with at least gateway present.
    let services = body["services"].as_object().expect("services map present");
    assert_eq!(
        services.get("svc-gateway").and_then(Value::as_str),
        Some("ok")
    );
    assert_eq!(
        services.get("svc-storage").and_then(Value::as_str),
        Some("ok")
    );
    assert_eq!(
        services.get("svc-index").and_then(Value::as_str),
        Some("ok")
    );

    // Drive shutdown through the HTTP surface.
    shutdown_macronode(child, &client, &base).await?;

    Ok(())
}
