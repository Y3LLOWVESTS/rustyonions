//! paid_storage_write_proxy.rs — integration tests for `/v1/paid/o` → `svc-storage`.
//!
//! RO:WHAT — Spin up dummy `svc-storage` and real omnigate route; assert paid write proxy behavior.
//! RO:WHY — WEB3 product UX needs BFF-facing paid write submission after wallet holds.
//! RO:INTERACTS — `omnigate::routes::v1::paid`, `svc-storage /paid/o`, reqwest client.
//! RO:INVARIANTS — omnigate is proxy-only; status/body pass through; transport failures map to 502.
//! RO:METRICS — route metrics are covered when mounted through full `App::build`.
//! RO:CONFIG — `OMNIGATE_STORAGE_BASE_URL`, `OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL`.
//! RO:SECURITY — verifies auth/payment/accounting headers reach storage without hop-by-hop headers.
//! RO:TEST — `cargo test -p omnigate --test paid_storage_write_proxy`.

use std::{net::SocketAddr, time::Duration};

use axum::{
    body::Bytes,
    http::{HeaderMap, Method, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::Value;
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
    connection: Option<String>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    reason: &'static str,
}

async fn start_dummy_storage() -> SocketAddr {
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
                connection: grab(&headers, "connection"),
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/paid/o", post(paid_write_handler).put(paid_write_handler));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy storage");
    let addr = listener.local_addr().expect("dummy storage local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("dummy storage serve");
    });

    wait_for_health(format!("http://{addr}/healthz")).await;
    addr
}

async fn start_omnigate_paid_route(storage_addr: SocketAddr) -> SocketAddr {
    let storage_base = format!("http://{storage_addr}");
    std::env::set_var("OMNIGATE_STORAGE_BASE_URL", storage_base);

    let router = Router::new().nest("/v1", omnigate::routes::v1::router());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind omnigate paid route");
    let addr = listener
        .local_addr()
        .expect("omnigate paid route local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("omnigate paid route serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    addr
}

#[tokio::test]
async fn paid_storage_write_proxy_post_happy_path() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let omnigate_addr = start_omnigate_paid_route(storage_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/paid/o"))
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
        .body("RustyOnions paid write through omnigate")
        .send()
        .await
        .expect("omnigate paid write response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse paid write JSON body");
    assert_eq!(body["paid"], true);
    assert_eq!(body["method"], "POST");
    assert_eq!(body["body_len"], 39);
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
    assert!(body["connection"].is_null());

    clear_env();
}

#[tokio::test]
async fn paid_storage_write_proxy_put_happy_path() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let omnigate_addr = start_omnigate_paid_route(storage_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .put(format!("http://{omnigate_addr}/v1/paid/o"))
        .header("content-type", "application/octet-stream")
        .header("x-ron-paid-op", "hold")
        .header("x-ron-paid-asset", "roc")
        .header("x-ron-paid-estimate-minor", "84")
        .body("put paid body")
        .send()
        .await
        .expect("omnigate paid write response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse paid write JSON body");
    assert_eq!(body["method"], "PUT");
    assert_eq!(body["body_len"], 13);
    assert_eq!(body["paid_asset"], "roc");

    clear_env();
}

#[tokio::test]
async fn paid_storage_write_proxy_passes_storage_errors_through() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let omnigate_addr = start_omnigate_paid_route(storage_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/paid/o"))
        .body(Bytes::new())
        .send()
        .await
        .expect("omnigate paid write bad request response");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: Value = resp.json().await.expect("parse paid write error JSON body");
    assert_eq!(body["error"], "bad_request");
    assert_eq!(body["reason"], "empty body");

    clear_env();
}

#[tokio::test]
async fn paid_storage_write_upstream_connect_failure_yields_problem_502() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    std::env::set_var("OMNIGATE_STORAGE_BASE_URL", "http://127.0.0.1:1");

    let router = Router::new().nest("/v1", omnigate::routes::v1::router());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind omnigate paid route");
    let addr = listener
        .local_addr()
        .expect("omnigate paid route local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("omnigate paid route serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/v1/paid/o"))
        .body("hello")
        .send()
        .await
        .expect("omnigate paid write response");

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

    let body: Value = resp.json().await.expect("parse omnigate Problem body");
    assert_eq!(body["code"], "upstream_unavailable");
    assert_eq!(body["message"], "storage paid route upstream unavailable");
    assert_eq!(body["retryable"], true);
    assert_eq!(body["reason"], "storage_connect");

    clear_env();
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

fn clear_env() {
    std::env::remove_var("OMNIGATE_STORAGE_BASE_URL");
    std::env::remove_var("OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL");
}
