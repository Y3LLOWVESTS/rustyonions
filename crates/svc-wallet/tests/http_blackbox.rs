//! RO:WHAT — HTTP black-box tests for svc-wallet’s actual Axum router.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/RES/DX. Locks the service-facing API contract, not just module seams.
//! RO:INTERACTS — routes::router, WalletState::dev, /healthz, /readyz, /metrics, /v1 wallet routes.
//! RO:INVARIANTS — bearer auth required; POST idempotency; nonce conflict; receipt lookup; balances reflect ledger truth.
//! RO:METRICS — asserts metrics endpoint renders wallet_* series after successes, replays, and rejects.
//! RO:CONFIG — uses WalletState::dev with amnesia-safe in-memory ledger.
//! RO:SECURITY — sends dummy bearer token only; verifies missing Authorization rejects.
//! RO:TEST — cargo test -p svc-wallet --test http_blackbox.

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

fn get_request(path: &str, bearer: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(Method::GET).uri(path);

    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }

    builder
        .body(Body::empty())
        .expect("GET request should build")
}

fn json_post_request(
    path: &str,
    bearer: Option<&str>,
    idempotency_key: Option<&str>,
    body: Value,
) -> Request<Body> {
    let encoded = serde_json::to_vec(&body).expect("JSON body should encode");
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }

    if let Some(idem) = idempotency_key {
        builder = builder.header("Idempotency-Key", idem);
    }

    builder
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

fn text_body(body: Vec<u8>) -> String {
    String::from_utf8(body).expect("response should be valid UTF-8")
}

#[tokio::test]
async fn health_ready_and_metrics_endpoints_are_live() {
    let router = app();

    let (health_status, health_body) = send(router.clone(), get_request("/healthz", None)).await;
    assert_eq!(health_status, StatusCode::OK);
    assert_eq!(json_body(&health_body)["service"], "svc-wallet");
    assert_eq!(json_body(&health_body)["ok"], true);

    let (ready_status, ready_body) = send(router.clone(), get_request("/readyz", None)).await;
    assert_eq!(ready_status, StatusCode::OK);
    assert_eq!(json_body(&ready_body)["ready"], true);
    assert_eq!(json_body(&ready_body)["shed_writes"], false);

    let (metrics_status, metrics_body) = send(router, get_request("/metrics", None)).await;
    assert_eq!(metrics_status, StatusCode::OK);

    let metrics_text = text_body(metrics_body);
    assert!(metrics_text.contains("wallet_requests_total"));
    assert!(metrics_text.contains("wallet_successes_total"));
    assert!(metrics_text.contains("wallet_ready"));
}

#[tokio::test]
async fn issue_transfer_balance_and_receipt_lookup_flow_through_http() {
    let router = app();

    let (initial_status, initial_body) = send(
        router.clone(),
        get_request("/v1/balance?account=acct_a&asset=roc", Some("dev")),
    )
    .await;
    assert_eq!(initial_status, StatusCode::OK);
    assert_eq!(json_body(&initial_body)["amount_minor"], "0");

    let issue_body = json!({
        "to": "acct_a",
        "asset": "roc",
        "amount_minor": "100",
        "memo": null
    });
    let (issue_status, issue_bytes) = send(
        router.clone(),
        json_post_request(
            "/v1/issue",
            Some("dev"),
            Some("idem_http_issue_1"),
            issue_body,
        ),
    )
    .await;
    assert_eq!(issue_status, StatusCode::OK);

    let issue_receipt = json_body(&issue_bytes);
    assert_eq!(issue_receipt["op"], "issue");
    assert_eq!(issue_receipt["to"], "acct_a");
    assert_eq!(issue_receipt["amount_minor"], "100");
    assert!(issue_receipt["receipt_hash"]
        .as_str()
        .expect("receipt_hash should be a string")
        .starts_with("b3:"));

    let transfer_body = json!({
        "from": "acct_a",
        "to": "acct_b",
        "asset": "roc",
        "amount_minor": "40",
        "nonce": 1,
        "memo": null
    });
    let (transfer_status, transfer_bytes) = send(
        router.clone(),
        json_post_request(
            "/v1/transfer",
            Some("dev"),
            Some("idem_http_transfer_1"),
            transfer_body,
        ),
    )
    .await;
    assert_eq!(transfer_status, StatusCode::OK);

    let transfer_receipt = json_body(&transfer_bytes);
    assert_eq!(transfer_receipt["op"], "transfer");
    assert_eq!(transfer_receipt["from"], "acct_a");
    assert_eq!(transfer_receipt["to"], "acct_b");
    assert_eq!(transfer_receipt["amount_minor"], "40");
    assert_eq!(transfer_receipt["nonce"], 1);

    let transfer_txid = transfer_receipt["txid"]
        .as_str()
        .expect("transfer txid should be a string");

    let (receipt_status, receipt_bytes) = send(
        router.clone(),
        get_request(&format!("/v1/tx/{transfer_txid}"), Some("dev")),
    )
    .await;
    assert_eq!(receipt_status, StatusCode::OK);
    assert_eq!(json_body(&receipt_bytes), transfer_receipt);

    let (acct_a_status, acct_a_body) = send(
        router.clone(),
        get_request("/v1/balance?account=acct_a&asset=roc", Some("dev")),
    )
    .await;
    assert_eq!(acct_a_status, StatusCode::OK);
    assert_eq!(json_body(&acct_a_body)["amount_minor"], "60");

    let (acct_b_status, acct_b_body) = send(
        router,
        get_request("/v1/balance?account=acct_b&asset=roc", Some("dev")),
    )
    .await;
    assert_eq!(acct_b_status, StatusCode::OK);
    assert_eq!(json_body(&acct_b_body)["amount_minor"], "40");
}

