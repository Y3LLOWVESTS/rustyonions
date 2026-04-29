//! RO:WHAT — Route-level settlement tests for /paid/o wallet capture/release mode.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/RES. Paid storage must settle through wallet, not ledger mutation.
//! RO:INTERACTS — svc_storage::http::server, mock svc-wallet /v1/tx /v1/capture /v1/release.
//! RO:INVARIANTS — hold validates first; capture actual cost; release remainder; object remains CID-addressed.
//! RO:METRICS — exercises paid-write accepted/settlement_error paths indirectly.
//! RO:CONFIG — sets wallet-receipt verifier plus wallet-capture settlement mode.
//! RO:SECURITY — mock bearer only; no real wallet secret, macaroon, PII, or external network.
//! RO:TEST — cargo test -p svc-storage --test paid_write_settlement.

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    extract::Path,
    http::{header, HeaderMap, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use svc_storage::{
    http::{extractors::AppState, server::build_router},
    policy::paid_write::{paid_storage_context_idem, WalletReceipt},
    storage::{MemoryStorage, Storage},
};
use tower::ServiceExt;

const OBJECT_BYTES: &[u8] = b"settlement stores this object";
const HOLD_AMOUNT: u128 = 70;
const VALID_HOLD_HASH: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const VALID_CAPTURE_HASH: &str =
    "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const VALID_RELEASE_HASH: &str =
    "b3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn storage_app() -> Router {
    let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
    let state = AppState { store };

    build_router().with_state(state)
}

fn expected_cid(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

fn context_idem_for_cid(cid: &str) -> String {
    paid_storage_context_idem(cid, "acct_user", "escrow_paid_write", "roc", HOLD_AMOUNT)
        .expect("paid storage context idem should compute")
}

fn valid_hold_receipt(txid: &str, cid: &str) -> WalletReceipt {
    WalletReceipt {
        txid: txid.to_string(),
        op: "hold".to_string(),
        from: Some("acct_user".to_string()),
        to: Some("escrow_paid_write".to_string()),
        asset: "roc".to_string(),
        amount_minor: HOLD_AMOUNT.to_string(),
        nonce: Some(1),
        idem: Some(context_idem_for_cid(cid)),
        ts: Some(1),
        ledger_seq_start: Some(1),
        ledger_seq_end: Some(2),
        ledger_root: Some(
            "b3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".to_string(),
        ),
        receipt_hash: VALID_HOLD_HASH.to_string(),
    }
}

fn receipt_from_transfer(
    txid: &str,
    op: &str,
    hash: &str,
    request: &Value,
    headers: &HeaderMap,
) -> WalletReceipt {
    WalletReceipt {
        txid: txid.to_string(),
        op: op.to_string(),
        from: Some(
            request["from"]
                .as_str()
                .expect("from should be a string")
                .to_string(),
        ),
        to: Some(
            request["to"]
                .as_str()
                .expect("to should be a string")
                .to_string(),
        ),
        asset: request["asset"]
            .as_str()
            .expect("asset should be a string")
            .to_string(),
        amount_minor: request["amount_minor"]
            .as_str()
            .expect("amount_minor should be a string")
            .to_string(),
        nonce: Some(request["nonce"].as_u64().expect("nonce should be u64")),
        idem: Some(
            headers
                .get("idempotency-key")
                .and_then(|value| value.to_str().ok())
                .expect("Idempotency-Key should be present")
                .to_string(),
        ),
        ts: Some(2),
        ledger_seq_start: Some(3),
        ledger_seq_end: Some(4),
        ledger_root: Some(
            "b3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string(),
        ),
        receipt_hash: hash.to_string(),
    }
}

fn authorized(headers: &HeaderMap) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        == Some("Bearer dev")
}

async fn wallet_receipt_route(Path(txid): Path<String>, headers: HeaderMap) -> Response {
    if !authorized(&headers) {
        return (StatusCode::UNAUTHORIZED, "missing or wrong bearer").into_response();
    }

    let object_cid = expected_cid(OBJECT_BYTES);

    match txid.as_str() {
        "tx_settle_good" => Json(valid_hold_receipt("tx_settle_good", &object_cid)).into_response(),
        _ => (StatusCode::NOT_FOUND, "receipt not found").into_response(),
    }
}

async fn wallet_capture_route(headers: HeaderMap, Json(request): Json<Value>) -> Response {
    if !authorized(&headers) {
        return (StatusCode::UNAUTHORIZED, "missing or wrong bearer").into_response();
    }

    if request["from"] != "escrow_paid_write"
        || request["to"] != "svc_storage"
        || request["asset"] != "roc"
        || request["nonce"] != 1
    {
        return (StatusCode::BAD_REQUEST, Json(request)).into_response();
    }

    Json(receipt_from_transfer(
        "tx_capture_paid_storage",
        "capture",
        VALID_CAPTURE_HASH,
        &request,
        &headers,
    ))
    .into_response()
}

