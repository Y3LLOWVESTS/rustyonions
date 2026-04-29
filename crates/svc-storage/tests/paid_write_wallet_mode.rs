//! RO:WHAT — Route-level tests for /paid/o in wallet-receipt verifier mode.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/RES. Paid storage must verify wallet receipts before CAS writes.
//! RO:INTERACTS — svc_storage::http::server, WalletReceiptHttpClient, mock svc-wallet /v1/tx/{txid}.
//! RO:INVARIANTS — wallet-receipt mode calls wallet; matching hold stores; mismatched wallet/context rejects.
//! RO:METRICS — exercises storage_paid_write_total status paths indirectly.
//! RO:CONFIG — sets RON_STORAGE_PAID_WRITE_VERIFIER_MODE, wallet base URL, bearer, and timeout in-process.
//! RO:SECURITY — mock bearer only; no real wallet secret, macaroon, PII, or external network.
//! RO:TEST — cargo test -p svc-storage --test paid_write_wallet_mode.

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    extract::Path,
    http::{header, HeaderMap, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::Value;
use svc_storage::{
    http::{extractors::AppState, server::build_router},
    policy::paid_write::{paid_storage_context_idem, WalletReceipt},
    storage::{MemoryStorage, Storage},
};
use tower::ServiceExt;

const OBJECT_BYTES: &[u8] = b"wallet receipt mode stores this paid object";
const VALID_RECEIPT_HASH: &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn storage_app() -> Router {
    let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
    let state = AppState { store };

    build_router().with_state(state)
}

fn expected_cid(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

fn context_idem_for_cid(cid: &str) -> String {
    paid_storage_context_idem(cid, "acct_user", "escrow_paid_write", "roc", 70)
        .expect("paid storage context idem should compute")
}

fn valid_wallet_receipt(txid: &str, cid: &str) -> WalletReceipt {
    WalletReceipt {
        txid: txid.to_string(),
        op: "hold".to_string(),
        from: Some("acct_user".to_string()),
        to: Some("escrow_paid_write".to_string()),
        asset: "roc".to_string(),
        amount_minor: "70".to_string(),
        nonce: Some(1),
        idem: Some(context_idem_for_cid(cid)),
        ts: Some(1),
        ledger_seq_start: Some(1),
        ledger_seq_end: Some(2),
        ledger_root: Some(
            "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
        ),
        receipt_hash: VALID_RECEIPT_HASH.to_string(),
    }
}

fn mismatched_wallet_receipt(txid: &str, cid: &str) -> WalletReceipt {
    let mut receipt = valid_wallet_receipt(txid, cid);
    receipt.from = Some("acct_attacker".to_string());
    receipt
}

async fn wallet_receipt_route(Path(txid): Path<String>, headers: HeaderMap) -> Response {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    if bearer != "Bearer dev" {
        return (StatusCode::UNAUTHORIZED, "missing or wrong bearer").into_response();
    }

    let object_cid = expected_cid(OBJECT_BYTES);

    match txid.as_str() {
        "tx_route_good" => Json(valid_wallet_receipt("tx_route_good", &object_cid)).into_response(),
        "tx_route_mismatch" => {
            Json(mismatched_wallet_receipt("tx_route_mismatch", &object_cid)).into_response()
        }
        _ => (StatusCode::NOT_FOUND, "receipt not found").into_response(),
    }
}

async fn spawn_wallet_mock() -> String {
    let app = Router::new().route("/v1/tx/:txid", get(wallet_receipt_route));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock wallet listener should bind");
    let addr = listener
        .local_addr()
        .expect("mock wallet listener should expose local addr");

    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("mock wallet server should run");
    });

    format!("http://{addr}")
}