#[tokio::test]
async fn burn_route_reduces_balance_and_receipt_lookup_works() {
    let router = app();

    let issue_body = json!({
        "to": "acct_a",
        "asset": "roc",
        "amount_minor": "100",
        "memo": null
    });
    let (issue_status, _) = send(
        router.clone(),
        json_post_request(
            "/v1/issue",
            Some("dev"),
            Some("idem_http_burn_issue"),
            issue_body,
        ),
    )
    .await;
    assert_eq!(issue_status, StatusCode::OK);

    let burn_body = json!({
        "from": "acct_a",
        "asset": "roc",
        "amount_minor": "30",
        "nonce": 1,
        "memo": null
    });
    let (burn_status, burn_bytes) = send(
        router.clone(),
        json_post_request("/v1/burn", Some("dev"), Some("idem_http_burn_1"), burn_body),
    )
    .await;
    assert_eq!(burn_status, StatusCode::OK);

    let burn_receipt = json_body(&burn_bytes);
    assert_eq!(burn_receipt["op"], "burn");
    assert_eq!(burn_receipt["from"], "acct_a");
    assert_eq!(burn_receipt["to"], Value::Null);
    assert_eq!(burn_receipt["amount_minor"], "30");
    assert_eq!(burn_receipt["nonce"], 1);

    let burn_txid = burn_receipt["txid"]
        .as_str()
        .expect("burn txid should be a string");

    let (receipt_status, receipt_bytes) = send(
        router.clone(),
        get_request(&format!("/v1/tx/{burn_txid}"), Some("dev")),
    )
    .await;
    assert_eq!(receipt_status, StatusCode::OK);
    assert_eq!(json_body(&receipt_bytes), burn_receipt);

    let (balance_status, balance_body) = send(
        router,
        get_request("/v1/balance?account=acct_a&asset=roc", Some("dev")),
    )
    .await;
    assert_eq!(balance_status, StatusCode::OK);
    assert_eq!(json_body(&balance_body)["amount_minor"], "70");
}

#[tokio::test]
async fn idempotent_issue_replay_returns_same_receipt_bytes() {
    let router = app();

    let issue_body = json!({
        "to": "acct_a",
        "asset": "roc",
        "amount_minor": "100",
        "memo": null
    });

    let (first_status, first_body) = send(
        router.clone(),
        json_post_request(
            "/v1/issue",
            Some("dev"),
            Some("idem_http_replay_issue"),
            issue_body.clone(),
        ),
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);

    let (second_status, second_body) = send(
        router,
        json_post_request(
            "/v1/issue",
            Some("dev"),
            Some("idem_http_replay_issue"),
            issue_body,
        ),
    )
    .await;
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(first_body, second_body);
}

#[tokio::test]
async fn same_idempotency_key_with_different_body_rejects() {
    let router = app();

    let first_body = json!({
        "to": "acct_a",
        "asset": "roc",
        "amount_minor": "100",
        "memo": null
    });
    let second_body = json!({
        "to": "acct_a",
        "asset": "roc",
        "amount_minor": "101",
        "memo": null
    });

    let (first_status, _) = send(
        router.clone(),
        json_post_request(
            "/v1/issue",
            Some("dev"),
            Some("idem_http_conflict_issue"),
            first_body,
        ),
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);

    let (second_status, second_bytes) = send(
        router,
        json_post_request(
            "/v1/issue",
            Some("dev"),
            Some("idem_http_conflict_issue"),
            second_body,
        ),
    )
    .await;
    assert_eq!(second_status, StatusCode::CONFLICT);

    let error = json_body(&second_bytes);
    assert_eq!(error["code"], "IDEMPOTENCY_CONFLICT");
    assert_eq!(error["http"], 409);
}

