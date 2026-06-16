//! RO:WHAT — QuickChain Phase-0 observability preflight tests for svc-storage.
//! RO:WHY — Metrics may expose service health/admission status, but must not leak storage CIDs, wallet IDs, receipts, roots, or authority.
//! RO:INTERACTS — /metrics route, paid /paid/o route, metrics module, in-memory storage.
//! RO:INVARIANTS — observability is low-cardinality; metrics are not balance truth, receipt truth, root truth, or finality truth.
//! RO:METRICS — directly checks Prometheus output and metric source labels.
//! RO:CONFIG — sets dev-header paid-write mode and disables settlement/accounting export for local in-process proof.
//! RO:SECURITY — no private identifiers or chain-authority fields may appear in metric labels/output.
//! RO:TEST — cargo test -p svc-storage --test quickchain_preflight_observability.

use std::{fs, path::PathBuf, sync::Arc};

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

const OBJECT_BYTES: &[u8] = b"quickchain observability paid storage object";
const WALLET_TXID: &str = "tx_metrics_no_private_labels";
const PAYER: &str = "acct_metrics_payer";
const ESCROW: &str = "escrow_metrics_storage";

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    fs::read_to_string(crate_dir().join(relative)).unwrap_or_else(|err| {
        panic!("failed to read {relative}: {err}");
    })
}

fn app() -> Router {
    let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
    let state = AppState { store };

    build_router().with_state(state)
}

fn request(method: Method, uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .expect("request should build")
}

fn paid_post_with_headers(headers: &[(String, String)], body: &[u8]) -> Request<Body> {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri("/paid/o")
        .header(header::CONTENT_TYPE, "application/octet-stream");

    for (name, value) in headers {
        builder = builder.header(name.as_str(), value.as_str());
    }

    builder
        .body(Body::from(body.to_vec()))
        .expect("paid POST request should build")
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

fn json_body(bytes: &[u8]) -> Value {
    serde_json::from_slice::<Value>(bytes).expect("response body should be JSON")
}

fn receipt_hash() -> String {
    format!("b3:{}", "d".repeat(64))
}

fn paid_headers(receipt_hash: &str) -> Vec<(String, String)> {
    vec![
        ("x-ron-paid-op".to_string(), "hold".to_string()),
        ("x-ron-paid-asset".to_string(), "roc".to_string()),
        ("x-ron-paid-estimate-minor".to_string(), "70".to_string()),
        ("x-ron-wallet-txid".to_string(), WALLET_TXID.to_string()),
        (
            "x-ron-wallet-receipt-hash".to_string(),
            receipt_hash.to_string(),
        ),
        ("x-ron-wallet-from".to_string(), PAYER.to_string()),
        ("x-ron-wallet-to".to_string(), ESCROW.to_string()),
        ("x-ron-tenant".to_string(), "9".to_string()),
        (
            "x-ron-accounting-subject".to_string(),
            "svc_storage_metrics_subject".to_string(),
        ),
        ("x-ron-region".to_string(), "us-central".to_string()),
        ("x-ron-pin-seconds".to_string(), "30".to_string()),
    ]
}

fn configure_local_paid_storage_metrics_path() {
    std::env::set_var("RON_STORAGE_PAID_WRITE_VERIFIER_MODE", "dev-header");
    std::env::set_var("RON_STORAGE_PAID_SETTLEMENT_MODE", "disabled");
    std::env::set_var("RON_STORAGE_ACCOUNTING_EXPORT_MODE", "disabled");
    std::env::remove_var("RON_STORAGE_ROC_ECONOMICS_PATH");
    std::env::remove_var("RON_STORAGE_ROC_ECONOMICS_ACTION");
}

fn clear_local_paid_storage_metrics_path() {
    std::env::remove_var("RON_STORAGE_PAID_WRITE_VERIFIER_MODE");
    std::env::remove_var("RON_STORAGE_PAID_SETTLEMENT_MODE");
    std::env::remove_var("RON_STORAGE_ACCOUNTING_EXPORT_MODE");
    std::env::remove_var("RON_STORAGE_ROC_ECONOMICS_PATH");
    std::env::remove_var("RON_STORAGE_ROC_ECONOMICS_ACTION");
}

#[tokio::test]
async fn metrics_do_not_expose_cids_wallet_receipts_accounts_or_chain_authority() {
    configure_local_paid_storage_metrics_path();

    let app = app();
    let receipt_hash = receipt_hash();

    let (paid_status, _paid_headers, paid_body) = send(
        app.clone(),
        paid_post_with_headers(&paid_headers(&receipt_hash), OBJECT_BYTES),
    )
    .await;

    assert_eq!(paid_status, StatusCode::OK);

    let paid_json = json_body(&paid_body);
    let cid = paid_json["cid"]
        .as_str()
        .expect("paid write response should include cid")
        .to_string();

    assert!(
        cid.starts_with("b3:"),
        "paid write response should still use content-addressed b3"
    );

    let (metrics_status, _metrics_headers, metrics_body) =
        send(app, request(Method::GET, "/metrics", Body::empty())).await;

    assert_eq!(metrics_status, StatusCode::OK);

    let metrics_text =
        String::from_utf8(metrics_body.to_vec()).expect("/metrics should be UTF-8 text");

    assert!(
        metrics_text.contains("storage_paid_write_total"),
        "metrics should expose low-cardinality paid-write admission status"
    );

    for forbidden in [
        cid.as_str(),
        receipt_hash.as_str(),
        WALLET_TXID,
        PAYER,
        ESCROW,
        "svc_storage_metrics_subject",
        "wallet_txid",
        "wallet_receipt_hash",
        "receipt_hash",
        "balance_minor",
        "available_minor",
        "held_minor",
        "state_root",
        "receipt_root",
        "checkpoint_hash",
        "validator_signature",
        "finalized",
        "anchored",
        "bridge_settled",
    ] {
        assert!(
            !metrics_text.contains(forbidden),
            "/metrics must not expose private identifiers or chain authority field `{forbidden}`"
        );
    }

    clear_local_paid_storage_metrics_path();
}

#[test]
fn metrics_source_keeps_labels_low_cardinality_and_non_authoritative() {
    let metrics = read("src/metrics.rs").to_ascii_lowercase();

    assert!(
        metrics.contains("&[\"status\"]"),
        "svc-storage metrics should use bounded machine-status labels"
    );

    for forbidden_label_shape in [
        "&[\"cid\"",
        "&[\"b3\"",
        "&[\"account\"",
        "&[\"payer\"",
        "&[\"escrow\"",
        "&[\"wallet\"",
        "&[\"txid\"",
        "&[\"receipt\"",
        "&[\"balance\"",
        "&[\"root\"",
        "&[\"checkpoint\"",
        "&[\"validator\"",
        "&[\"bridge\"",
        "with_label_values(&[cid",
        "with_label_values(&[payer",
        "with_label_values(&[escrow",
        "with_label_values(&[wallet",
        "with_label_values(&[receipt",
    ] {
        assert!(
            !metrics.contains(forbidden_label_shape),
            "metrics labels must stay low-cardinality and non-authoritative: found `{forbidden_label_shape}`"
        );
    }

    for required in [
        "storage_paid_write_total",
        "storage_paid_write_bytes_total",
        "storage_accounting_export_total",
    ] {
        assert!(
            metrics.contains(required),
            "metrics source should keep expected storage observability family `{required}`"
        );
    }
}
