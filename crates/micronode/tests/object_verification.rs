//! RO:WHAT — Live HTTP tests for micronode deterministic object verification.
//! RO:WHY — Proves valid bytes and digest mismatches create bounded canonical pending evidence.
//! RO:INTERACTS — `/api/v1/verification/object`, pending queue, status projection, ron-proto evidence.
//! RO:INVARIANTS — replay/full queue reject; no IP, reward, wallet, ledger, receipt, or confirmed ROC.
//! RO:TEST — cargo test -p micronode --test object_verification.

use micronode::{
    app::build_router,
    config::schema::{Config, Server, UserNodeCfg},
    verification::{OBJECT_VERIFICATION_REQUEST_SCHEMA, OBJECT_VERIFICATION_REQUEST_VERSION},
};
use reqwest::{header::CONTENT_TYPE, StatusCode};
use serde_json::{json, Value};
use std::{net::SocketAddr, time::Duration};
use tokio::task::JoinHandle;

const ABC_CID: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

async fn spawn(user_node: UserNodeCfg) -> (SocketAddr, JoinHandle<()>) {
    let listener =
        tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind micronode verifier");

    let address = listener.local_addr().expect("micronode verifier address");

    let config = Config {
        server: Server { bind: address, dev_routes: false },
        user_node,
        ..Config::default()
    };

    let (router, state) = build_router(config);

    state.probes.set_cfg_loaded(true);
    state.probes.set_listeners_bound(true);
    state.probes.set_metrics_bound(true);
    state.probes.set_deps_ok(true);

    let handle = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, router).await {
            eprintln!("[micronode-object-verification-test] {error}");
        }
    });

    (address, handle)
}

fn request(bytes: &[u8], idempotency_key: &str) -> Value {
    json!({
        "schema":
            OBJECT_VERIFICATION_REQUEST_SCHEMA,

        "version":
            OBJECT_VERIFICATION_REQUEST_VERSION,

        "object": ABC_CID,
        "bytes": bytes,

        "observedAtMs":
            1_700_000_000_000_u64,

        "nonce":
            format!("{idempotency_key}-nonce"),

        "idempotencyKey":
            idempotency_key,

        "privacyRouteId":
            "relay:phase22e",
    })
}

async fn post_json(
    client: &reqwest::Client,
    url: &str,
    body: &Value,
) -> (StatusCode, Value, String) {
    let encoded = serde_json::to_vec(body).expect("encode verification body");

    let response = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .body(encoded)
        .send()
        .await
        .expect("POST verification body");

    let status = response.status();

    let text = response.text().await.expect("read verification body");

    let value = serde_json::from_str(&text).expect("verification JSON");

    (status, value, text)
}

#[tokio::test]
async fn valid_and_corrupt_objects_create_pending_evidence_without_authority() {
    let (address, _handle) =
        spawn(UserNodeCfg { pending_evidence_limit: 4, ..UserNodeCfg::default() }).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("verification client");

    let endpoint = format!("http://{address}/api/v1/verification/object");

    let (valid_status, valid, valid_text) =
        post_json(&client, &endpoint, &request(b"abc", "phase22e-valid")).await;

    assert_eq!(valid_status, StatusCode::ACCEPTED,);

    assert_eq!(valid["queueState"], "pending",);

    assert_eq!(valid["queueDepth"], 1);

    assert_eq!(valid["expectedObject"], ABC_CID,);

    assert_eq!(valid["calculatedObject"], ABC_CID,);

    assert_eq!(valid["fullDigestVerified"], true,);

    assert_eq!(valid["challengeRaised"], false,);

    assert_eq!(valid["pendingEvidence"], true,);

    assert_eq!(valid["evidence"]["result"], "verified_valid",);

    assert_eq!(valid["evidence"]["failure_reason"], Value::Null,);

    assert_eq!(valid["evidence"]["evidence_only"], true,);

    assert_eq!(valid["evidence"]["accounting_accepted"], false,);

    assert_eq!(valid["evidence"]["reward_eligible"], false,);

    assert_eq!(valid["evidence"]["reward_truth"], false,);

    assert_eq!(valid["evidence"]["payout_authority"], false,);

    assert_eq!(valid["evidence"]["wallet_mutation"], false,);

    assert_eq!(valid["evidence"]["ledger_mutation"], false,);

    assert_eq!(valid["confirmedRocMinorUnits"], Value::Null,);

    assert_eq!(valid["privacySafe"], true,);

    assert_eq!(valid["rawPeerIpRecorded"], false,);

    assert!(
        !valid_text.contains("127.0.0.1"),
        "verification response leaked loopback address: {valid_text}",
    );

    assert!(
        !valid_text.contains("socket"),
        "verification response leaked socket material: {valid_text}",
    );

    let (corrupt_status, corrupt, corrupt_text) =
        post_json(&client, &endpoint, &request(b"abd", "phase22e-corrupt")).await;

    assert_eq!(corrupt_status, StatusCode::ACCEPTED,);

    assert_eq!(corrupt["queueDepth"], 2,);

    assert_eq!(corrupt["fullDigestVerified"], false,);

    assert_eq!(corrupt["challengeRaised"], true,);

    assert_eq!(corrupt["evidence"]["result"], "challenge_raised",);

    assert_eq!(corrupt["evidence"]["failure_reason"], "digest_mismatch",);

    assert_ne!(corrupt["calculatedObject"], ABC_CID,);

    assert!(
        !corrupt_text.contains("127.0.0.1"),
        "challenge response leaked address material: {corrupt_text}",
    );

    let pending_response = client
        .get(format!("http://{address}/api/v1/verification/pending?limit=8"))
        .send()
        .await
        .expect("GET pending evidence");

    assert_eq!(pending_response.status(), StatusCode::OK,);

    let pending_text = pending_response.text().await.expect("read pending evidence");

    let pending: Value = serde_json::from_str(&pending_text).expect("pending evidence JSON");

    assert_eq!(pending["count"], 2);

    assert_eq!(pending["items"].as_array().expect("pending evidence items").len(), 2,);

    assert_eq!(pending["evidenceOnly"], true,);

    assert_eq!(pending["accountingAccepted"], false,);

    assert_eq!(pending["rewardEligible"], false,);

    assert_eq!(pending["rewardTruth"], false,);

    assert_eq!(pending["payoutAuthority"], false,);

    assert_eq!(pending["walletMutation"], false,);

    assert_eq!(pending["ledgerMutation"], false,);

    assert_eq!(pending["confirmedRocMinorUnits"], Value::Null,);

    let status_response = client
        .get(format!("http://{address}/api/v1/status"))
        .send()
        .await
        .expect("GET micronode status");

    let status_text = status_response.text().await.expect("read micronode status");

    let status: Value = serde_json::from_str(&status_text).expect("micronode status JSON");

    assert_eq!(status["passive_runtime"]["verification_queue"]["status"], "active",);

    assert_eq!(status["passive_runtime"]["verification_queue"]["pending_items"], 2,);

    assert_eq!(status["passive_runtime"]["confirmed_roc_minor_units"], Value::Null,);
}

