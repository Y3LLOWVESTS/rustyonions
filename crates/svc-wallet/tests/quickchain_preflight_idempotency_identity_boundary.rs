//! RO:WHAT — Tests that wallet idempotency remains retry identity, not
//! QuickChain operation authority.
//! RO:WHY — QuickChain uses backend-assigned durable operation_id; svc-wallet
//! must not promote client retry keys into chain authority.
//! RO:INTERACTS — HTTP issue route, idempotency store, Receipt DTO, and
//! quickchain preflight projection helper.
//! RO:INVARIANTS — same idempotent request replays byte-identical receipt;
//! different request conflicts; operation_id is explicit projection context only.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — prevents client-controlled idempotency keys from becoming
//! spend, finality, settlement, unlock, root, or chain authority.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_idempotency_identity_boundary.

#![cfg(feature = "quickchain-preflight")]

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use svc_wallet::{
    dto::responses::Receipt,
    quickchain::{
        project_wallet_receipt_for_quickchain_preflight, QuickChainWalletReceiptProjectionContext,
    },
    routes::{self, WalletState},
};
use tower::ServiceExt;

fn app() -> Router {
    routes::router(WalletState::dev().expect("dev wallet state should build"))
}

fn json_post_request(path: &str, idempotency_key: &str, body: Value) -> Request<Body> {
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

fn parse_receipt(bytes: &[u8]) -> Receipt {
    serde_json::from_slice(bytes).expect("response should deserialize as wallet Receipt")
}

fn parse_json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("response should deserialize as JSON")
}

#[tokio::test]
async fn same_idempotency_key_replays_same_receipt_without_becoming_operation_id() {
    let router = app();

    let body = json!({
        "to": "acct_qc_idem_identity_alice",
        "asset": "roc",
        "amount_minor": "123",
        "memo": null
    });

    let (first_status, first_bytes) = send(
        router.clone(),
        json_post_request(
            "/v1/issue",
            "idem_qc_retry_identity_not_authority",
            body.clone(),
        ),
    )
    .await;

    let (second_status, second_bytes) = send(
        router,
        json_post_request("/v1/issue", "idem_qc_retry_identity_not_authority", body),
    )
    .await;

    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(
        first_bytes, second_bytes,
        "idempotent replay must return byte-identical wallet receipt"
    );

    let first_receipt = parse_receipt(&first_bytes);
    let second_receipt = parse_receipt(&second_bytes);

    assert_eq!(first_receipt, second_receipt);
    assert_eq!(first_receipt.idem, "idem_qc_retry_identity_not_authority");

    let operation_context = QuickChainWalletReceiptProjectionContext::accepted(
        "roc-dev",
        "op:wallet:backend-assigned:0001",
    )
    .expect("explicit backend operation context should validate");

    let first_projection =
        project_wallet_receipt_for_quickchain_preflight(&first_receipt, &operation_context)
            .expect("first projection should succeed");

    let second_projection =
        project_wallet_receipt_for_quickchain_preflight(&second_receipt, &operation_context)
            .expect("second projection should succeed");

    assert_eq!(
        first_projection, second_projection,
        "projecting a replayed receipt with the same explicit operation context should be stable"
    );

    assert_eq!(
        first_projection.idempotency_key.as_str(),
        first_receipt.idem.as_str()
    );
    assert_eq!(
        first_projection.operation_id.as_str(),
        "op:wallet:backend-assigned:0001"
    );
    assert_ne!(
        first_projection.operation_id.as_str(),
        first_receipt.idem.as_str(),
        "operation_id must be explicit backend context, not silently derived from idempotency key"
    );
    assert_eq!(
        first_projection.receipt_hash.as_str(),
        first_receipt.receipt_hash.as_str()
    );
}

#[tokio::test]
async fn changed_body_under_same_idempotency_key_conflicts_under_preflight() {
    let router = app();

    let first_body = json!({
        "to": "acct_qc_idem_conflict",
        "asset": "roc",
        "amount_minor": "10",
        "memo": null
    });

    let changed_body = json!({
        "to": "acct_qc_idem_conflict",
        "asset": "roc",
        "amount_minor": "11",
        "memo": null
    });

    let (first_status, _first_bytes) = send(
        router.clone(),
        json_post_request("/v1/issue", "idem_qc_conflict_retry_key", first_body),
    )
    .await;

    let (second_status, second_bytes) = send(
        router,
        json_post_request("/v1/issue", "idem_qc_conflict_retry_key", changed_body),
    )
    .await;

    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(
        second_status,
        StatusCode::CONFLICT,
        "same idempotency key with different request body must remain a wallet conflict"
    );

    let error = parse_json(&second_bytes);
    assert_eq!(error["code"], "IDEMPOTENCY_CONFLICT");
}

#[tokio::test]
async fn changing_explicit_operation_id_does_not_rewrite_wallet_receipt_hash() {
    let router = app();

    let body = json!({
        "to": "acct_qc_idem_projection_split",
        "asset": "roc",
        "amount_minor": "77",
        "memo": null
    });

    let (status, bytes) = send(
        router,
        json_post_request("/v1/issue", "idem_qc_projection_split_retry_key", body),
    )
    .await;

    assert_eq!(status, StatusCode::OK);

    let receipt = parse_receipt(&bytes);

    let context_a = QuickChainWalletReceiptProjectionContext::accepted(
        "roc-dev",
        "op:wallet:backend-assigned:a",
    )
    .expect("context A should validate");

    let context_b = QuickChainWalletReceiptProjectionContext::accepted(
        "roc-dev",
        "op:wallet:backend-assigned:b",
    )
    .expect("context B should validate");

    let projection_a = project_wallet_receipt_for_quickchain_preflight(&receipt, &context_a)
        .expect("projection A should succeed");

    let projection_b = project_wallet_receipt_for_quickchain_preflight(&receipt, &context_b)
        .expect("projection B should succeed");

    assert_ne!(
        projection_a.operation_id, projection_b.operation_id,
        "operation_id is caller-supplied backend context"
    );

    assert_eq!(
        projection_a.idempotency_key, projection_b.idempotency_key,
        "idempotency key remains wallet retry identity"
    );

    assert_eq!(
        projection_a.receipt_hash, projection_b.receipt_hash,
        "changing projection operation_id must not rewrite the wallet receipt hash"
    );

    assert_eq!(
        projection_a.receipt_hash.as_str(),
        receipt.receipt_hash.as_str()
    );
}
