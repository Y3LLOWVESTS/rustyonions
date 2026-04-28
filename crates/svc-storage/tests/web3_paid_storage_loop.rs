//! RO:WHAT — Cross-crate paid-storage smoke using real svc-wallet, svc-storage, and ron-accounting APIs.
//! RO:WHY — Pillar 12, Concerns ECON/RES/DX; proves ROC escrow gates CAS write and emits accounting usage.
//! RO:INTERACTS — svc_wallet::routes, svc_storage::http::server, ron_accounting::record_usage_events.
//! RO:INVARIANTS — unpaid write rejects; paid write emits bytes_stored/request_ok/pin_seconds; settlement conserves value.
//! RO:METRICS — wallet route metrics are exercised indirectly; accounting ingest is proven in-process.
//! RO:CONFIG — in-process amnesia-safe wallet/storage state only.
//! RO:SECURITY — dev-only Bearer token; no real macaroon, key, private content, or external chain.
//! RO:TEST — cargo test -p svc-storage --test web3_paid_storage_loop.

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body, Bytes},
    http::{header, HeaderMap, Method, Request, StatusCode},
    Router,
};
use ron_accounting::{
    record_usage_events, Dimension, EventIngestPolicy, MetricKind, Recorder, UsageEvent,
};
use serde_json::{json, Value};
use svc_storage::{
    http::{extractors::AppState, server::build_router},
    storage::{MemoryStorage, Storage},
};
use svc_wallet::routes::{self as wallet_routes, WalletState};
use tower::ServiceExt;

const PAYER: &str = "acct_storage_user";
const UNFUNDED_PAYER: &str = "acct_storage_unfunded";
const ESCROW: &str = "escrow_storage_paid_write_1";
const UNFUNDED_ESCROW: &str = "escrow_storage_unfunded_write_1";
const STORAGE_TREASURY: &str = "svc_storage";
const ASSET: &str = "roc";
const OBJECT_BYTES: &[u8] = b"hello paid storage from svc-storage";
const TENANT: u128 = 7;
const ACCOUNTING_SUBJECT: &str = "svc_storage_provider";
const REGION: &str = "us-central";
const PIN_SECONDS: u64 = 60;

fn storage_app() -> Router {
    let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
    let state = AppState { store };

    build_router().with_state(state)
}

fn wallet_app() -> Router {
    let state = WalletState::dev().expect("dev wallet state should build");
    wallet_routes::router(state)
}

fn expected_cid(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

fn wallet_get(path: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(path)
        .header(header::AUTHORIZATION, "Bearer dev")
        .body(Body::empty())
        .expect("wallet GET request should build")
}

fn wallet_post(path: &str, idem: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::AUTHORIZATION, "Bearer dev")
        .header(header::CONTENT_TYPE, "application/json")
        .header("Idempotency-Key", idem)
        .body(Body::from(body.to_string()))
        .expect("wallet POST request should build")
}

fn storage_paid_post(path: &str, hold_receipt: &Value, body: &'static [u8]) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header("x-ron-paid-op", "hold")
        .header("x-ron-paid-asset", ASSET)
        .header(
            "x-ron-paid-estimate-minor",
            hold_receipt["amount_minor"].as_str().unwrap(),
        )
        .header("x-ron-wallet-txid", hold_receipt["txid"].as_str().unwrap())
        .header(
            "x-ron-wallet-receipt-hash",
            hold_receipt["receipt_hash"].as_str().unwrap(),
        )
        .header("x-ron-wallet-from", hold_receipt["from"].as_str().unwrap())
        .header("x-ron-wallet-to", hold_receipt["to"].as_str().unwrap())
        .header("x-ron-tenant", TENANT.to_string())
        .header("x-ron-accounting-subject", ACCOUNTING_SUBJECT)
        .header("x-ron-region", REGION)
        .header("x-ron-pin-seconds", PIN_SECONDS.to_string())
        .body(Body::from(body))
        .expect("storage paid POST request should build")
}

fn storage_paid_post_without_proof(path: &str, body: &'static [u8]) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(Body::from(body))
        .expect("storage paid POST request should build")
}

fn storage_get(path: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(path)
        .body(Body::empty())
        .expect("storage GET request should build")
}

fn storage_head(path: &str) -> Request<Body> {
    Request::builder()
        .method(Method::HEAD)
        .uri(path)
        .body(Body::empty())
        .expect("storage HEAD request should build")
}

async fn send_json(router: Router, req: Request<Body>) -> (StatusCode, HeaderMap, Value) {
    let response = router
        .oneshot(req)
        .await
        .expect("router request should complete");

    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read");
    let value = serde_json::from_slice::<Value>(&bytes).expect("response body should be JSON");

    (status, headers, value)
}

async fn send_bytes(router: Router, req: Request<Body>) -> (StatusCode, HeaderMap, Bytes) {
    let response = router
        .oneshot(req)
        .await
        .expect("router request should complete");

    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read");

    (status, headers, bytes)
}

