//! RO:WHAT — HTTP black-box tests for svc-wallet escrow endpoints.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/RES/DX. Paid storage needs hold → capture → release over the public wallet API.
//! RO:INTERACTS — routes::router, WalletState::dev, /v1/issue, /v1/hold, /v1/capture, /v1/release, /v1/balance.
//! RO:INVARIANTS — bearer auth required; idempotent replay; nonce conflict protection; balances reflect ledger truth.
//! RO:METRICS — asserts wallet_ops_total labels include hold/capture/release and replay count increases.
//! RO:CONFIG — uses WalletState::dev with amnesia-safe in-memory ledger.
//! RO:SECURITY — sends dummy bearer token only; no real macaroons or secrets.
//! RO:TEST — cargo test -p svc-wallet --test http_escrow.

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use svc_wallet::routes::{self, WalletState};
use tower::ServiceExt;

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

async fn send(router: Router, request: Request<Body>) -> (StatusCode, Value) {
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

fn flat_error_code(body: &Value) -> &str {
    body["code"]
        .as_str()
        .expect("svc-wallet error envelope should expose flat code")
}

async fn issue(router: Router, to: &str, amount: &str, idem: &str) -> Value {
    let (status, body) = send(
        router,
        json_post_request(
            "/v1/issue",
            idem,
            json!({
                "to": to,
                "asset": "roc",
                "amount_minor": amount,
                "memo": null
            }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["op"], "issue");
    body
}

async fn balance(router: Router, account: &str) -> String {
    let path = format!("/v1/balance?account={account}&asset=roc");
    let (status, body) = send(router, get_request(&path)).await;
    assert_eq!(status, StatusCode::OK);
    body["amount_minor"]
        .as_str()
        .expect("amount_minor should be string")
        .to_string()
}

#[tokio::test]
async fn hold_capture_release_flow_works_through_http() {
    let app = app();

    issue(app.clone(), "acct_user", "100", "idem_issue_http_escrow").await;

    let hold_body = json!({
        "from": "acct_user",
        "to": "escrow_hold_1",
        "asset": "roc",
        "amount_minor": "70",
        "nonce": 1,
        "memo": "storage hold"
    });

    let (hold_status, hold_receipt) = send(
        app.clone(),
        json_post_request("/v1/hold", "idem_http_hold_1", hold_body.clone()),
    )
    .await;

    assert_eq!(hold_status, StatusCode::OK);
    assert_eq!(hold_receipt["op"], "hold");
    assert_eq!(hold_receipt["from"], "acct_user");
    assert_eq!(hold_receipt["to"], "escrow_hold_1");
    assert_eq!(hold_receipt["amount_minor"], "70");

    let (hold_replay_status, hold_replay) = send(
        app.clone(),
        json_post_request("/v1/hold", "idem_http_hold_1", hold_body),
    )
    .await;

    assert_eq!(hold_replay_status, StatusCode::OK);
    assert_eq!(hold_replay, hold_receipt);

    assert_eq!(balance(app.clone(), "acct_user").await, "30");
    assert_eq!(balance(app.clone(), "escrow_hold_1").await, "70");

    let (capture_status, capture_receipt) = send(
        app.clone(),
        json_post_request(
            "/v1/capture",
            "idem_http_capture_1",
            json!({
                "from": "escrow_hold_1",
                "to": "svc_storage",
                "asset": "roc",
                "amount_minor": "40",
                "nonce": 1,
                "memo": "storage capture"
            }),
        ),
    )
    .await;

    assert_eq!(capture_status, StatusCode::OK);
    assert_eq!(capture_receipt["op"], "capture");
    assert_eq!(capture_receipt["from"], "escrow_hold_1");
    assert_eq!(capture_receipt["to"], "svc_storage");
    assert_eq!(capture_receipt["amount_minor"], "40");

    assert_eq!(balance(app.clone(), "escrow_hold_1").await, "30");
    assert_eq!(balance(app.clone(), "svc_storage").await, "40");

    let (release_status, release_receipt) = send(
        app.clone(),
        json_post_request(
            "/v1/release",
            "idem_http_release_1",
            json!({
                "from": "escrow_hold_1",
                "to": "acct_user",
                "asset": "roc",
                "amount_minor": "30",
                "nonce": 2,
                "memo": "storage release"
            }),
        ),
    )
    .await;

    assert_eq!(release_status, StatusCode::OK);
    assert_eq!(release_receipt["op"], "release");
    assert_eq!(release_receipt["from"], "escrow_hold_1");
    assert_eq!(release_receipt["to"], "acct_user");
    assert_eq!(release_receipt["amount_minor"], "30");

    assert_eq!(balance(app.clone(), "acct_user").await, "60");
    assert_eq!(balance(app.clone(), "escrow_hold_1").await, "0");
    assert_eq!(balance(app.clone(), "svc_storage").await, "40");

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
    assert!(metrics.contains("wallet_ops_total{op=\"hold\"} 1"));
    assert!(metrics.contains("wallet_ops_total{op=\"capture\"} 1"));
    assert!(metrics.contains("wallet_ops_total{op=\"release\"} 1"));
    assert!(metrics.contains("wallet_idempotency_replays_total 1"));
}

#[tokio::test]
async fn capture_more_than_held_rejects_through_http_without_crediting_payee() {
    let app = app();

    issue(
        app.clone(),
        "acct_user",
        "50",
        "idem_issue_http_capture_reject",
    )
    .await;

    let (hold_status, _hold_receipt) = send(
        app.clone(),
        json_post_request(
            "/v1/hold",
            "idem_http_hold_capture_reject",
            json!({
                "from": "acct_user",
                "to": "escrow_hold_2",
                "asset": "roc",
                "amount_minor": "20",
                "nonce": 1,
                "memo": "storage hold"
            }),
        ),
    )
    .await;

    assert_eq!(hold_status, StatusCode::OK);

    let (capture_status, capture_error) = send(
        app.clone(),
        json_post_request(
            "/v1/capture",
            "idem_http_capture_too_much",
            json!({
                "from": "escrow_hold_2",
                "to": "svc_storage",
                "asset": "roc",
                "amount_minor": "21",
                "nonce": 1,
                "memo": "storage capture too much"
            }),
        ),
    )
    .await;

    assert_eq!(capture_status, StatusCode::CONFLICT);
    assert_eq!(flat_error_code(&capture_error), "INSUFFICIENT_FUNDS");

    assert_eq!(balance(app.clone(), "acct_user").await, "30");
    assert_eq!(balance(app.clone(), "escrow_hold_2").await, "20");
    assert_eq!(balance(app.clone(), "svc_storage").await, "0");
}

#[tokio::test]
async fn hold_nonce_replay_rejects_through_http() {
    let app = app();

    issue(app.clone(), "acct_user", "100", "idem_issue_nonce_hold").await;

    let first_hold = json!({
        "from": "acct_user",
        "to": "escrow_hold_3",
        "asset": "roc",
        "amount_minor": "20",
        "nonce": 1,
        "memo": "first hold"
    });

    let (first_status, _first_receipt) = send(
        app.clone(),
        json_post_request("/v1/hold", "idem_http_hold_nonce_1", first_hold),
    )
    .await;

    assert_eq!(first_status, StatusCode::OK);

    let replayed_nonce_hold = json!({
        "from": "acct_user",
        "to": "escrow_hold_4",
        "asset": "roc",
        "amount_minor": "10",
        "nonce": 1,
        "memo": "bad duplicate nonce"
    });

    let (second_status, second_body) = send(
        app.clone(),
        json_post_request("/v1/hold", "idem_http_hold_nonce_2", replayed_nonce_hold),
    )
    .await;

    assert_eq!(second_status, StatusCode::CONFLICT);
    assert_eq!(flat_error_code(&second_body), "NONCE_CONFLICT");
    assert_eq!(balance(app.clone(), "acct_user").await, "80");
    assert_eq!(balance(app.clone(), "escrow_hold_4").await, "0");
}