#[tokio::test]
async fn verification_replay_and_queue_overflow_fail_closed() {
    let (address, _handle) =
        spawn(UserNodeCfg { pending_evidence_limit: 1, ..UserNodeCfg::default() }).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("verification client");

    let endpoint = format!("http://{address}/api/v1/verification/object");

    let first = request(b"abc", "phase22e-first");

    let (first_status, _, _) = post_json(&client, &endpoint, &first).await;

    assert_eq!(first_status, StatusCode::ACCEPTED,);

    let (duplicate_status, duplicate, _) = post_json(&client, &endpoint, &first).await;

    assert_eq!(duplicate_status, StatusCode::CONFLICT,);

    assert_eq!(duplicate["error"], "duplicate_idempotency_key",);

    assert_eq!(duplicate["duplicateRejected"], true,);

    assert_eq!(duplicate["queueMutation"], false,);

    let (full_status, full, _) =
        post_json(&client, &endpoint, &request(b"abd", "phase22e-second")).await;

    assert_eq!(full_status, StatusCode::TOO_MANY_REQUESTS,);

    assert_eq!(full["error"], "pending_queue_full",);

    assert_eq!(full["queueMutation"], false,);

    assert_eq!(full["walletMutation"], false,);

    assert_eq!(full["ledgerMutation"], false,);
}