#[tokio::test]
async fn nonce_replay_rejects_through_transfer_route() {
    let router = app();

    let issue_body = json!({
        "to": "acct_a",
        "asset": "roc",
        "amount_minor": "100",
        "memo": null
    });
    let (issue_status, _) = send(
        router.clone(),
        json_post_request(
            "/v1/issue",
            Some("dev"),
            Some("idem_http_nonce_issue"),
            issue_body,
        ),
    )
    .await;
    assert_eq!(issue_status, StatusCode::OK);

    let transfer_body = json!({
        "from": "acct_a",
        "to": "acct_b",
        "asset": "roc",
        "amount_minor": "40",
        "nonce": 1,
        "memo": null
    });
    let (first_status, _) = send(
        router.clone(),
        json_post_request(
            "/v1/transfer",
            Some("dev"),
            Some("idem_http_nonce_transfer_1"),
            transfer_body.clone(),
        ),
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);

    let (second_status, second_bytes) = send(
        router,
        json_post_request(
            "/v1/transfer",
            Some("dev"),
            Some("idem_http_nonce_transfer_2"),
            transfer_body,
        ),
    )
    .await;
    assert_eq!(second_status, StatusCode::CONFLICT);

    let error = json_body(&second_bytes);
    assert_eq!(error["code"], "NONCE_CONFLICT");
    assert_eq!(error["http"], 409);
}

#[tokio::test]
async fn metrics_reflect_success_replay_reject_and_operation_counts() {
    let router = app();

    let issue_body = json!({
        "to": "acct_a",
        "asset": "roc",
        "amount_minor": "100",
        "memo": null
    });

    let (first_status, _) = send(
        router.clone(),
        json_post_request(
            "/v1/issue",
            Some("dev"),
            Some("idem_http_metrics_issue"),
            issue_body.clone(),
        ),
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);

    let (replay_status, _) = send(
        router.clone(),
        json_post_request(
            "/v1/issue",
            Some("dev"),
            Some("idem_http_metrics_issue"),
            issue_body,
        ),
    )
    .await;
    assert_eq!(replay_status, StatusCode::OK);

    let conflict_body = json!({
        "to": "acct_a",
        "asset": "roc",
        "amount_minor": "101",
        "memo": null
    });
    let (conflict_status, _) = send(
        router.clone(),
        json_post_request(
            "/v1/issue",
            Some("dev"),
            Some("idem_http_metrics_issue"),
            conflict_body,
        ),
    )
    .await;
    assert_eq!(conflict_status, StatusCode::CONFLICT);

    let burn_body = json!({
        "from": "acct_a",
        "asset": "roc",
        "amount_minor": "25",
        "nonce": 1,
        "memo": null
    });
    let (burn_status, _) = send(
        router.clone(),
        json_post_request(
            "/v1/burn",
            Some("dev"),
            Some("idem_http_metrics_burn"),
            burn_body,
        ),
    )
    .await;
    assert_eq!(burn_status, StatusCode::OK);

    let (metrics_status, metrics_body) = send(router, get_request("/metrics", None)).await;
    assert_eq!(metrics_status, StatusCode::OK);

    let metrics = text_body(metrics_body);
    assert!(metrics.contains("wallet_idempotency_replays_total 1"));
    assert!(metrics.contains("wallet_ops_total{op=\"issue\"} 1"));
    assert!(metrics.contains("wallet_ops_total{op=\"burn\"} 1"));
    assert!(metrics.contains("wallet_rejects_total{reason=\"IDEMPOTENCY_CONFLICT\"} 1"));
    assert!(metrics.contains("wallet_ready 1"));
}

#[tokio::test]
async fn missing_authorization_rejects_v1_route() {
    let router = app();

    let (status, body) = send(
        router,
        get_request("/v1/balance?account=acct_a&asset=roc", None),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let error = json_body(&body);
    assert_eq!(error["code"], "UNAUTHORIZED");
    assert_eq!(error["http"], 401);
}

#[tokio::test]
async fn unknown_receipt_returns_not_found() {
    let router = app();

    let (status, body) = send(router, get_request("/v1/tx/tx_does_not_exist", Some("dev"))).await;

    assert_eq!(status, StatusCode::NOT_FOUND);

    let error = json_body(&body);
    assert_eq!(error["code"], "NOT_FOUND");
    assert_eq!(error["http"], 404);
}
