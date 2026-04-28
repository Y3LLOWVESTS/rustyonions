//! RO:WHAT — Negative admission tests for svc-storage paid-write proof headers.
//! RO:WHY — Pillar 12, Concerns ECON/SEC/RES; paid writes must fail closed before storing bytes.
//! RO:INTERACTS — svc_storage::http::server, MemoryStorage, /paid/o, /o/:cid, /metrics.
//! RO:INVARIANTS — malformed payment proof never writes; malformed accounting context never writes.
//! RO:METRICS — asserts storage_paid_write_total and storage_paid_write_bytes_total are exported.
//! RO:CONFIG — in-process amnesia-safe storage only.
//! RO:SECURITY — test proof headers only; no real bearer tokens, macaroons, or private data.
//! RO:TEST — cargo test -p svc-storage --test paid_write_policy.

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, HeaderMap, Method, Request, StatusCode},
    Router,
};
use serde_json::Value;
use svc_storage::{
    http::{extractors::AppState, server::build_router},
    storage::{MemoryStorage, Storage},
};
use tower::ServiceExt;

const OBJECT_BYTES: &[u8] = b"paid write policy hardening";
const VALID_RECEIPT_HASH: &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn app() -> Router {
    let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
    let state = AppState { store };

    build_router().with_state(state)
}

fn expected_cid(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

fn head_object(cid: &str) -> Request<Body> {
    Request::builder()
        .method(Method::HEAD)
        .uri(format!("/o/{cid}"))
        .body(Body::empty())
        .expect("HEAD object request should build")
}

fn paid_post_with_headers(headers: &[(&str, &str)]) -> Request<Body> {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri("/paid/o")
        .header(header::CONTENT_TYPE, "application/octet-stream");

    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }

    builder
        .body(Body::from(OBJECT_BYTES))
        .expect("paid POST request should build")
}

fn valid_paid_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("x-ron-paid-op", "hold"),
        ("x-ron-paid-asset", "roc"),
        ("x-ron-paid-estimate-minor", "70"),
        ("x-ron-wallet-txid", "tx_test_hold_1"),
        ("x-ron-wallet-receipt-hash", VALID_RECEIPT_HASH),
        ("x-ron-wallet-from", "acct_user"),
        ("x-ron-wallet-to", "escrow_paid_write"),
    ]
}

async fn send(router: Router, request: Request<Body>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let response = router
        .oneshot(request)
        .await
        .expect("router request should complete");

    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read")
        .to_vec();

    (status, headers, body)
}

