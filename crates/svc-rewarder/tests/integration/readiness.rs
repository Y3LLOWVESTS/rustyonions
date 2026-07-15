//! RO:WHAT — Rewarder readiness and Phase 23 dependency-unavailable chaos tests.
//! RO:WHY — Degraded dependencies must block new economic planning and wallet
//! egress while preserving truthful liveness and read-only inspection.
//! RO:INTERACTS — health state, readyz, compute, manifest inspection,
//! settlement preview, wallet emission, and canonical accounting input.
//! RO:INVARIANTS — auth remains first; failed readiness creates no new
//! manifest; read-only truth remains available; wallet egress fails closed.
//! RO:SECURITY — no fake receipt, balance, ledger root, payout, confirmed ROC,
//! or finality during dependency failure.
//! RO:TEST — cargo test -p svc-rewarder --test integration readiness.

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    response::Response,
};
use serde_json::{json, Value};
use svc_rewarder::{
    http::{routes::router, RewarderState},
    inputs::{canonical_snapshot_cid, AccountingSnapshot},
    Config,
};
use tower::ServiceExt;

fn snapshot_value() -> Value {
    json!({
        "produced_at_millis": 1,
        "pool_minor_units": "1000",
        "contributions": [
            {
                "account": "acct_b",
                "bytes_stored": 200,
                "bytes_served": 0,
                "uptime_seconds": 20
            },
            {
                "account": "acct_a",
                "bytes_stored": 100,
                "bytes_served": 50,
                "uptime_seconds": 10
            }
        ]
    })
}

fn compute_body(dry_run: bool) -> Value {
    let snapshot = snapshot_value();

    let accounting = serde_json::from_value::<AccountingSnapshot>(snapshot.clone())
        .expect("Phase 23 accounting snapshot must deserialize");

    let inputs_cid = canonical_snapshot_cid(accounting)
        .expect("Phase 23 accounting snapshot must produce a canonical CID");

    json!({
        "inputs_cid": inputs_cid,
        "policy_id": "policy:v1",
        "policy_hash": format!("b3:{}", "b".repeat(64)),
        "dry_run": dry_run,
        "snapshot": snapshot,
        "policy": {
            "id": "policy:v1",
            "hash": format!("b3:{}", "b".repeat(64)),
            "signed": true,
            "funding_source": "protocol_pool",
            "max_payout_minor_units": "1000",
            "min_payout_minor_units": "1",
            "weight_bps": 10000,
            "rounding": "floor"
        }
    })
}

async fn response_json(response: Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body must be readable");

    serde_json::from_slice(&body).expect("response body must contain valid JSON")
}

async fn response_text(response: Response) -> String {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body must be readable");

    String::from_utf8(body.to_vec()).expect("response body must contain UTF-8")
}

fn assert_no_authority_truth(body: &Value) {
    assert!(body.get("receipt").is_none());
    assert!(body.get("receipt_hash").is_none());
    assert!(body.get("wallet_receipt").is_none());
    assert!(body.get("balance").is_none());
    assert!(body.get("ledger_root").is_none());
    assert!(body.get("payout").is_none());
    assert!(body.get("confirmed_roc_minor_units").is_none());
    assert!(body.get("finality").is_none());
}

#[tokio::test]
async fn readyz_is_ok_after_state_initialization() {
    let state = RewarderState::new(Config::default()).expect("rewarder state");
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .expect("ready request"),
        )
        .await
        .expect("ready response");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;

    assert_eq!(body["config_loaded"], true);
    assert_eq!(body["ledger_ok"], true);
    assert_eq!(body["policy_registry_ok"], true);
    assert_eq!(body["queue_ok"], true);
}

#[tokio::test]
async fn readyz_degrades_when_queue_gate_false() {
    let state = RewarderState::new(Config::default()).expect("rewarder state");

    state.health.set(|snapshot| {
        snapshot.queue_ok = false;
    });

    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .expect("ready request"),
        )
        .await
        .expect("ready response");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE,);

    assert_eq!(
        response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok()),
        Some("1"),
    );

    let body = response_json(response).await;

    assert_eq!(body["degraded"], true);
    assert_eq!(body["missing"], json!(["queue_ok"]));
    assert_eq!(body["retry_after"], 1);
}

