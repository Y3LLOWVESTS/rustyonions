//! RO:WHAT — Request-poisoning matrix for svc-wallet QuickChain preflight.
//! RO:WHY — QuickChain preflight may expose inert review vocabulary, but live
//! wallet mutation requests must reject client-supplied chain authority fields.
//! RO:INTERACTS — /v1/issue, /v1/transfer, /v1/burn, /v1/hold, /v1/capture,
//! /v1/release, WalletState::dev, strict request DTOs.
//! RO:INVARIANTS — schema/chain_id/operation_id/root/checkpoint/anchor/
//! validator/finality fields cannot be smuggled into wallet mutation bodies;
//! rejected poisoned requests do not consume idempotency keys or nonces.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — prevents client request bodies from becoming spend, unlock,
//! settlement, finality, root, bridge, validator, or chain authority.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_request_poisoning_matrix.

#![cfg(feature = "quickchain-preflight")]

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use svc_wallet::{
    dto::responses::Receipt,
    routes::{self, WalletState},
};
use tower::ServiceExt;

fn app() -> Router {
    routes::router(WalletState::dev().expect("dev wallet state should build"))
}

fn post_request(path: &str, idempotency_key: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::AUTHORIZATION, "Bearer dev")
        .header(header::CONTENT_TYPE, "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(
            serde_json::to_vec(&body).expect("JSON request body should encode"),
        ))
        .expect("POST request should build")
}

async fn send(router: Router, request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = router
        .oneshot(request)
        .await
        .expect("router request should complete");

    let status = response.status();
    let body = to_bytes(response.into_body(), 1_048_576)
        .await
        .expect("response body should read")
        .to_vec();

    (status, body)
}

async fn post_json(
    router: Router,
    path: &str,
    idempotency_key: &str,
    body: Value,
) -> (StatusCode, Vec<u8>) {
    send(router, post_request(path, idempotency_key, body)).await
}

