//! RO:WHAT — Matrix tests proving live svc-wallet routes remain wallet receipts
//! under the quickchain-preflight feature.
//! RO:WHY — The feature may expose inert projection helpers, but every HTTP
//! mutation/read route must remain wallet/ledger authority only.
//! RO:INTERACTS — /v1/issue, /v1/transfer, /v1/burn, /v1/hold, /v1/capture,
//! /v1/release, /v1/tx/{txid}, WalletState::dev.
//! RO:INVARIANTS — no schema, chain_id, operation_id, idempotency_key,
//! produced_at_ms, legacy_ledger_root, checkpoint, anchor, validator, or root
//! fields leak into live HTTP receipts.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — prevents feature-gated QuickChain review vocabulary from
//! becoming spend, settlement, finality, unlock, or chain authority.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_live_route_matrix.

#![cfg(feature = "quickchain-preflight")]

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use svc_wallet::routes::{self, WalletState};
use tower::ServiceExt;

fn app() -> Router {
    routes::router(WalletState::dev().expect("dev wallet state should build"))
}

fn post_request(path: &str, idem: &str, body: Value) -> Request<Body> {
    let encoded = serde_json::to_vec(&body).expect("JSON body should encode");

    Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::AUTHORIZATION, "Bearer dev")
        .header(header::CONTENT_TYPE, "application/json")
        .header("Idempotency-Key", idem)
        .body(Body::from(encoded))
        .expect("POST request should build")
}

fn get_request(path: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(path)
        .header(header::AUTHORIZATION, "Bearer dev")
        .body(Body::empty())
        .expect("GET request should build")
}

async fn send(router: Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router
        .oneshot(request)
        .await
        .expect("router request should complete");

    let status = response.status();
    let body = to_bytes(response.into_body(), 1_048_576)
        .await
        .expect("response body should read");

    let value = serde_json::from_slice::<Value>(&body).expect("response should be JSON");
    (status, value)
}

async fn post_json(router: Router, path: &str, idem: &str, body: Value) -> (StatusCode, Value) {
    send(router, post_request(path, idem, body)).await
}

async fn get_json(router: Router, path: &str) -> (StatusCode, Value) {
    send(router, get_request(path)).await
}

fn assert_live_wallet_receipt(value: &Value, expected_op: &str) {
    let object = value
        .as_object()
        .expect("live wallet receipt should be a JSON object");

    assert_eq!(value["op"], expected_op);
    assert_eq!(value["settlement_status"], "accepted");

    assert!(
        value["txid"]
            .as_str()
            .expect("txid should be string")
            .starts_with("tx_"),
        "wallet receipt should carry wallet txid"
    );

    assert!(
        value["receipt_hash"]
            .as_str()
            .expect("receipt_hash should be string")
            .starts_with("b3:"),
        "wallet receipt should carry b3 receipt hash"
    );

    for forbidden in [
        "schema",
        "chain_id",
        "operation_id",
        "idempotency_key",
        "produced_at_ms",
        "legacy_ledger_root",
        "state_root",
        "receipt_root",
        "checkpoint",
        "anchor",
        "validator",
        "finalized",
        "finality",
        "settlement_root",
    ] {
        assert!(
            !object.contains_key(forbidden),
            "live wallet route must not expose QuickChain projection/settlement field {forbidden}"
        );
    }
}

#[tokio::test]
async fn all_live_mutation_routes_remain_wallet_receipts_under_quickchain_preflight() {
    let router = app();

    let (status, issue_for_transfer) = post_json(
        router.clone(),
        "/v1/issue",
        "idem_qc_matrix_issue_transfer_source",
        json!({
            "to": "acct_qc_matrix_transfer_source",
            "asset": "roc",
            "amount_minor": "100",
            "memo": null
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_live_wallet_receipt(&issue_for_transfer, "issue");

    let (status, transfer) = post_json(
        router.clone(),
        "/v1/transfer",
        "idem_qc_matrix_transfer",
        json!({
            "from": "acct_qc_matrix_transfer_source",
            "to": "acct_qc_matrix_transfer_dest",
            "asset": "roc",
            "amount_minor": "25",
            "nonce": 1,
            "memo": null
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_live_wallet_receipt(&transfer, "transfer");

    let (status, burn) = post_json(
        router.clone(),
        "/v1/burn",
        "idem_qc_matrix_burn",
        json!({
            "from": "acct_qc_matrix_transfer_source",
            "asset": "roc",
            "amount_minor": "5",
            "nonce": 2,
            "memo": null
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_live_wallet_receipt(&burn, "burn");

    let (status, issue_for_escrow) = post_json(
        router.clone(),
        "/v1/issue",
        "idem_qc_matrix_issue_escrow_user",
        json!({
            "to": "acct_qc_matrix_escrow_user",
            "asset": "roc",
            "amount_minor": "100",
            "memo": null
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_live_wallet_receipt(&issue_for_escrow, "issue");

    let (status, hold) = post_json(
        router.clone(),
        "/v1/hold",
        "idem_qc_matrix_hold",
        json!({
            "from": "acct_qc_matrix_escrow_user",
            "to": "escrow_qc_matrix_hold_1",
            "asset": "roc",
            "amount_minor": "70",
            "nonce": 1,
            "memo": "storage hold"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_live_wallet_receipt(&hold, "hold");

    let (status, capture) = post_json(
        router.clone(),
        "/v1/capture",
        "idem_qc_matrix_capture",
        json!({
            "from": "escrow_qc_matrix_hold_1",
            "to": "svc_storage",
            "asset": "roc",
            "amount_minor": "40",
            "nonce": 1,
            "memo": "storage capture"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_live_wallet_receipt(&capture, "capture");

    let (status, release) = post_json(
        router.clone(),
        "/v1/release",
        "idem_qc_matrix_release",
        json!({
            "from": "escrow_qc_matrix_hold_1",
            "to": "acct_qc_matrix_escrow_user",
            "asset": "roc",
            "amount_minor": "30",
            "nonce": 2,
            "memo": "storage release"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_live_wallet_receipt(&release, "release");
}

#[tokio::test]
async fn receipt_lookup_route_remains_wallet_receipt_not_quickchain_projection() {
    let router = app();

    let (status, issued) = post_json(
        router.clone(),
        "/v1/issue",
        "idem_qc_matrix_receipt_lookup_issue",
        json!({
            "to": "acct_qc_matrix_receipt_lookup",
            "asset": "roc",
            "amount_minor": "11",
            "memo": null
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_live_wallet_receipt(&issued, "issue");

    let txid = issued["txid"]
        .as_str()
        .expect("issue receipt should include txid");

    let (status, looked_up) = get_json(router, &format!("/v1/tx/{txid}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(looked_up, issued);
    assert_live_wallet_receipt(&looked_up, "issue");
}

#[tokio::test]
async fn idempotent_replay_remains_same_wallet_receipt_under_quickchain_preflight() {
    let router = app();

    let body = json!({
        "to": "acct_qc_matrix_replay",
        "asset": "roc",
        "amount_minor": "19",
        "memo": null
    });

    let (first_status, first) = post_json(
        router.clone(),
        "/v1/issue",
        "idem_qc_matrix_idempotent_replay",
        body.clone(),
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);
    assert_live_wallet_receipt(&first, "issue");

    let (second_status, second) = post_json(
        router,
        "/v1/issue",
        "idem_qc_matrix_idempotent_replay",
        body,
    )
    .await;
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(
        second, first,
        "idempotent replay must return the exact same wallet receipt JSON"
    );
    assert_live_wallet_receipt(&second, "issue");
}