#[tokio::test]
async fn phase23_rewarder_unavailable_fails_closed_for_compute_and_emit() {
    let state = RewarderState::new(Config::default()).expect("rewarder state");

    let app = router(state.clone());

    // Establish one valid pre-degradation manifest. This proves read-only
    // inspection remains available after the dependency failure begins.
    let healthy_compute = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rewarder/epochs/phase23g-existing/compute")
                .header("authorization", "Bearer dev")
                .header("content-type", "application/json")
                .body(Body::from(compute_body(false).to_string()))
                .expect("healthy compute request"),
        )
        .await
        .expect("healthy compute response");

    assert_eq!(healthy_compute.status(), StatusCode::OK);

    let healthy_manifest = response_json(healthy_compute).await;

    assert_eq!(healthy_manifest["epoch_id"], "phase23g-existing",);

    // Model rewarder unavailability through the real shared readiness state.
    state.health.set(|snapshot| {
        snapshot.ledger_ok = false;
    });

    // Process liveness remains truthful even though economic work is blocked.
    let health = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .expect("health request"),
        )
        .await
        .expect("health response");

    assert_eq!(health.status(), StatusCode::OK);
    assert_eq!(response_text(health).await, "ok");

    // Readiness reports the exact unavailable dependency and retry posture.
    let ready = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .expect("ready request"),
        )
        .await
        .expect("ready response");

    assert_eq!(ready.status(), StatusCode::SERVICE_UNAVAILABLE,);

    assert_eq!(
        ready
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok()),
        Some("1"),
    );

    let ready_body = response_json(ready).await;

    assert_eq!(ready_body["degraded"], true);
    assert_eq!(ready_body["missing"], json!(["ledger_ok"]));
    assert_eq!(ready_body["retry_after"], 1);

    // New computation must fail before a new manifest can be inserted.
    let blocked_compute = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rewarder/epochs/phase23g-blocked/compute")
                .header("authorization", "Bearer dev")
                .header("content-type", "application/json")
                .body(Body::from(compute_body(false).to_string()))
                .expect("blocked compute request"),
        )
        .await
        .expect("blocked compute response");

    assert_eq!(blocked_compute.status(), StatusCode::SERVICE_UNAVAILABLE,);

    let blocked_compute_body = response_json(blocked_compute).await;

    assert_eq!(
        blocked_compute_body["error"]["code"],
        "DEPENDENCY_UNAVAILABLE",
    );

    assert_eq!(
        blocked_compute_body["error"]["details"]["reason"],
        "dependency",
    );

    assert!(blocked_compute_body["error"]["message"]
        .as_str()
        .expect("dependency message")
        .contains("ledger_ok"),);

    assert_no_authority_truth(&blocked_compute_body);

    let blocked_manifest = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/rewarder/epochs/phase23g-blocked")
                .header("authorization", "Bearer dev")
                .body(Body::empty())
                .expect("blocked manifest request"),
        )
        .await
        .expect("blocked manifest response");

    assert_eq!(blocked_manifest.status(), StatusCode::NOT_FOUND);

    // Existing planning truth remains inspectable during degradation.
    let existing_manifest = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/rewarder/epochs/phase23g-existing")
                .header("authorization", "Bearer dev")
                .body(Body::empty())
                .expect("existing manifest request"),
        )
        .await
        .expect("existing manifest response");

    assert_eq!(existing_manifest.status(), StatusCode::OK);

    let existing_manifest_body = response_json(existing_manifest).await;

    assert_eq!(existing_manifest_body["epoch_id"], "phase23g-existing",);

    // Settlement preview remains available because it is deterministic and
    // read-only; it performs no wallet or ledger mutation.
    let settlement_preview = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/rewarder/epochs/phase23g-existing/settlement")
                .header("authorization", "Bearer dev")
                .body(Body::empty())
                .expect("settlement preview request"),
        )
        .await
        .expect("settlement preview response");

    assert_eq!(settlement_preview.status(), StatusCode::OK);

    let settlement_preview_body = response_json(settlement_preview).await;

    assert!(
        !settlement_preview_body.is_null(),
        "read-only settlement preview must remain available",
    );

    // Wallet emission is an economic egress path and must stop before the
    // wallet client is created or contacted.
    let blocked_emit = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rewarder/epochs/phase23g-existing/emit")
                .header("authorization", "Bearer dev")
                .body(Body::empty())
                .expect("blocked emit request"),
        )
        .await
        .expect("blocked emit response");

    assert_eq!(blocked_emit.status(), StatusCode::SERVICE_UNAVAILABLE,);

    let blocked_emit_body = response_json(blocked_emit).await;

    assert_eq!(blocked_emit_body["error"]["code"], "DEPENDENCY_UNAVAILABLE",);

    assert_eq!(
        blocked_emit_body["error"]["details"]["reason"],
        "dependency",
    );

    assert!(blocked_emit_body["error"]["message"]
        .as_str()
        .expect("dependency message")
        .contains("ledger_ok"),);

    assert_no_authority_truth(&blocked_emit_body);

    println!(
        "Phase 23G passed: degraded rewarder readiness remained truthful, \
         liveness and read-only manifest/settlement inspection stayed \
         available, compute and wallet emission failed closed before \
         mutation or egress, no new manifest was recorded, and no receipt, \
         balance, ledger, payout, confirmed-ROC, or finality truth was \
         fabricated."
    );
}