fn add_quickchain_authority_poison(mut body: Value) -> Value {
    let object = body
        .as_object_mut()
        .expect("test request body should be a JSON object");

    object.insert(
        "schema".to_string(),
        json!("svc-wallet.quickchain-receipt-projection.v1"),
    );
    object.insert("chain_id".to_string(), json!("roc-dev"));
    object.insert(
        "operation_id".to_string(),
        json!("op:attacker:client-supplied-authority"),
    );
    object.insert(
        "receipt_root".to_string(),
        json!("b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    );
    object.insert(
        "state_root".to_string(),
        json!("b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
    );
    object.insert(
        "checkpoint".to_string(),
        json!("b3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"),
    );
    object.insert(
        "anchor".to_string(),
        json!("external-anchor-must-not-authorize"),
    );
    object.insert("validator".to_string(), json!("validator_attacker"));
    object.insert("finalized".to_string(), json!(true));
    object.insert("finality".to_string(), json!("anchored"));
    object.insert(
        "settlement_root".to_string(),
        json!("b3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"),
    );
    object.insert("settlement_status".to_string(), json!("finalized"));

    body
}

fn parse_receipt(bytes: &[u8]) -> Receipt {
    serde_json::from_slice(bytes).expect("response should deserialize as wallet Receipt")
}

fn assert_poison_rejected_without_receipt(route: &str, status: StatusCode, bytes: &[u8]) {
    assert!(
        status.is_client_error(),
        "poisoned {route} request must be rejected as a client error, got {status}"
    );
    assert_ne!(
        status,
        StatusCode::OK,
        "poisoned {route} request must not succeed"
    );
    assert!(
        serde_json::from_slice::<Receipt>(bytes).is_err(),
        "poisoned {route} request must not return a wallet receipt"
    );
}

fn assert_clean_wallet_receipt(
    route: &str,
    status: StatusCode,
    bytes: &[u8],
    expected_op: &str,
) -> Receipt {
    assert_eq!(
        status,
        StatusCode::OK,
        "clean {route} request should still succeed after poisoned rejection"
    );

    let receipt = parse_receipt(bytes);
    assert_eq!(receipt.op.as_str(), expected_op);
    assert_eq!(receipt.settlement_status.as_str(), "accepted");
    assert!(
        receipt.txid.starts_with("tx_"),
        "clean {route} receipt should retain wallet txid vocabulary"
    );
    assert!(
        receipt.receipt_hash.starts_with("b3:"),
        "clean {route} receipt should carry backend-derived b3 receipt hash"
    );

    receipt
}

async fn post_clean_receipt(
    router: Router,
    path: &str,
    idempotency_key: &str,
    body: Value,
    expected_op: &str,
) -> Receipt {
    let (status, bytes) = post_json(router, path, idempotency_key, body).await;
    assert_clean_wallet_receipt(path, status, &bytes, expected_op)
}

async fn assert_poison_then_clean_succeeds(
    router: Router,
    path: &str,
    idempotency_key: &str,
    clean_body: Value,
    expected_op: &str,
) -> Receipt {
    let poisoned_body = add_quickchain_authority_poison(clean_body.clone());

    let (poison_status, poison_bytes) =
        post_json(router.clone(), path, idempotency_key, poisoned_body).await;

    assert_poison_rejected_without_receipt(path, poison_status, &poison_bytes);

    // Reuse the same idempotency key and nonce after the poisoned request. If
    // the poisoned body consumed wallet retry identity or nonce state, this
    // clean request would replay, conflict, or fail nonce validation.
    let (clean_status, clean_bytes) = post_json(router, path, idempotency_key, clean_body).await;

    assert_clean_wallet_receipt(path, clean_status, &clean_bytes, expected_op)
}

#[tokio::test]
async fn issue_transfer_and_burn_reject_quickchain_authority_fields_without_consuming_identity() {
    let router = app();

    assert_poison_then_clean_succeeds(
        router.clone(),
        "/v1/issue",
        "idem_qc_poison_issue",
        json!({
            "to": "acct_qc_poison_issue",
            "asset": "roc",
            "amount_minor": "13",
            "memo": null
        }),
        "issue",
    )
    .await;

    post_clean_receipt(
        router.clone(),
        "/v1/issue",
        "idem_qc_poison_transfer_seed",
        json!({
            "to": "acct_qc_poison_transfer_source",
            "asset": "roc",
            "amount_minor": "100",
            "memo": null
        }),
        "issue",
    )
    .await;

    assert_poison_then_clean_succeeds(
        router.clone(),
        "/v1/transfer",
        "idem_qc_poison_transfer",
        json!({
            "from": "acct_qc_poison_transfer_source",
            "to": "acct_qc_poison_transfer_dest",
            "asset": "roc",
            "amount_minor": "25",
            "nonce": 1,
            "memo": null
        }),
        "transfer",
    )
    .await;

    post_clean_receipt(
        router.clone(),
        "/v1/issue",
        "idem_qc_poison_burn_seed",
        json!({
            "to": "acct_qc_poison_burn_source",
            "asset": "roc",
            "amount_minor": "50",
            "memo": null
        }),
        "issue",
    )
    .await;

    assert_poison_then_clean_succeeds(
        router,
        "/v1/burn",
        "idem_qc_poison_burn",
        json!({
            "from": "acct_qc_poison_burn_source",
            "asset": "roc",
            "amount_minor": "5",
            "nonce": 1,
            "memo": null
        }),
        "burn",
    )
    .await;
}

#[tokio::test]
async fn hold_capture_and_release_reject_quickchain_authority_fields_without_consuming_identity() {
    let router = app();

    post_clean_receipt(
        router.clone(),
        "/v1/issue",
        "idem_qc_poison_hold_seed",
        json!({
            "to": "acct_qc_poison_hold_user",
            "asset": "roc",
            "amount_minor": "100",
            "memo": null
        }),
        "issue",
    )
    .await;

    assert_poison_then_clean_succeeds(
        router.clone(),
        "/v1/hold",
        "idem_qc_poison_hold",
        json!({
            "from": "acct_qc_poison_hold_user",
            "to": "escrow_qc_poison_hold_1",
            "asset": "roc",
            "amount_minor": "70",
            "nonce": 1,
            "memo": "storage hold"
        }),
        "hold",
    )
    .await;

    post_clean_receipt(
        router.clone(),
        "/v1/issue",
        "idem_qc_poison_capture_seed",
        json!({
            "to": "acct_qc_poison_capture_user",
            "asset": "roc",
            "amount_minor": "100",
            "memo": null
        }),
        "issue",
    )
    .await;

    post_clean_receipt(
        router.clone(),
        "/v1/hold",
        "idem_qc_poison_capture_hold_seed",
        json!({
            "from": "acct_qc_poison_capture_user",
            "to": "escrow_qc_poison_capture_1",
            "asset": "roc",
            "amount_minor": "70",
            "nonce": 1,
            "memo": "storage hold"
        }),
        "hold",
    )
    .await;

    assert_poison_then_clean_succeeds(
        router.clone(),
        "/v1/capture",
        "idem_qc_poison_capture",
        json!({
            "from": "escrow_qc_poison_capture_1",
            "to": "svc_storage",
            "asset": "roc",
            "amount_minor": "40",
            "nonce": 1,
            "memo": "storage capture"
        }),
        "capture",
    )
    .await;

    post_clean_receipt(
        router.clone(),
        "/v1/issue",
        "idem_qc_poison_release_seed",
        json!({
            "to": "acct_qc_poison_release_user",
            "asset": "roc",
            "amount_minor": "100",
            "memo": null
        }),
        "issue",
    )
    .await;

    post_clean_receipt(
        router.clone(),
        "/v1/hold",
        "idem_qc_poison_release_hold_seed",
        json!({
            "from": "acct_qc_poison_release_user",
            "to": "escrow_qc_poison_release_1",
            "asset": "roc",
            "amount_minor": "70",
            "nonce": 1,
            "memo": "storage hold"
        }),
        "hold",
    )
    .await;

    assert_poison_then_clean_succeeds(
        router,
        "/v1/release",
        "idem_qc_poison_release",
        json!({
            "from": "escrow_qc_poison_release_1",
            "to": "acct_qc_poison_release_user",
            "asset": "roc",
            "amount_minor": "30",
            "nonce": 1,
            "memo": "storage release"
        }),
        "release",
    )
    .await;
}