async fn issue(wallet: Router, account: &str, amount: &str, idem: &str) -> Value {
    let (status, _headers, body) = send_json(
        wallet,
        wallet_post(
            "/v1/issue",
            idem,
            json!({
                "to": account,
                "asset": ASSET,
                "amount_minor": amount,
                "memo": "fund svc-storage paid write"
            }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["op"], "issue");
    assert_eq!(body["to"], account);
    assert_eq!(body["amount_minor"], amount);
    body
}

async fn balance(wallet: Router, account: &str) -> String {
    let path = format!("/v1/balance?account={account}&asset={ASSET}");
    let (status, _headers, body) = send_json(wallet, wallet_get(&path)).await;

    assert_eq!(status, StatusCode::OK);
    body["amount_minor"]
        .as_str()
        .expect("wallet balance amount should be string")
        .to_string()
}

async fn hold(
    wallet: Router,
    from: &str,
    to: &str,
    amount: &str,
    nonce: u64,
    idem: &str,
) -> (StatusCode, Value) {
    let (status, _headers, body) = send_json(
        wallet,
        wallet_post(
            "/v1/hold",
            idem,
            json!({
                "from": from,
                "to": to,
                "asset": ASSET,
                "amount_minor": amount,
                "nonce": nonce,
                "memo": "svc-storage paid write hold"
            }),
        ),
    )
    .await;

    (status, body)
}

async fn capture(
    wallet: Router,
    from: &str,
    to: &str,
    amount: &str,
    nonce: u64,
    idem: &str,
) -> (StatusCode, Value) {
    let (status, _headers, body) = send_json(
        wallet,
        wallet_post(
            "/v1/capture",
            idem,
            json!({
                "from": from,
                "to": to,
                "asset": ASSET,
                "amount_minor": amount,
                "nonce": nonce,
                "memo": "svc-storage paid write capture"
            }),
        ),
    )
    .await;

    (status, body)
}

async fn release(
    wallet: Router,
    from: &str,
    to: &str,
    amount: &str,
    nonce: u64,
    idem: &str,
) -> (StatusCode, Value) {
    let (status, _headers, body) = send_json(
        wallet,
        wallet_post(
            "/v1/release",
            idem,
            json!({
                "from": from,
                "to": to,
                "asset": ASSET,
                "amount_minor": amount,
                "nonce": nonce,
                "memo": "svc-storage paid write release"
            }),
        ),
    )
    .await;

    (status, body)
}

fn assert_accounting_usage_events_are_ingestable(post_json: &Value) {
    let usage_events = serde_json::from_value::<Vec<UsageEvent>>(post_json["usage_events"].clone())
        .expect("paid storage response usage_events should parse as ron-accounting UsageEvent");

    assert_eq!(usage_events.len(), 3);

    assert!(usage_events.iter().any(|event| {
        event.tenant == TENANT
            && event.subject == ACCOUNTING_SUBJECT
            && event.metric_kind == MetricKind::BytesStored
            && event.value == OBJECT_BYTES.len() as u64
            && event.source_service.as_deref() == Some("svc-storage")
            && event.region.as_deref() == Some(REGION)
            && event.route.as_deref() == Some("/paid/o")
    }));

    assert!(usage_events.iter().any(|event| {
        event.tenant == TENANT
            && event.subject == ACCOUNTING_SUBJECT
            && event.metric_kind == MetricKind::RequestOk
            && event.value == 1
    }));

    assert!(usage_events.iter().any(|event| {
        event.tenant == TENANT
            && event.subject == ACCOUNTING_SUBJECT
            && event.metric_kind == MetricKind::PinSeconds
            && event.value == PIN_SECONDS
    }));

    let recorder = Recorder::default();
    let report = record_usage_events(&recorder, &usage_events, &EventIngestPolicy::default())
        .expect("ron-accounting should ingest paid storage usage events");

    assert_eq!(report.inspected, 3);
    assert_eq!(report.recorded, 3);
    assert_eq!(report.skipped_zero, 0);

    let rows = recorder.snapshot();
    assert_eq!(rows.len(), 3);

    assert!(rows.iter().any(|row| {
        row.key.labels.tenant == TENANT
            && row.key.labels.service == ACCOUNTING_SUBJECT
            && row.key.labels.region == REGION
            && row.key.labels.method == "PUT"
            && row.key.labels.route == "/paid/o"
            && row.key.dimension == Dimension::Bytes
            && row.value == OBJECT_BYTES.len() as u64
    }));

    assert!(rows.iter().any(|row| {
        row.key.labels.tenant == TENANT
            && row.key.labels.service == ACCOUNTING_SUBJECT
            && row.key.labels.method == "REQ_OK"
            && row.key.dimension == Dimension::Requests
            && row.value == 1
    }));

    assert!(rows.iter().any(|row| {
        row.key.labels.tenant == TENANT
            && row.key.labels.service == ACCOUNTING_SUBJECT
            && row.key.labels.method == "PIN_SECONDS"
            && row.key.dimension == Dimension::Requests
            && row.value == PIN_SECONDS
    }));
}

#[tokio::test]
async fn paid_storage_loop_rejects_unfunded_then_stores_after_wallet_hold() {
    let wallet = wallet_app();
    let storage = storage_app();
    let cid = expected_cid(OBJECT_BYTES);
    let object_path = format!("/o/{cid}");

    let (unfunded_hold_status, unfunded_hold_error) = hold(
        wallet.clone(),
        UNFUNDED_PAYER,
        UNFUNDED_ESCROW,
        "70",
        1,
        "idem_storage_unfunded_hold",
    )
    .await;

    assert_eq!(unfunded_hold_status, StatusCode::CONFLICT);
    assert_eq!(unfunded_hold_error["code"], "INSUFFICIENT_FUNDS");

    let (unpaid_write_status, _unpaid_write_headers, unpaid_write_body) = send_bytes(
        storage.clone(),
        storage_paid_post_without_proof("/paid/o", OBJECT_BYTES),
    )
    .await;

    assert_eq!(unpaid_write_status, StatusCode::PAYMENT_REQUIRED);

    let unpaid_write_error: Value =
        serde_json::from_slice(&unpaid_write_body).expect("paid reject should be JSON");
    assert_eq!(unpaid_write_error["error"], "payment_required");

    let (unwritten_status, _unwritten_headers, _unwritten_body) =
        send_bytes(storage.clone(), storage_head(&object_path)).await;

    assert_eq!(
        unwritten_status,
        StatusCode::NOT_FOUND,
        "unfunded paid write must not create the object"
    );

    issue(
        wallet.clone(),
        PAYER,
        "100",
        "idem_storage_paid_write_issue",
    )
    .await;

    assert_eq!(balance(wallet.clone(), PAYER).await, "100");

    let (hold_status, hold_receipt) = hold(
        wallet.clone(),
        PAYER,
        ESCROW,
        "70",
        1,
        "idem_storage_paid_write_hold",
    )
    .await;

    assert_eq!(hold_status, StatusCode::OK);
    assert_eq!(hold_receipt["op"], "hold");
    assert_eq!(hold_receipt["from"], PAYER);
    assert_eq!(hold_receipt["to"], ESCROW);
    assert_eq!(hold_receipt["amount_minor"], "70");
    assert!(hold_receipt["receipt_hash"]
        .as_str()
        .unwrap()
        .starts_with("b3:"));

    assert_eq!(balance(wallet.clone(), PAYER).await, "30");
    assert_eq!(balance(wallet.clone(), ESCROW).await, "70");

    let (post_status, _post_headers, post_body) = send_bytes(
        storage.clone(),
        storage_paid_post("/paid/o", &hold_receipt, OBJECT_BYTES),
    )
    .await;

    assert_eq!(post_status, StatusCode::OK);

    let post_json: Value =
        serde_json::from_slice(&post_body).expect("storage paid POST response should be JSON");

    assert_eq!(post_json["cid"], cid);
    assert_eq!(post_json["paid"], true);
    assert_eq!(post_json["payer"], PAYER);
    assert_eq!(post_json["escrow"], ESCROW);
    assert_eq!(
        post_json["wallet_receipt_hash"],
        hold_receipt["receipt_hash"]
    );
    assert_eq!(post_json["estimate_minor"], "70");

    assert_accounting_usage_events_are_ingestable(&post_json);

    let (get_status, get_headers, get_body) =
        send_bytes(storage.clone(), storage_get(&object_path)).await;

    assert_eq!(get_status, StatusCode::OK);
    assert_eq!(get_body.as_ref(), OBJECT_BYTES);
    assert_eq!(
        get_headers
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok()),
        Some(OBJECT_BYTES.len().to_string().as_str())
    );

    let (capture_status, capture_receipt) = capture(
        wallet.clone(),
        ESCROW,
        STORAGE_TREASURY,
        "40",
        1,
        "idem_storage_paid_write_capture",
    )
    .await;

    assert_eq!(capture_status, StatusCode::OK);
    assert_eq!(capture_receipt["op"], "capture");
    assert_eq!(capture_receipt["from"], ESCROW);
    assert_eq!(capture_receipt["to"], STORAGE_TREASURY);
    assert_eq!(capture_receipt["amount_minor"], "40");

    let (release_status, release_receipt) = release(
        wallet.clone(),
        ESCROW,
        PAYER,
        "30",
        2,
        "idem_storage_paid_write_release",
    )
    .await;

    assert_eq!(release_status, StatusCode::OK);
    assert_eq!(release_receipt["op"], "release");
    assert_eq!(release_receipt["from"], ESCROW);
    assert_eq!(release_receipt["to"], PAYER);
    assert_eq!(release_receipt["amount_minor"], "30");

    assert_eq!(balance(wallet.clone(), PAYER).await, "60");
    assert_eq!(balance(wallet.clone(), ESCROW).await, "0");
    assert_eq!(balance(wallet.clone(), STORAGE_TREASURY).await, "40");

    let final_total = balance(wallet.clone(), PAYER)
        .await
        .parse::<u128>()
        .expect("payer balance parses")
        + balance(wallet.clone(), ESCROW)
            .await
            .parse::<u128>()
            .expect("escrow balance parses")
        + balance(wallet.clone(), STORAGE_TREASURY)
            .await
            .parse::<u128>()
            .expect("storage balance parses");

    assert_eq!(final_total, 100);
}
