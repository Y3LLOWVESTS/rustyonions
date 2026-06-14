//! RO:WHAT — HTTP boundary tests proving QuickChain preflight does not become
//! live svc-wallet route authority.
//! RO:WHY — svc-wallet may expose an inert feature-gated projection helper, but
//! normal wallet routes must remain wallet/ledger hot-path receipts only.
//! RO:INTERACTS — routes::router, WalletState::dev, dto::responses::Receipt,
//! quickchain::project_wallet_receipt_for_quickchain_preflight.
//! RO:INVARIANTS — no chain_id, operation_id, schema, produced_at_ms, or
//! legacy_ledger_root in live HTTP receipts; projection requires explicit
//! backend context; request DTOs reject unknown QuickChain authority fields.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — prevents feature-gated review DTOs from becoming spend,
//! finality, settlement, root, anchor, or chain authority.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_no_runtime_authority.

#![cfg(feature = "quickchain-preflight")]

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use svc_wallet::{
    dto::responses::{Receipt, ReceiptSettlementStatus, WalletOp},
    quickchain::{
        project_wallet_receipt_for_quickchain_preflight, QuickChainWalletReceiptProjectionContext,
        QuickChainWalletReceiptStatus, SVC_WALLET_QUICKCHAIN_RECEIPT_PROJECTION_SCHEMA,
    },
    routes::{self, WalletState},
};
use tower::ServiceExt;

fn app() -> Router {
    let state = WalletState::dev().expect("dev wallet state should build");
    routes::router(state)
}

fn json_post_request(path: &str, idempotency_key: &str, body: Value) -> Request<Body> {
    let encoded = serde_json::to_vec(&body).expect("JSON body should encode");

    Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::AUTHORIZATION, "Bearer dev")
        .header(header::CONTENT_TYPE, "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(encoded))
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

fn json_body(body: &[u8]) -> Value {
    serde_json::from_slice(body).expect("response should be JSON")
}

fn assert_object_field_absent(value: &Value, field: &str) {
    let object = value.as_object().expect("receipt JSON should be object");
    assert!(
        !object.contains_key(field),
        "live HTTP receipt must not contain QuickChain projection field {field}"
    );
}

#[tokio::test]
async fn quickchain_preflight_does_not_change_live_http_receipt_shape() {
    let router = app();

    let issue_body = json!({
        "to": "acct_qc_http_alice",
        "asset": "roc",
        "amount_minor": "100",
        "memo": null
    });

    let (status, body) = send(
        router,
        json_post_request(
            "/v1/issue",
            "idem_qc_http_no_runtime_authority_issue",
            issue_body,
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);

    let receipt_json = json_body(&body);
    assert_eq!(receipt_json["op"], "issue");
    assert_eq!(receipt_json["to"], "acct_qc_http_alice");
    assert_eq!(receipt_json["amount_minor"], "100");
    assert_eq!(receipt_json["settlement_status"], "accepted");

    assert!(receipt_json["txid"]
        .as_str()
        .expect("txid should be string")
        .starts_with("tx_"));
    assert!(receipt_json["receipt_hash"]
        .as_str()
        .expect("receipt_hash should be string")
        .starts_with("b3:"));

    // Live wallet receipts keep wallet vocabulary. They must not expose the
    // feature-gated projection's chain/context/future-settlement vocabulary.
    assert_object_field_absent(&receipt_json, "schema");
    assert_object_field_absent(&receipt_json, "chain_id");
    assert_object_field_absent(&receipt_json, "operation_id");
    assert_object_field_absent(&receipt_json, "idempotency_key");
    assert_object_field_absent(&receipt_json, "produced_at_ms");
    assert_object_field_absent(&receipt_json, "legacy_ledger_root");
}

#[tokio::test]
async fn quickchain_projection_is_manual_and_requires_explicit_context() {
    let router = app();

    let issue_body = json!({
        "to": "acct_qc_http_projection",
        "asset": "roc",
        "amount_minor": "77",
        "memo": null
    });

    let (status, body) = send(
        router,
        json_post_request("/v1/issue", "idem_qc_http_manual_projection", issue_body),
    )
    .await;

    assert_eq!(status, StatusCode::OK);

    let receipt: Receipt =
        serde_json::from_slice(&body).expect("live HTTP receipt should deserialize");
    assert_eq!(receipt.op, WalletOp::Issue);
    assert_eq!(receipt.settlement_status, ReceiptSettlementStatus::Accepted);

    let context = QuickChainWalletReceiptProjectionContext::accepted(
        "roc-dev",
        "op:wallet:issue:http-boundary",
    )
    .expect("explicit projection context should validate");

    let projection = project_wallet_receipt_for_quickchain_preflight(&receipt, &context)
        .expect("manual projection should succeed");

    assert_eq!(
        projection.schema,
        SVC_WALLET_QUICKCHAIN_RECEIPT_PROJECTION_SCHEMA
    );
    assert_eq!(projection.chain_id, "roc-dev");
    assert_eq!(projection.operation_id, "op:wallet:issue:http-boundary");
    assert_eq!(projection.txid, receipt.txid);
    assert_eq!(projection.op, WalletOp::Issue);
    assert_eq!(projection.idempotency_key, receipt.idem);
    assert_eq!(projection.legacy_ledger_root, receipt.ledger_root);
    assert_eq!(projection.receipt_hash, receipt.receipt_hash);
    assert_eq!(
        projection.settlement_status,
        QuickChainWalletReceiptStatus::Accepted
    );
}

#[tokio::test]
async fn quickchain_authority_fields_are_rejected_as_unknown_request_fields() {
    let router = app();

    let poisoned_issue_body = json!({
        "to": "acct_qc_http_poisoned",
        "asset": "roc",
        "amount_minor": "9",
        "memo": null,
        "schema": "svc-wallet.quickchain-receipt-projection.v1",
        "chain_id": "roc-dev",
        "operation_id": "op:attacker:must-not-be-authority"
    });

    let (status, _body) = send(
        router,
        json_post_request(
            "/v1/issue",
            "idem_qc_http_unknown_fields_reject",
            poisoned_issue_body,
        ),
    )
    .await;

    assert!(
        status.is_client_error(),
        "unknown QuickChain authority fields must not be accepted by wallet request DTOs"
    );
    assert_ne!(status, StatusCode::OK);
}
