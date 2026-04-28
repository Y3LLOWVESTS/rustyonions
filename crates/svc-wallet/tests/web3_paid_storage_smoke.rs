//! RO:WHAT — WEB3 paid-storage state-machine smoke using svc-wallet escrow HTTP endpoints.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES/DX. Proves pay-for-storage admission before wiring real svc-storage.
//! RO:INTERACTS — /v1/issue, /v1/hold, /v1/capture, /v1/release, /v1/balance, /metrics.
//! RO:INVARIANTS — zero-ROC payer rejects; funded payer reserves; capture pays storage; release refunds remainder.
//! RO:METRICS — verifies hold/capture/release op counters are visible.
//! RO:CONFIG — uses WalletState::dev with in-memory amnesia-safe ledger.
//! RO:SECURITY — dev-only Bearer token; no real macaroon, private content, or PII.
//! RO:TEST — cargo test -p svc-wallet --test web3_paid_storage_smoke.

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use svc_wallet::routes::{self, WalletState};
use tower::ServiceExt;

const PAYER: &str = "acct_paid_storage_user";
const UNFUNDED_PAYER: &str = "acct_unfunded_storage_user";
const ESCROW: &str = "escrow_paid_storage_1";
const STORAGE_TREASURY: &str = "svc_storage";
const ASSET: &str = "roc";

#[derive(Debug, Clone, PartialEq, Eq)]
struct FakeStorageReceipt {
    cid: String,
    bytes_stored: usize,
}

fn app() -> Router {
    let state = WalletState::dev().expect("dev wallet state should build");
    routes::router(state)
}

fn get_request(path: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(path)
        .header(header::AUTHORIZATION, "Bearer dev")
        .body(Body::empty())
        .expect("GET request should build")
}

fn post_json(path: &str, idem: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::AUTHORIZATION, "Bearer dev")
        .header(header::CONTENT_TYPE, "application/json")
        .header("Idempotency-Key", idem)
        .body(Body::from(body.to_string()))
        .expect("POST request should build")
}

async fn send_json(router: Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router
        .oneshot(request)
        .await
        .expect("router request should complete");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read");
    let value = serde_json::from_slice::<Value>(&bytes).expect("response body should be JSON");

    (status, value)
}

async fn send_text(router: Router, request: Request<Body>) -> (StatusCode, String) {
    let response = router
        .oneshot(request)
        .await
        .expect("router request should complete");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read");
    let text = String::from_utf8(bytes.to_vec()).expect("response body should be UTF-8");

    (status, text)
}

