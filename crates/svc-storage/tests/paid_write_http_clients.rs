//! RO:WHAT — HTTP contract tests for svc-storage wallet receipt lookup client.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/RES. Wallet-backed paid storage must fail closed over HTTP.
//! RO:INTERACTS — WalletReceiptHttpClient, WalletReceipt, mock /v1/tx/{txid} Axum server.
//! RO:INVARIANTS — bearer forwarded; non-2xx rejects; bad JSON rejects; matched hold receipt verifies.
//! RO:METRICS — none; this validates client/verifier semantics only.
//! RO:CONFIG — mirrors RON_STORAGE_WALLET_BASE_URL and RON_STORAGE_WALLET_BEARER behavior without env.
//! RO:SECURITY — test bearer only; no real wallet secret or external network.
//! RO:TEST — cargo test -p svc-storage --test paid_write_http_client.

use std::time::Duration;

use axum::{
    extract::Path,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use svc_storage::policy::paid_write::{
    WalletReceipt, WalletReceiptHttpClient, H_PAID_ASSET, H_PAID_ESTIMATE_MINOR, H_PAID_OP,
    H_WALLET_FROM, H_WALLET_RECEIPT_HASH, H_WALLET_TO, H_WALLET_TXID,
};

const VALID_RECEIPT_HASH: &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn valid_wallet_receipt() -> WalletReceipt {
    WalletReceipt {
        txid: "tx_test_hold_1".to_string(),
        op: "hold".to_string(),
        from: Some("acct_user".to_string()),
        to: Some("escrow_paid_write".to_string()),
        asset: "roc".to_string(),
        amount_minor: "70".to_string(),
        nonce: Some(1),
        idem: Some("idem_hold_1".to_string()),
        ts: Some(1),
        ledger_seq_start: Some(1),
        ledger_seq_end: Some(2),
        ledger_root: Some(
            "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
        ),
        receipt_hash: VALID_RECEIPT_HASH.to_string(),
    }
}

fn valid_paid_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(H_PAID_OP, "hold".parse().expect("header value"));
    headers.insert(H_PAID_ASSET, "roc".parse().expect("header value"));
    headers.insert(H_PAID_ESTIMATE_MINOR, "70".parse().expect("header value"));
    headers.insert(
        H_WALLET_TXID,
        "tx_test_hold_1".parse().expect("header value"),
    );
    headers.insert(
        H_WALLET_RECEIPT_HASH,
        VALID_RECEIPT_HASH.parse().expect("header value"),
    );
    headers.insert(H_WALLET_FROM, "acct_user".parse().expect("header value"));
    headers.insert(
        H_WALLET_TO,
        "escrow_paid_write".parse().expect("header value"),
    );
    headers
}

async fn wallet_receipt_route(Path(txid): Path<String>, headers: HeaderMap) -> Response {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    if bearer != "Bearer dev" {
        return (StatusCode::UNAUTHORIZED, "missing or wrong bearer").into_response();
    }

    match txid.as_str() {
        "tx_test_hold_1" => Json(valid_wallet_receipt()).into_response(),
        "bad_json" => (StatusCode::OK, "not-json").into_response(),
        _ => (StatusCode::NOT_FOUND, "receipt not found").into_response(),
    }
}

async fn spawn_wallet_mock() -> String {
    let app = Router::new().route("/v1/tx/:txid", get(wallet_receipt_route));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock listener should bind");
    let addr = listener
        .local_addr()
        .expect("mock listener should expose local addr");

    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("mock wallet server should run");
    });

    format!("http://{addr}")
}

#[tokio::test]
async fn http_client_fetches_wallet_receipt_with_bearer() {
    let base_url = spawn_wallet_mock().await;
    let client = WalletReceiptHttpClient::new(base_url, Duration::from_secs(2), Some("dev".into()))
        .expect("client should build");

    let receipt = client
        .lookup_receipt("tx_test_hold_1")
        .await
        .expect("receipt lookup should succeed");

    assert_eq!(receipt.txid, "tx_test_hold_1");
    assert_eq!(receipt.op, "hold");
    assert_eq!(receipt.asset, "roc");
    assert_eq!(receipt.amount_minor, "70");
    assert_eq!(receipt.receipt_hash, VALID_RECEIPT_HASH);
}

#[tokio::test]
async fn http_client_verify_headers_accepts_matching_wallet_receipt() {
    let base_url = spawn_wallet_mock().await;
    let client = WalletReceiptHttpClient::new(base_url, Duration::from_secs(2), Some("dev".into()))
        .expect("client should build");

    let verified = client
        .verify_headers(&valid_paid_headers())
        .await
        .expect("matching HTTP wallet receipt should verify");

    assert_eq!(verified.verifier, "wallet-http");
    assert_eq!(verified.proof.txid, "tx_test_hold_1");
    assert_eq!(verified.proof.receipt_hash, VALID_RECEIPT_HASH);
    assert_eq!(verified.proof.payer, "acct_user");
    assert_eq!(verified.proof.escrow, "escrow_paid_write");
    assert_eq!(verified.proof.estimate_minor, 70);
}

#[tokio::test]
async fn http_client_fails_closed_without_bearer() {
    let base_url = spawn_wallet_mock().await;
    let client = WalletReceiptHttpClient::new(base_url, Duration::from_secs(2), None)
        .expect("client should build");

    let err = client
        .lookup_receipt("tx_test_hold_1")
        .await
        .expect_err("missing bearer should fail closed");

    assert!(err.reason().contains("status 401"));
}

#[tokio::test]
async fn http_client_fails_closed_on_missing_receipt() {
    let base_url = spawn_wallet_mock().await;
    let client = WalletReceiptHttpClient::new(base_url, Duration::from_secs(2), Some("dev".into()))
        .expect("client should build");

    let err = client
        .lookup_receipt("tx_missing")
        .await
        .expect_err("missing receipt should fail closed");

    assert!(err.reason().contains("status 404"));
}

#[tokio::test]
async fn http_client_fails_closed_on_bad_json() {
    let base_url = spawn_wallet_mock().await;
    let client = WalletReceiptHttpClient::new(base_url, Duration::from_secs(2), Some("dev".into()))
        .expect("client should build");

    let err = client
        .lookup_receipt("bad_json")
        .await
        .expect_err("bad JSON should fail closed");

    assert!(err.reason().contains("invalid JSON"));
}

#[tokio::test]
async fn http_client_rejects_unsafe_txid_before_network() {
    let base_url = spawn_wallet_mock().await;
    let client = WalletReceiptHttpClient::new(base_url, Duration::from_secs(2), Some("dev".into()))
        .expect("client should build");

    let err = client
        .lookup_receipt("../tx_test_hold_1")
        .await
        .expect_err("unsafe txid should fail before network");

    assert!(err.reason().contains("unsafe"));
}

#[test]
fn http_client_rejects_invalid_base_url() {
    let err = WalletReceiptHttpClient::new("127.0.0.1:8088", Duration::from_secs(2), None)
        .expect_err("base URL without scheme should reject");

    assert!(err.reason().contains("http:// or https://"));
}