async fn wallet_release_route(headers: HeaderMap, Json(request): Json<Value>) -> Response {
    if !authorized(&headers) {
        return (StatusCode::UNAUTHORIZED, "missing or wrong bearer").into_response();
    }

    if request["from"] != "escrow_paid_write"
        || request["to"] != "acct_user"
        || request["asset"] != "roc"
        || request["nonce"] != 2
    {
        return (StatusCode::BAD_REQUEST, Json(request)).into_response();
    }

    Json(receipt_from_transfer(
        "tx_release_paid_storage",
        "release",
        VALID_RELEASE_HASH,
        &request,
        &headers,
    ))
    .into_response()
}

async fn spawn_wallet_mock() -> String {
    let app = Router::new()
        .route("/v1/tx/:txid", get(wallet_receipt_route))
        .route("/v1/capture", post(wallet_capture_route))
        .route("/v1/release", post(wallet_release_route));

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

fn paid_headers(cid: &str) -> Vec<(String, String)> {
    vec![
        ("x-ron-paid-op".to_string(), "hold".to_string()),
        ("x-ron-paid-asset".to_string(), "roc".to_string()),
        (
            "x-ron-paid-estimate-minor".to_string(),
            HOLD_AMOUNT.to_string(),
        ),
        (
            "x-ron-wallet-txid".to_string(),
            "tx_settle_good".to_string(),
        ),
        (
            "x-ron-wallet-receipt-hash".to_string(),
            VALID_HOLD_HASH.to_string(),
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

fn configure_settlement_mode(wallet_base_url: &str) {
    std::env::set_var("RON_STORAGE_PAID_WRITE_VERIFIER_MODE", "wallet-receipt");
    std::env::set_var("RON_STORAGE_PAID_SETTLEMENT_MODE", "wallet-capture");
    std::env::set_var("RON_STORAGE_PAID_SETTLEMENT_PAYEE", "svc_storage");
    std::env::set_var("RON_STORAGE_WALLET_BASE_URL", wallet_base_url);
    std::env::set_var("RON_STORAGE_WALLET_BEARER", "dev");
    std::env::set_var("RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS", "2000");
}

fn clear_settlement_mode() {
    std::env::remove_var("RON_STORAGE_PAID_WRITE_VERIFIER_MODE");
    std::env::remove_var("RON_STORAGE_PAID_SETTLEMENT_MODE");
    std::env::remove_var("RON_STORAGE_PAID_SETTLEMENT_PAYEE");
    std::env::remove_var("RON_STORAGE_WALLET_BASE_URL");
    std::env::remove_var("RON_STORAGE_WALLET_BEARER");
    std::env::remove_var("RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS");
}

#[tokio::test]
async fn paid_route_wallet_settlement_captures_actual_cost_and_releases_remainder() {
    let wallet_base_url = spawn_wallet_mock().await;
    configure_settlement_mode(&wallet_base_url);

    let app = storage_app();
    let cid = expected_cid(OBJECT_BYTES);
    let capture_amount = OBJECT_BYTES.len() as u128;
    let release_amount = HOLD_AMOUNT - capture_amount;

    let (status, _headers, body) = send(
        app.clone(),
        paid_post_with_headers(&paid_headers(&cid), OBJECT_BYTES),
    )
    .await;

    assert_eq!(status, StatusCode::OK);

    let json = json_body(&body);
    assert_eq!(json["paid"], true);
    assert_eq!(json["cid"], cid);
    assert_eq!(json["verifier"], "wallet-http");

    let settlement = &json["settlement"];
    assert_eq!(settlement["mode"], "wallet-capture");
    assert_eq!(
        settlement["capture_amount_minor"],
        capture_amount.to_string()
    );
    assert_eq!(
        settlement["release_amount_minor"],
        release_amount.to_string()
    );

    assert_eq!(settlement["capture_receipt"]["op"], "capture");
    assert_eq!(settlement["capture_receipt"]["from"], "escrow_paid_write");
    assert_eq!(settlement["capture_receipt"]["to"], "svc_storage");
    assert_eq!(
        settlement["capture_receipt"]["amount_minor"],
        capture_amount.to_string()
    );
    assert_eq!(
        settlement["capture_receipt"]["receipt_hash"],
        VALID_CAPTURE_HASH
    );

    assert_eq!(settlement["release_receipt"]["op"], "release");
    assert_eq!(settlement["release_receipt"]["from"], "escrow_paid_write");
    assert_eq!(settlement["release_receipt"]["to"], "acct_user");
    assert_eq!(
        settlement["release_receipt"]["amount_minor"],
        release_amount.to_string()
    );
    assert_eq!(
        settlement["release_receipt"]["receipt_hash"],
        VALID_RELEASE_HASH
    );

    let (get_status, _get_headers, get_body) = send(app, get_object(&cid)).await;
    assert_eq!(get_status, StatusCode::OK);
    assert_eq!(get_body, OBJECT_BYTES);

    clear_settlement_mode();
}