async fn issue(router: Router, to: &str, amount: &str, idem: &str) -> Value {
    let (status, body) = send_json(
        router,
        post_json(
            "/v1/issue",
            idem,
            json!({
                "to": to,
                "asset": ASSET,
                "amount_minor": amount,
                "memo": "fund paid storage smoke"
            }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["op"], "issue");
    assert_eq!(body["to"], to);
    assert_eq!(body["amount_minor"], amount);
    body
}

async fn hold(
    router: Router,
    payer: &str,
    escrow: &str,
    amount: &str,
    nonce: u64,
    idem: &str,
) -> (StatusCode, Value) {
    send_json(
        router,
        post_json(
            "/v1/hold",
            idem,
            json!({
                "from": payer,
                "to": escrow,
                "asset": ASSET,
                "amount_minor": amount,
                "nonce": nonce,
                "memo": "paid storage estimate hold"
            }),
        ),
    )
    .await
}

async fn capture(
    router: Router,
    escrow: &str,
    payee: &str,
    amount: &str,
    nonce: u64,
    idem: &str,
) -> (StatusCode, Value) {
    send_json(
        router,
        post_json(
            "/v1/capture",
            idem,
            json!({
                "from": escrow,
                "to": payee,
                "asset": ASSET,
                "amount_minor": amount,
                "nonce": nonce,
                "memo": "paid storage actual capture"
            }),
        ),
    )
    .await
}

async fn release(
    router: Router,
    escrow: &str,
    payer: &str,
    amount: &str,
    nonce: u64,
    idem: &str,
) -> (StatusCode, Value) {
    send_json(
        router,
        post_json(
            "/v1/release",
            idem,
            json!({
                "from": escrow,
                "to": payer,
                "asset": ASSET,
                "amount_minor": amount,
                "nonce": nonce,
                "memo": "paid storage remainder release"
            }),
        ),
    )
    .await
}

async fn balance(router: Router, account: &str) -> String {
    let path = format!("/v1/balance?account={account}&asset={ASSET}");
    let (status, body) = send_json(router, get_request(&path)).await;

    assert_eq!(status, StatusCode::OK);
    body["amount_minor"]
        .as_str()
        .expect("amount_minor should be string")
        .to_string()
}

fn fake_storage_write(bytes: &[u8]) -> FakeStorageReceipt {
    FakeStorageReceipt {
        cid: format!("b3:{}", blake3::hash(bytes).to_hex()),
        bytes_stored: bytes.len(),
    }
}

#[tokio::test]
async fn paid_storage_state_machine_rejects_unfunded_and_charges_funded_payer() {
    let app = app();

    let (unfunded_status, unfunded_error) = hold(
        app.clone(),
        UNFUNDED_PAYER,
        "escrow_unfunded_paid_storage",
        "70",
        1,
        "idem_paid_storage_unfunded_hold",
    )
    .await;

    assert_eq!(unfunded_status, StatusCode::CONFLICT);
    assert_eq!(unfunded_error["code"], "INSUFFICIENT_FUNDS");
    assert_eq!(balance(app.clone(), UNFUNDED_PAYER).await, "0");
    assert_eq!(
        balance(app.clone(), "escrow_unfunded_paid_storage").await,
        "0"
    );

    issue(app.clone(), PAYER, "100", "idem_paid_storage_fund_user").await;

    assert_eq!(balance(app.clone(), PAYER).await, "100");

    let (hold_status, hold_receipt) = hold(
        app.clone(),
        PAYER,
        ESCROW,
        "70",
        1,
        "idem_paid_storage_hold",
    )
    .await;

    assert_eq!(hold_status, StatusCode::OK);
    assert_eq!(hold_receipt["op"], "hold");
    assert_eq!(hold_receipt["from"], PAYER);
    assert_eq!(hold_receipt["to"], ESCROW);
    assert_eq!(hold_receipt["amount_minor"], "70");

    assert_eq!(balance(app.clone(), PAYER).await, "30");
    assert_eq!(balance(app.clone(), ESCROW).await, "70");

    let object = fake_storage_write(b"hello paid storage");
    assert!(object.cid.starts_with("b3:"));
    assert_eq!(object.bytes_stored, 18);

    let (capture_status, capture_receipt) = capture(
        app.clone(),
        ESCROW,
        STORAGE_TREASURY,
        "40",
        1,
        "idem_paid_storage_capture",
    )
    .await;

    assert_eq!(capture_status, StatusCode::OK);
    assert_eq!(capture_receipt["op"], "capture");
    assert_eq!(capture_receipt["from"], ESCROW);
    assert_eq!(capture_receipt["to"], STORAGE_TREASURY);
    assert_eq!(capture_receipt["amount_minor"], "40");

    assert_eq!(balance(app.clone(), ESCROW).await, "30");
    assert_eq!(balance(app.clone(), STORAGE_TREASURY).await, "40");

    let (release_status, release_receipt) = release(
        app.clone(),
        ESCROW,
        PAYER,
        "30",
        2,
        "idem_paid_storage_release",
    )
    .await;

    assert_eq!(release_status, StatusCode::OK);
    assert_eq!(release_receipt["op"], "release");
    assert_eq!(release_receipt["from"], ESCROW);
    assert_eq!(release_receipt["to"], PAYER);
    assert_eq!(release_receipt["amount_minor"], "30");

    assert_eq!(balance(app.clone(), PAYER).await, "60");
    assert_eq!(balance(app.clone(), ESCROW).await, "0");
    assert_eq!(balance(app.clone(), STORAGE_TREASURY).await, "40");

    let final_total = balance(app.clone(), PAYER)
        .await
        .parse::<u128>()
        .expect("payer balance parses")
        + balance(app.clone(), ESCROW)
            .await
            .parse::<u128>()
            .expect("escrow balance parses")
        + balance(app.clone(), STORAGE_TREASURY)
            .await
            .parse::<u128>()
            .expect("storage balance parses");

    assert_eq!(final_total, 100);

    let (metrics_status, metrics) = send_text(
        app.clone(),
        Request::builder()
            .method(Method::GET)
            .uri("/metrics")
            .body(Body::empty())
            .expect("metrics request should build"),
    )
    .await;

    assert_eq!(metrics_status, StatusCode::OK);
    assert!(metrics.contains("wallet_ops_total{op=\"issue\"} 1"));
    assert!(metrics.contains("wallet_ops_total{op=\"hold\"} 1"));
    assert!(metrics.contains("wallet_ops_total{op=\"capture\"} 1"));
    assert!(metrics.contains("wallet_ops_total{op=\"release\"} 1"));
    assert!(metrics.contains("wallet_rejects_total{reason=\"INSUFFICIENT_FUNDS\"} 1"));
}
