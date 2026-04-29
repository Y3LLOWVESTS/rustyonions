//! paid_storage_write_proxy.rs — integration tests for `/paid/o` → omnigate.
//!
//! RO:WHAT — Spin up dummy omnigate and real `svc-gateway`; assert paid write proxy behavior.
//! RO:WHY — WEB3 product UX needs edge-facing paid write submission after wallet holds.
//! RO:INTERACTS — `svc_gateway::routes`, Config upstream omnigate base URL, reqwest client.
//! RO:INVARIANTS — gateway is proxy-only; status/body pass through; transport failures map to 502.
//! RO:METRICS — exercises gateway route metrics/correlation layer.
//! RO:CONFIG — `SVC_GATEWAY_OMNIGATE_BASE_URL`, `SVC_GATEWAY_BIND_ADDR`.
//! RO:SECURITY — verifies auth/payment/accounting headers can reach omnigate without hop-by-hop headers.
//! RO:TEST — `cargo test -p svc-gateway --test paid_storage_write_proxy`.

use std::{net::SocketAddr, time::Duration};

use axum::{
    body::Bytes,
    http::{HeaderMap, Method, StatusCode},
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::OnceCell;
use serde::Serialize;
use serde_json::Value;
use svc_gateway::{config::Config, observability::metrics, routes, state::AppState};
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Serialize)]
struct PaidWriteEcho {
    cid: &'static str,
    paid: bool,
    method: String,
    body_len: usize,
    authorization: Option<String>,
    wallet_txid: Option<String>,
    wallet_receipt_hash: Option<String>,
    wallet_from: Option<String>,
    wallet_to: Option<String>,
    wallet_idem: Option<String>,
    paid_asset: Option<String>,
    paid_estimate_minor: Option<String>,
    tenant: Option<String>,
    accounting_subject: Option<String>,
    region: Option<String>,
    pin_seconds: Option<String>,
    idempotency_key: Option<String>,
    x_request_id: Option<String>,
    connection: Option<String>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    reason: &'static str,
}

fn test_metrics_handles() -> metrics::MetricsHandles {
    static CELL: OnceCell<metrics::MetricsHandles> = OnceCell::new();
    CELL.get_or_init(|| metrics::register().expect("register metrics once for test process"))
        .clone()
}

async fn start_dummy_omnigate() -> SocketAddr {
    async fn healthz() -> &'static str {
        "ok"
    }

    async fn paid_write_handler(
        method: Method,
        headers: HeaderMap,
        body: Bytes,
    ) -> (StatusCode, Json<Value>) {
        if body.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!(ErrorBody {
                    error: "bad_request",
                    reason: "empty body",
                })),
            );
        }

        (
            StatusCode::OK,
            Json(serde_json::json!(PaidWriteEcho {
                cid: "b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57",
                paid: true,
                method: method.as_str().to_owned(),
                body_len: body.len(),
                authorization: grab(&headers, "authorization"),
                wallet_txid: grab(&headers, "x-ron-wallet-txid"),
                wallet_receipt_hash: grab(&headers, "x-ron-wallet-receipt-hash"),
                wallet_from: grab(&headers, "x-ron-wallet-from"),
                wallet_to: grab(&headers, "x-ron-wallet-to"),
                wallet_idem: grab(&headers, "x-ron-wallet-idem"),
                paid_asset: grab(&headers, "x-ron-paid-asset"),
                paid_estimate_minor: grab(&headers, "x-ron-paid-estimate-minor"),
                tenant: grab(&headers, "x-ron-tenant"),
                accounting_subject: grab(&headers, "x-ron-accounting-subject"),
                region: grab(&headers, "x-ron-region"),
                pin_seconds: grab(&headers, "x-ron-pin-seconds"),
                idempotency_key: grab(&headers, "idempotency-key"),
                x_request_id: grab(&headers, "x-request-id"),
                connection: grab(&headers, "connection"),
            })),
        )
    }

    let router = Router::new().route("/healthz", get(healthz)).route(
        "/v1/paid/o",
        post(paid_write_handler).put(paid_write_handler),
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy omnigate");
    let addr = listener.local_addr().expect("dummy omnigate local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("dummy omnigate serve");
    });

    wait_for_health(format!("http://{addr}/healthz")).await;
    addr
}

