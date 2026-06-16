use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use svc_rewarder::http::routes::router;
use svc_rewarder::http::RewarderState;
use svc_rewarder::inputs::{canonical_snapshot_cid, AccountingSnapshot};
use svc_rewarder::Config;
use tower::ServiceExt;

fn snapshot_value() -> Value {
    json!({
        "produced_at_millis": 1,
        "pool_minor_units": "1000",
        "contributions": [
            {"account":"acct_b","bytes_stored":200,"bytes_served":0,"uptime_seconds":20},
            {"account":"acct_a","bytes_stored":100,"bytes_served":50,"uptime_seconds":10}
        ]
    })
}

fn inputs_cid_for_snapshot_value(snapshot: &Value) -> String {
    let snapshot = serde_json::from_value::<AccountingSnapshot>(snapshot.clone())
        .expect("test snapshot should deserialize");
    canonical_snapshot_cid(snapshot).expect("test snapshot should produce canonical cid")
}

fn compute_body(dry_run: bool) -> Value {
    let snapshot = snapshot_value();
    let inputs_cid = inputs_cid_for_snapshot_value(&snapshot);

    json!({
        "inputs_cid": inputs_cid,
        "policy_id": "policy:v1",
        "policy_hash": format!("b3:{}", "b".repeat(64)),
        "dry_run": dry_run,
        "snapshot": snapshot,
        "policy": {
            "id":"policy:v1",
            "hash": format!("b3:{}", "b".repeat(64)),
            "signed": true,
            "funding_source": "protocol_pool",
            "max_payout_minor_units":"1000",
            "min_payout_minor_units":"1",
            "weight_bps":10000,
            "rounding":"floor"
        }
    })
}

fn compute_body_with_bad_inputs_cid(dry_run: bool) -> Value {
    let mut body = compute_body(dry_run);
    body["inputs_cid"] = json!(format!("b3:{}", "c".repeat(64)));
    body
}

async fn post_compute(app: axum::Router, epoch_id: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(format!("/rewarder/epochs/{epoch_id}/compute"))
        .header("authorization", "Bearer dev")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice::<Value>(&bytes).unwrap();
    (status, value)
}

#[tokio::test]
async fn compute_happy_path_and_replay_are_deterministic() {
    let state = RewarderState::new(Config::default()).unwrap();
    let app = router(state);
    let body = compute_body(true);

    let req = Request::builder()
        .method("POST")
        .uri("/rewarder/epochs/epoch-1/compute")
        .header("authorization", "Bearer dev")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let first = app.clone().oneshot(req).await.unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = to_bytes(first.into_body(), usize::MAX).await.unwrap();

    let req2 = Request::builder()
        .method("POST")
        .uri("/rewarder/epochs/epoch-1/compute")
        .header("authorization", "Bearer dev")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let second = app.oneshot(req2).await.unwrap();
    assert_eq!(second.status(), StatusCode::OK);
    let second_body = to_bytes(second.into_body(), usize::MAX).await.unwrap();

    assert_eq!(first_body, second_body);
}

#[tokio::test]
async fn compute_rejects_inputs_cid_that_does_not_match_snapshot() {
    let state = RewarderState::new(Config::default()).unwrap();
    let app = router(state);

    let (status, value) = post_compute(
        app,
        "epoch-bad-input-cid-1",
        compute_body_with_bad_inputs_cid(true),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(value["error"]["code"], "BAD_REQUEST");
    assert_eq!(value["error"]["details"]["reason"], "bad_request");
    assert!(value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("inputs_cid mismatch"));
}

#[tokio::test]
async fn settlement_preview_endpoint_returns_wallet_issue_batch() {
    let state = RewarderState::new(Config::default()).unwrap();
    let app = router(state);
    let body = compute_body(false);

    let (status, manifest) = post_compute(app.clone(), "epoch-settle-1", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(manifest["ledger"]["result"], "accepted");

    let req = Request::builder()
        .method("GET")
        .uri("/rewarder/epochs/epoch-settle-1/settlement")
        .header("authorization", "Bearer dev")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice::<Value>(&bytes).unwrap();

    assert_eq!(value["run_key"], manifest["run_key"]);
    assert_eq!(value["epoch_id"], "epoch-settle-1");
    assert_eq!(value["manifest_commitment"], manifest["commitment"]);
    assert_eq!(value["wallet_path"], "/v1/issue");
    assert_eq!(value["funding_source"], "protocol_pool");

    let requests = value["requests"].as_array().unwrap();
    assert!(!requests.is_empty());

    for req in requests {
        assert_eq!(req["asset"], "roc");
        assert!(req.get("funding_source").is_none());
        assert!(
            req["amount_minor"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                > 0
        );
        assert!(req["idempotency_key"].as_str().unwrap().starts_with("b3:"));
        assert!(req["idempotency_key"].as_str().unwrap().len() <= 64);
        assert!(req["memo"]
            .as_str()
            .unwrap()
            .starts_with("svc-rewarder:epoch-settle-1:"));
    }
}

#[tokio::test]
async fn dry_run_can_promote_to_production_without_consuming_run_key() {
    let state = RewarderState::new(Config::default()).unwrap();
    let app = router(state);

    let (dry_status, dry_manifest) =
        post_compute(app.clone(), "epoch-promote-1", compute_body(true)).await;
    assert_eq!(dry_status, StatusCode::OK);
    assert_eq!(dry_manifest["ledger"]["result"], "dry_run");

    let (live_status, live_manifest) =
        post_compute(app.clone(), "epoch-promote-1", compute_body(false)).await;
    assert_eq!(live_status, StatusCode::OK);
    assert_eq!(live_manifest["ledger"]["result"], "accepted");

    assert_eq!(dry_manifest["run_key"], live_manifest["run_key"]);
    assert_ne!(dry_manifest["commitment"], live_manifest["commitment"]);

    let req = Request::builder()
        .method("GET")
        .uri("/rewarder/epochs/epoch-promote-1/settlement")
        .header("authorization", "Bearer dev")
        .body(Body::empty())
        .unwrap();

    let settlement = app.oneshot(req).await.unwrap();
    assert_eq!(settlement.status(), StatusCode::OK);
}

#[tokio::test]
async fn metrics_include_planned_settlement_intents_after_compute() {
    let state = RewarderState::new(Config::default()).unwrap();
    let app = router(state);

    let (status, _manifest) =
        post_compute(app.clone(), "epoch-metrics-1", compute_body(false)).await;
    assert_eq!(status, StatusCode::OK);

    let req = Request::builder()
        .method("GET")
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let text = String::from_utf8(
        to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();

    assert!(text.contains("svc_rewarder_settlement_intents_planned_total"));
    assert!(text.contains("svc_rewarder_ledger_intents_total"));
}
