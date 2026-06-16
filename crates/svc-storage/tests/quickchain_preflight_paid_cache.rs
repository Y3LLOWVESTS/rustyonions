//! RO:WHAT — Paid/cache preflight tests for svc-storage QuickChain boundaries.
//! RO:WHY — Storage may cache/serve b3 bytes, but cache alone cannot prove paid entitlement.
//! RO:INTERACTS — /paid/o admission route, /o/:cid object route, MemoryStorage.
//! RO:INVARIANTS — missing proof never stores; GET does not become a paid-unlock authority.
//! RO:METRICS — none.
//! RO:CONFIG — in-process defaults; dev-header verifier remains explicit.
//! RO:SECURITY — fake paid/cache headers cannot unlock absent content or invent receipts.
//! RO:TEST — cargo test -p svc-storage --test quickchain_preflight_paid_cache.

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body, Bytes},
    http::{header, HeaderMap, Method, Request, StatusCode},
    Router,
};
use serde_json::Value;
use svc_storage::{
    http::{extractors::AppState, server::build_router},
    storage::{MemoryStorage, Storage},
};
use tower::ServiceExt;

const OBJECT_BYTES: &[u8] = b"quickchain paid cache boundary object";
const VALID_RECEIPT_HASH: &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn clear_paid_env() {
    std::env::remove_var("RON_STORAGE_PAID_WRITE_VERIFIER_MODE");
    std::env::remove_var("RON_STORAGE_PAID_SETTLEMENT_MODE");
    std::env::remove_var("RON_STORAGE_ACCOUNTING_EXPORT_MODE");
    std::env::remove_var("RON_STORAGE_ROC_ECONOMICS_PATH");
    std::env::remove_var("RON_STORAGE_ROC_ECONOMICS_ACTION");
}

fn app() -> Router {
    let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
    build_router().with_state(AppState { store })
}

fn expected_cid(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

fn paid_post(headers: &[(&str, &str)], bytes: &'static [u8]) -> Request<Body> {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri("/paid/o")
        .header(header::CONTENT_TYPE, "application/octet-stream");

    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }

    builder
        .body(Body::from(bytes))
        .expect("paid POST request should build")
}

fn get_with_headers(path: &str, headers: &[(&str, &str)]) -> Request<Body> {
    let mut builder = Request::builder().method(Method::GET).uri(path);

    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }

    builder
        .body(Body::empty())
        .expect("GET request should build")
}

fn head(path: &str) -> Request<Body> {
    Request::builder()
        .method(Method::HEAD)
        .uri(path)
        .body(Body::empty())
        .expect("HEAD request should build")
}

async fn send(router: Router, request: Request<Body>) -> (StatusCode, HeaderMap, Bytes) {
    let response = router
        .oneshot(request)
        .await
        .expect("router request should complete");

    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read");

    (status, headers, body)
}

async fn assert_absent(router: Router, cid: &str) {
    let (status, _headers, _body) = send(router, head(&format!("/o/{cid}"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn paid_write_rejects_without_backend_derived_proof_and_does_not_cache_bytes() {
    clear_paid_env();

    let app = app();
    let cid = expected_cid(OBJECT_BYTES);

    let (status, _headers, body) = send(app.clone(), paid_post(&[], OBJECT_BYTES)).await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED);

    let json: Value = serde_json::from_slice(&body).expect("error body should be JSON");
    assert_eq!(json["error"], "payment_required");

    assert_absent(app, &cid).await;
}

#[tokio::test]
async fn fake_cache_or_paid_headers_do_not_unlock_absent_object() {
    clear_paid_env();

    let app = app();
    let absent_cid = format!("b3:{}", "f".repeat(64));
    let path = format!("/o/{absent_cid}");

    let fake_unlock_headers = [
        ("x-ron-paid", "true"),
        ("x-ron-paid-unlock", "true"),
        ("x-ron-wallet-receipt-hash", VALID_RECEIPT_HASH),
        ("x-ron-cache-hit", "true"),
        ("x-ron-finalized", "true"),
    ];

    let (status, _headers, body) = send(app, get_with_headers(&path, &fake_unlock_headers)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(
        body.is_empty(),
        "absent paid object must not return fake cached bytes"
    );
}

#[tokio::test]
async fn dev_header_paid_write_response_is_labeled_as_storage_admission_not_finality() {
    clear_paid_env();

    let app = app();

    let headers = [
        ("x-ron-paid-op", "hold"),
        ("x-ron-paid-asset", "roc"),
        ("x-ron-paid-estimate-minor", "70"),
        ("x-ron-wallet-txid", "tx_dev_paid_cache_boundary"),
        ("x-ron-wallet-receipt-hash", VALID_RECEIPT_HASH),
        ("x-ron-wallet-from", "acct_user"),
        ("x-ron-wallet-to", "escrow_paid_write"),
    ];

    let (status, _headers, body) = send(app, paid_post(&headers, OBJECT_BYTES)).await;
    assert_eq!(status, StatusCode::OK);

    let json: Value = serde_json::from_slice(&body).expect("paid response should be JSON");
    assert_eq!(json["paid"], true);
    assert_eq!(json["verifier"], "dev-header");
    assert_eq!(json["settlement"], Value::Null);

    for forbidden in [
        "balance_minor",
        "available_minor",
        "state_root",
        "receipt_root",
        "checkpoint_hash",
        "finalized",
        "anchored",
        "bridge_settled",
    ] {
        assert!(
            json.get(forbidden).is_none(),
            "paid storage response must not claim {forbidden}"
        );
    }
}