fn paid_headers(txid: &'static str, cid: &str) -> Vec<(String, String)> {
    vec![
        ("x-ron-paid-op".to_string(), "hold".to_string()),
        ("x-ron-paid-asset".to_string(), "roc".to_string()),
        ("x-ron-paid-estimate-minor".to_string(), "70".to_string()),
        ("x-ron-wallet-txid".to_string(), txid.to_string()),
        (
            "x-ron-wallet-receipt-hash".to_string(),
            VALID_RECEIPT_HASH.to_string(),
        ),
        ("x-ron-wallet-from".to_string(), "acct_user".to_string()),
        (
            "x-ron-wallet-to".to_string(),
            "escrow_paid_write".to_string(),
        ),
        ("x-ron-wallet-idem".to_string(), context_idem_for_cid(cid)),
        ("x-ron-tenant".to_string(), "7".to_string()),
        (
            "x-ron-accounting-subject".to_string(),
            "svc_storage_provider".to_string(),
        ),
        ("x-ron-region".to_string(), "us-central".to_string()),
        ("x-ron-pin-seconds".to_string(), "60".to_string()),
    ]
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

fn get_object(cid: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(format!("/o/{cid}"))
        .body(Body::empty())
        .expect("GET object request should build")
}

fn head_object(cid: &str) -> Request<Body> {
    Request::builder()
        .method(Method::HEAD)
        .uri(format!("/o/{cid}"))
        .body(Body::empty())
        .expect("HEAD object request should build")
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

fn json_body(bytes: &[u8]) -> Value {
    serde_json::from_slice::<Value>(bytes).expect("response body should be JSON")
}

async fn assert_object_absent(router: Router, cid: &str) {
    let (status, _headers, _body) = send(router, head_object(cid)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

fn configure_wallet_receipt_mode(wallet_base_url: &str) {
    std::env::set_var("RON_STORAGE_PAID_WRITE_VERIFIER_MODE", "wallet-receipt");
    std::env::set_var("RON_STORAGE_WALLET_BASE_URL", wallet_base_url);
    std::env::set_var("RON_STORAGE_WALLET_BEARER", "dev");
    std::env::set_var("RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS", "2000");
}

fn clear_wallet_receipt_mode() {
    std::env::remove_var("RON_STORAGE_PAID_WRITE_VERIFIER_MODE");
    std::env::remove_var("RON_STORAGE_WALLET_BASE_URL");
    std::env::remove_var("RON_STORAGE_WALLET_BEARER");
    std::env::remove_var("RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS");
}

#[tokio::test]
async fn paid_route_wallet_receipt_mode_accepts_matching_receipt_and_rejects_mismatch() {
    let wallet_base_url = spawn_wallet_mock().await;
    configure_wallet_receipt_mode(&wallet_base_url);

    let app = storage_app();
    let cid = expected_cid(OBJECT_BYTES);

    let (ok_status, _ok_headers, ok_body) = send(
        app.clone(),
        paid_post_with_headers(&paid_headers("tx_route_good", &cid), OBJECT_BYTES),
    )
    .await;

    assert_eq!(ok_status, StatusCode::OK);

    let ok_json = json_body(&ok_body);
    assert_eq!(ok_json["paid"], true);
    assert_eq!(ok_json["cid"], cid);
    assert_eq!(ok_json["payer"], "acct_user");
    assert_eq!(ok_json["escrow"], "escrow_paid_write");
    assert_eq!(ok_json["wallet_txid"], "tx_route_good");
    assert_eq!(ok_json["wallet_receipt_hash"], VALID_RECEIPT_HASH);
    assert_eq!(ok_json["wallet_idem"], context_idem_for_cid(&cid));
    assert_eq!(ok_json["paid_context_idem"], context_idem_for_cid(&cid));
    assert_eq!(ok_json["verifier"], "wallet-http");
    assert_eq!(ok_json["estimate_minor"], "70");

    let usage_events = ok_json["usage_events"]
        .as_array()
        .expect("usage_events should be an array");

    assert!(
        usage_events
            .iter()
            .any(|event| event["metric_kind"] == "bytes_stored"),
        "paid write should include bytes_stored accounting event"
    );
    assert!(
        usage_events
            .iter()
            .any(|event| event["metric_kind"] == "request_ok"),
        "paid write should include request_ok accounting event"
    );
    assert!(
        usage_events
            .iter()
            .any(|event| event["metric_kind"] == "pin_seconds"),
        "paid write should include pin_seconds accounting event when requested"
    );

    let (get_status, _get_headers, get_body) = send(app.clone(), get_object(&cid)).await;
    assert_eq!(get_status, StatusCode::OK);
    assert_eq!(get_body, OBJECT_BYTES);

    let mismatch_bytes = b"wallet receipt mismatch should not store";
    let mismatch_cid = expected_cid(mismatch_bytes);

    let mismatch_request = paid_post_with_headers(
        &paid_headers("tx_route_mismatch", &mismatch_cid),
        mismatch_bytes,
    );

    let (bad_status, _bad_headers, bad_body) = send(app.clone(), mismatch_request).await;
    assert_eq!(bad_status, StatusCode::PAYMENT_REQUIRED);

    let bad_json = json_body(&bad_body);
    assert_eq!(bad_json["error"], "payment_required");
    assert!(
        bad_json["reason"]
            .as_str()
            .expect("reason should be string")
            .contains("payer mismatch"),
        "mismatched wallet receipt should explain payer mismatch"
    );

    assert_object_absent(app.clone(), &mismatch_cid).await;

    let context_mismatch_bytes = b"same receipt wrong body should not store";
    let context_mismatch_cid = expected_cid(context_mismatch_bytes);

    let context_mismatch_request =
        paid_post_with_headers(&paid_headers("tx_route_good", &cid), context_mismatch_bytes);

    let (context_status, _context_headers, context_body) =
        send(app.clone(), context_mismatch_request).await;
    assert_eq!(context_status, StatusCode::PAYMENT_REQUIRED);

    let context_json = json_body(&context_body);
    assert_eq!(context_json["error"], "payment_required");
    assert!(
        context_json["reason"]
            .as_str()
            .expect("reason should be string")
            .contains("paid storage context idem mismatch"),
        "same valid hold must reject when reused for a different CID"
    );

    assert_object_absent(app, &context_mismatch_cid).await;

    clear_wallet_receipt_mode();
}