#[tokio::test]
async fn paused_user_node_rejects_new_verification_and_preserves_read_only_truth() {
    let (address, _handle) =
        spawn(UserNodeCfg { pending_evidence_limit: 4, ..UserNodeCfg::default() }).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("verification client");

    let verification_endpoint = format!("http://{address}/api/v1/verification/object");

    let first_request = request(b"abc", "phase23b-existing");

    let (first_status, first, _) = post_json(&client, &verification_endpoint, &first_request).await;

    assert_eq!(first_status, StatusCode::ACCEPTED);
    assert_eq!(first["queueDepth"], 1);

    let pause_response = client
        .post(format!("http://{address}/api/v1/verification/pause"))
        .send()
        .await
        .expect("pause User Node verification");

    assert_eq!(pause_response.status(), StatusCode::OK);

    let pause_text = pause_response.text().await.expect("read pause response");

    let pause: Value = serde_json::from_str(&pause_text).expect("pause response JSON");

    assert_eq!(pause["schema"], "micronode.verification_control.v1");
    assert_eq!(pause["status"], "verification paused");
    assert_eq!(pause["changed"], true);
    assert_eq!(pause["lifecycleState"], "paused");
    assert_eq!(pause["verificationQueueStatus"], "paused");
    assert_eq!(pause["pendingItems"], 1);
    assert_eq!(pause["readOnlyStatusAvailable"], true);
    assert_eq!(pause["newVerificationWritesEnabled"], false);
    assert_eq!(pause["queueMutation"], false);
    assert_eq!(pause["accountingAccepted"], false);
    assert_eq!(pause["rewardEligible"], false);
    assert_eq!(pause["rewardTruth"], false);
    assert_eq!(pause["payoutAuthority"], false);
    assert_eq!(pause["walletMutation"], false);
    assert_eq!(pause["ledgerMutation"], false);
    assert_eq!(pause["confirmedRocMinorUnits"], Value::Null);

    let status_response = client
        .get(format!("http://{address}/api/v1/status"))
        .send()
        .await
        .expect("GET paused User Node status");

    assert_eq!(status_response.status(), StatusCode::OK);

    let status_text = status_response.text().await.expect("read paused status");

    let status: Value = serde_json::from_str(&status_text).expect("paused status JSON");

    assert_eq!(status["verification_enabled"], true);
    assert_eq!(status["passive_runtime"]["lifecycle_state"], "paused");
    assert_eq!(status["passive_runtime"]["verification_queue"]["status"], "paused");
    assert_eq!(status["passive_runtime"]["verification_queue"]["pending_items"], 1);
    assert_eq!(status["passive_runtime"]["economic_replay_worker"]["status"], "paused");
    assert_eq!(status["passive_runtime"]["confirmed_roc_minor_units"], Value::Null);
    assert_eq!(status["passive_runtime"]["wallet_mutation"], false);
    assert_eq!(status["passive_runtime"]["ledger_mutation"], false);

    let paused_request = request(b"abd", "phase23b-paused");

    let (paused_status, paused, _) =
        post_json(&client, &verification_endpoint, &paused_request).await;

    assert_eq!(paused_status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(paused["error"], "verification_paused");
    assert_eq!(paused["duplicateRejected"], false);
    assert_eq!(paused["queueMutation"], false);
    assert_eq!(paused["accountingAccepted"], false);
    assert_eq!(paused["rewardEligible"], false);
    assert_eq!(paused["rewardTruth"], false);
    assert_eq!(paused["payoutAuthority"], false);
    assert_eq!(paused["walletMutation"], false);
    assert_eq!(paused["ledgerMutation"], false);
    assert_eq!(paused["confirmedRocMinorUnits"], Value::Null);

    let pending_response = client
        .get(format!("http://{address}/api/v1/verification/pending?limit=4"))
        .send()
        .await
        .expect("GET pending evidence while paused");

    assert_eq!(pending_response.status(), StatusCode::OK);

    let pending_text = pending_response.text().await.expect("read pending evidence while paused");

    let pending: Value = serde_json::from_str(&pending_text).expect("pending JSON");

    assert_eq!(pending["count"], 1);
    assert_eq!(pending["evidenceOnly"], true);
    assert_eq!(pending["accountingAccepted"], false);
    assert_eq!(pending["rewardEligible"], false);
    assert_eq!(pending["walletMutation"], false);
    assert_eq!(pending["ledgerMutation"], false);
    assert_eq!(pending["confirmedRocMinorUnits"], Value::Null);

    let resume_response = client
        .post(format!("http://{address}/api/v1/verification/resume"))
        .send()
        .await
        .expect("resume User Node verification");

    assert_eq!(resume_response.status(), StatusCode::OK);

    let resume_text = resume_response.text().await.expect("read resume response");

    let resume: Value = serde_json::from_str(&resume_text).expect("resume response JSON");

    assert_eq!(resume["status"], "verification resumed");
    assert_eq!(resume["changed"], true);
    assert_eq!(resume["lifecycleState"], "active");
    assert_eq!(resume["verificationQueueStatus"], "active");
    assert_eq!(resume["pendingItems"], 1);
    assert_eq!(resume["readOnlyStatusAvailable"], true);
    assert_eq!(resume["newVerificationWritesEnabled"], true);
    assert_eq!(resume["queueMutation"], false);

    let (resumed_status, resumed, _) =
        post_json(&client, &verification_endpoint, &paused_request).await;

    assert_eq!(resumed_status, StatusCode::ACCEPTED);
    assert_eq!(resumed["queueDepth"], 2);
    assert_eq!(resumed["challengeRaised"], true);

    println!(
        "Phase 23B passed: the User Node paused truthfully, rejected new \
         verification writes without consuming replay identity, retained \
         read-only pending evidence, resumed safely, and created no \
         accounting, reward, wallet, ledger, receipt, or confirmed-ROC \
         authority."
    );
}

#[tokio::test]
async fn disabled_verification_queue_rejects_work_without_mutation() {
    let (address, _handle) =
        spawn(UserNodeCfg { verification_queue_enabled: false, ..UserNodeCfg::default() }).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("verification client");

    let (status, body, _) = post_json(
        &client,
        &format!("http://{address}/api/v1/verification/object"),
        &request(b"abc", "phase22e-disabled"),
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE,);

    assert_eq!(body["error"], "verification_disabled",);

    assert_eq!(body["queueMutation"], false,);

    let status_response = client
        .get(format!("http://{address}/api/v1/status"))
        .send()
        .await
        .expect("GET disabled status");

    let status_text = status_response.text().await.expect("read disabled status");

    let status_body: Value = serde_json::from_str(&status_text).expect("disabled status JSON");

    assert_eq!(status_body["verification_enabled"], false,);

    assert_eq!(status_body["passive_runtime"]["verification_queue"]["status"], "disabled",);

    assert_eq!(status_body["passive_runtime"]["verification_queue"]["pending_items"], 0,);
}