async fn assert_object_absent(router: Router, cid: &str) {
    let (status, _headers, _body) = send(router, head_object(cid)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

fn error_body(bytes: &[u8]) -> Value {
    serde_json::from_slice::<Value>(bytes).expect("error body should be JSON")
}

#[cfg(feature = "metrics")]
async fn metrics_text(router: Router) -> String {
    let request = Request::builder()
        .method(Method::GET)
        .uri("/metrics")
        .body(Body::empty())
        .expect("metrics request should build");

    let (status, _headers, body) = send(router, request).await;
    assert_eq!(status, StatusCode::OK);

    String::from_utf8(body).expect("metrics body should be UTF-8")
}

#[tokio::test]
async fn paid_write_without_proof_rejects_and_does_not_store() {
    let app = app();
    let cid = expected_cid(OBJECT_BYTES);

    let (status, _headers, body) = send(app.clone(), paid_post_with_headers(&[])).await;

    assert_eq!(status, StatusCode::PAYMENT_REQUIRED);

    let error = error_body(&body);
    assert_eq!(error["error"], "payment_required");
    assert!(error["reason"]
        .as_str()
        .expect("reason should be string")
        .contains("missing required paid proof header"));

    assert_object_absent(app, &cid).await;
}

#[tokio::test]
async fn malformed_paid_proof_headers_reject_and_do_not_store() {
    let cases: Vec<(&str, Vec<(&'static str, &'static str)>)> = vec![
        (
            "wrong operation",
            vec![
                ("x-ron-paid-op", "transfer"),
                ("x-ron-paid-asset", "roc"),
                ("x-ron-paid-estimate-minor", "70"),
                ("x-ron-wallet-txid", "tx_test_hold_1"),
                ("x-ron-wallet-receipt-hash", VALID_RECEIPT_HASH),
                ("x-ron-wallet-from", "acct_user"),
                ("x-ron-wallet-to", "escrow_paid_write"),
            ],
        ),
        (
            "wrong asset",
            vec![
                ("x-ron-paid-op", "hold"),
                ("x-ron-paid-asset", "sol"),
                ("x-ron-paid-estimate-minor", "70"),
                ("x-ron-wallet-txid", "tx_test_hold_1"),
                ("x-ron-wallet-receipt-hash", VALID_RECEIPT_HASH),
                ("x-ron-wallet-from", "acct_user"),
                ("x-ron-wallet-to", "escrow_paid_write"),
            ],
        ),
        (
            "zero estimate",
            vec![
                ("x-ron-paid-op", "hold"),
                ("x-ron-paid-asset", "roc"),
                ("x-ron-paid-estimate-minor", "0"),
                ("x-ron-wallet-txid", "tx_test_hold_1"),
                ("x-ron-wallet-receipt-hash", VALID_RECEIPT_HASH),
                ("x-ron-wallet-from", "acct_user"),
                ("x-ron-wallet-to", "escrow_paid_write"),
            ],
        ),
        (
            "non integer estimate",
            vec![
                ("x-ron-paid-op", "hold"),
                ("x-ron-paid-asset", "roc"),
                ("x-ron-paid-estimate-minor", "7.5"),
                ("x-ron-wallet-txid", "tx_test_hold_1"),
                ("x-ron-wallet-receipt-hash", VALID_RECEIPT_HASH),
                ("x-ron-wallet-from", "acct_user"),
                ("x-ron-wallet-to", "escrow_paid_write"),
            ],
        ),
        (
            "bad receipt hash",
            vec![
                ("x-ron-paid-op", "hold"),
                ("x-ron-paid-asset", "roc"),
                ("x-ron-paid-estimate-minor", "70"),
                ("x-ron-wallet-txid", "tx_test_hold_1"),
                ("x-ron-wallet-receipt-hash", "not-a-b3-cid"),
                ("x-ron-wallet-from", "acct_user"),
                ("x-ron-wallet-to", "escrow_paid_write"),
            ],
        ),
    ];

    for (label, headers) in cases {
        let app = app();
        let cid = expected_cid(OBJECT_BYTES);

        let (status, _response_headers, body) =
            send(app.clone(), paid_post_with_headers(&headers)).await;

        assert_eq!(status, StatusCode::PAYMENT_REQUIRED, "{label}");

        let error = error_body(&body);
        assert_eq!(error["error"], "payment_required", "{label}");

        assert_object_absent(app, &cid).await;
    }
}

#[tokio::test]
async fn malformed_accounting_context_rejects_and_does_not_store() {
    let cases: Vec<(&str, Vec<(&'static str, &'static str)>)> = vec![
        ("zero tenant", {
            let mut headers = valid_paid_headers();
            headers.push(("x-ron-tenant", "0"));
            headers
        }),
        ("non integer tenant", {
            let mut headers = valid_paid_headers();
            headers.push(("x-ron-tenant", "tenant-a"));
            headers
        }),
        ("non integer pin seconds", {
            let mut headers = valid_paid_headers();
            headers.push(("x-ron-pin-seconds", "sixty"));
            headers
        }),
    ];

    for (label, headers) in cases {
        let app = app();
        let cid = expected_cid(OBJECT_BYTES);

        let (status, _response_headers, body) =
            send(app.clone(), paid_post_with_headers(&headers)).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "{label}");

        let error = error_body(&body);
        assert_eq!(error["error"], "bad_accounting_context", "{label}");

        assert_object_absent(app, &cid).await;
    }
}

#[tokio::test]
async fn valid_minimal_paid_proof_stores_object() {
    let app = app();
    let cid = expected_cid(OBJECT_BYTES);

    let (status, _headers, body) =
        send(app.clone(), paid_post_with_headers(&valid_paid_headers())).await;

    assert_eq!(status, StatusCode::OK);

    let response = serde_json::from_slice::<Value>(&body).expect("paid response should be JSON");
    assert_eq!(response["cid"], cid);
    assert_eq!(response["paid"], true);
    assert_eq!(response["usage_events"].as_array().unwrap().len(), 2);

    let (head_status, _head_headers, _head_body) = send(app, head_object(&cid)).await;
    assert_eq!(head_status, StatusCode::OK);
}

#[cfg(feature = "metrics")]
#[tokio::test]
async fn paid_write_metrics_expose_admission_statuses() {
    let app = app();

    let (_missing_status, _missing_headers, _missing_body) =
        send(app.clone(), paid_post_with_headers(&[])).await;

    let (_bad_context_status, _bad_context_headers, _bad_context_body) = {
        let mut headers = valid_paid_headers();
        headers.push(("x-ron-tenant", "0"));
        send(app.clone(), paid_post_with_headers(&headers)).await
    };

    let (_ok_status, _ok_headers, _ok_body) =
        send(app.clone(), paid_post_with_headers(&valid_paid_headers())).await;

    let metrics = metrics_text(app.clone()).await;

    assert!(metrics.contains("storage_paid_write_total"));
    assert!(metrics.contains("storage_paid_write_bytes_total"));
    assert!(metrics.contains("storage_paid_write_total{status=\"payment_required\"}"));
    assert!(metrics.contains("storage_paid_write_total{status=\"bad_accounting_context\"}"));
    assert!(metrics.contains("storage_paid_write_total{status=\"accepted\"}"));
}