async fn start_gateway(omnigate_addr: SocketAddr) -> SocketAddr {
    let omnigate_base = format!("http://{omnigate_addr}");

    std::env::set_var("SVC_GATEWAY_OMNIGATE_BASE_URL", omnigate_base);
    std::env::set_var("SVC_GATEWAY_BIND_ADDR", "127.0.0.1:0");

    let cfg = Config::load().expect("load config with omnigate env override");
    let metrics_handles = test_metrics_handles();
    let state = AppState::new(cfg.clone(), metrics_handles);
    let router = routes::build_router(&state);

    let listener = TcpListener::bind(&cfg.server.bind_addr)
        .await
        .expect("bind gateway");
    let gateway_addr = listener.local_addr().expect("gateway local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("gateway serve");
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    gateway_addr
}

#[tokio::test]
async fn paid_storage_write_proxy_post_happy_path() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{gateway_addr}/paid/o"))
        .header("authorization", "Bearer dev")
        .header("content-type", "application/octet-stream")
        .header("idempotency-key", "idem-paid-write")
        .header("x-ron-paid-op", "hold")
        .header("x-ron-paid-asset", "roc")
        .header("x-ron-paid-estimate-minor", "84")
        .header("x-ron-wallet-txid", "tx_123")
        .header(
            "x-ron-wallet-receipt-hash",
            "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .header("x-ron-wallet-from", "acct_user")
        .header("x-ron-wallet-to", "escrow_paid_write")
        .header("x-ron-wallet-idem", "storage_put:abc123")
        .header("x-ron-tenant", "7")
        .header("x-ron-accounting-subject", "svc_storage_provider")
        .header("x-ron-region", "local")
        .header("x-ron-pin-seconds", "60")
        .body("RustyOnions paid write through gateway")
        .send()
        .await
        .expect("gateway paid write response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse paid write JSON body");
    assert_eq!(body["paid"], true);
    assert_eq!(body["method"], "POST");
    assert_eq!(body["body_len"], 38);
    assert_eq!(body["authorization"], "Bearer dev");
    assert_eq!(body["paid_asset"], "roc");
    assert_eq!(body["paid_estimate_minor"], "84");
    assert_eq!(body["wallet_txid"], "tx_123");
    assert_eq!(
        body["wallet_receipt_hash"],
        "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert_eq!(body["wallet_from"], "acct_user");
    assert_eq!(body["wallet_to"], "escrow_paid_write");
    assert_eq!(body["wallet_idem"], "storage_put:abc123");
    assert_eq!(body["tenant"], "7");
    assert_eq!(body["accounting_subject"], "svc_storage_provider");
    assert_eq!(body["region"], "local");
    assert_eq!(body["pin_seconds"], "60");
    assert_eq!(body["idempotency_key"], "idem-paid-write");

    let req_id = body["x_request_id"].as_str().unwrap_or_default();
    assert!(
        !req_id.is_empty(),
        "expected gateway correlation layer to generate x-request-id"
    );

    assert!(body["connection"].is_null());

    clear_gateway_env();
}

#[tokio::test]
async fn paid_storage_write_proxy_put_happy_path() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .put(format!("http://{gateway_addr}/paid/o"))
        .header("content-type", "application/octet-stream")
        .header("x-ron-paid-op", "hold")
        .header("x-ron-paid-asset", "roc")
        .header("x-ron-paid-estimate-minor", "84")
        .body("put paid body")
        .send()
        .await
        .expect("gateway paid write response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse paid write JSON body");
    assert_eq!(body["method"], "PUT");
    assert_eq!(body["body_len"], 13);
    assert_eq!(body["paid_asset"], "roc");

    clear_gateway_env();
}

#[tokio::test]
async fn paid_storage_write_proxy_passes_omnigate_errors_through() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{gateway_addr}/paid/o"))
        .body(Bytes::new())
        .send()
        .await
        .expect("gateway paid write bad request response");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: Value = resp.json().await.expect("parse paid write error JSON body");
    assert_eq!(body["error"], "bad_request");
    assert_eq!(body["reason"], "empty body");

    clear_gateway_env();
}

#[tokio::test]
async fn paid_storage_write_upstream_connect_failure_yields_problem_502() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    std::env::set_var("SVC_GATEWAY_OMNIGATE_BASE_URL", "http://127.0.0.1:1");
    std::env::set_var("SVC_GATEWAY_BIND_ADDR", "127.0.0.1:0");

    let cfg = Config::load().expect("load config with omnigate env override");
    let metrics_handles = test_metrics_handles();
    let state = AppState::new(cfg.clone(), metrics_handles);
    let router = routes::build_router(&state);

    let listener = TcpListener::bind(&cfg.server.bind_addr)
        .await
        .expect("bind gateway");
    let gateway_addr = listener.local_addr().expect("gateway local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("gateway serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{gateway_addr}/paid/o"))
        .body("hello")
        .send()
        .await
        .expect("gateway paid write response");

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

    let body: Value = resp.json().await.expect("parse gateway Problem body");
    assert_eq!(body["code"], "upstream_unavailable");
    assert_eq!(body["retryable"], true);

    clear_gateway_env();
}

async fn wait_for_health(url: String) {
    let client = reqwest::Client::new();

    for _ in 0..50 {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    panic!("service did not become healthy at {url}");
}

fn grab(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn clear_gateway_env() {
    std::env::remove_var("SVC_GATEWAY_OMNIGATE_BASE_URL");
    std::env::remove_var("SVC_GATEWAY_STORAGE_BASE_URL");
    std::env::remove_var("SVC_GATEWAY_BIND_ADDR");
}
