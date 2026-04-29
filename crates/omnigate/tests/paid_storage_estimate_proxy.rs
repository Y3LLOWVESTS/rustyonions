//! paid_storage_estimate_proxy.rs — integration tests for `/v1/paid/o/estimate` → svc-storage.
//!
//! RO:WHAT — Spin up dummy svc-storage and real omnigate route; assert paid estimate proxy behavior.
//! RO:WHY — WEB3 product UX needs BFF-facing preflight price estimates before wallet holds.
//! RO:INTERACTS — omnigate::routes::v1::paid, svc-storage /paid/o/estimate, reqwest client.
//! RO:INVARIANTS — estimate proxy is read-only; status/body pass through; transport failures map to 502.
//! RO:METRICS — route metrics are covered when mounted through full App::build.
//! RO:CONFIG — OMNIGATE_STORAGE_BASE_URL, OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL.
//! RO:SECURITY — verifies auth/correlation/x-ron headers reach storage without hop-by-hop headers.
//! RO:TEST — cargo test -p omnigate --test paid_storage_estimate_proxy.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    http::{HeaderMap, StatusCode, Uri},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use serde_json::Value;
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Serialize)]
struct EstimateEcho {
    schema: &'static str,
    route: &'static str,
    action: &'static str,
    asset: &'static str,
    bytes: u64,
    amount_minor: &'static str,
    minimum_hold_minor: &'static str,
    pricing_mode: &'static str,
    authorization: Option<String>,
    x_ron_token: Option<String>,
    x_ron_passport: Option<String>,
    x_correlation_id: Option<String>,
    x_request_id: Option<String>,
    host: Option<String>,
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

    async fn estimate_handler(headers: HeaderMap, uri: Uri) -> (StatusCode, Json<Value>) {
        let query = parse_query(&uri);
        let Some(raw_bytes) = query.get("bytes") else {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!(ErrorBody {
                    error: "bad_request",
                    reason: "missing required query parameter: bytes",
                })),
            );
        };

        let Ok(bytes) = raw_bytes.parse::<u64>() else {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!(ErrorBody {
                    error: "bad_request",
                    reason: "bytes must be an unsigned integer",
                })),
            );
        };

        (
            StatusCode::OK,
            Json(serde_json::json!(EstimateEcho {
                schema: "svc-storage.paid-storage-estimate.v1",
                route: "/paid/o",
                action: "paid_storage_put",
                asset: "roc",
                bytes,
                amount_minor: "84",
                minimum_hold_minor: "84",
                pricing_mode: "roc-economics",
                authorization: grab(&headers, "authorization"),
                x_ron_token: grab(&headers, "x-ron-token"),
                x_ron_passport: grab(&headers, "x-ron-passport"),
                x_correlation_id: grab(&headers, "x-correlation-id"),
                x_request_id: grab(&headers, "x-request-id"),
                host: grab(&headers, "host"),
                connection: grab(&headers, "connection"),
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/paid/o/estimate", get(estimate_handler));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy storage");
    let addr = listener.local_addr().expect("dummy storage local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("dummy storage serve");
    });

    let base = format!("http://{addr}");
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if let Ok(resp) = client.get(format!("{base}/healthz")).send().await {
            if resp.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

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
async fn paid_storage_estimate_proxy_happy_path() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let omnigate_addr = start_omnigate_paid_route(storage_addr).await;

    let url = format!("http://{omnigate_addr}/v1/paid/o/estimate?bytes=48");
    let client = reqwest::Client::new();

    let resp = client
        .get(&url)
        .header("authorization", "Bearer dev")
        .header("x-ron-token", "ron-token-123")
        .header("x-ron-passport", "passport-abc")
        .header("x-correlation-id", "corr-paid-estimate")
        .header("x-request-id", "req-paid-estimate")
        .send()
        .await
        .expect("omnigate paid storage estimate response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse estimate JSON body");

    assert_eq!(body["schema"], "svc-storage.paid-storage-estimate.v1");
    assert_eq!(body["route"], "/paid/o");
    assert_eq!(body["action"], "paid_storage_put");
    assert_eq!(body["asset"], "roc");
    assert_eq!(body["bytes"], 48);
    assert_eq!(body["amount_minor"], "84");
    assert_eq!(body["minimum_hold_minor"], "84");
    assert_eq!(body["pricing_mode"], "roc-economics");

    assert_eq!(body["authorization"], "Bearer dev");
    assert_eq!(body["x_ron_token"], "ron-token-123");
    assert_eq!(body["x_ron_passport"], "passport-abc");
    assert_eq!(body["x_correlation_id"], "corr-paid-estimate");
    assert_eq!(body["x_request_id"], "req-paid-estimate");

    assert!(body["connection"].is_null());

    clear_env();
}

#[tokio::test]
async fn paid_storage_estimate_proxy_passes_storage_errors_through() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let omnigate_addr = start_omnigate_paid_route(storage_addr).await;

    let url = format!("http://{omnigate_addr}/v1/paid/o/estimate?bytes=not-a-number");
    let client = reqwest::Client::new();

    let resp = client
        .get(&url)
        .send()
        .await
        .expect("omnigate paid storage estimate bad request response");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: Value = resp.json().await.expect("parse estimate error JSON body");
    assert_eq!(body["error"], "bad_request");
    assert_eq!(body["reason"], "bytes must be an unsigned integer");

    clear_env();
}

#[tokio::test]
async fn paid_storage_estimate_upstream_connect_failure_yields_problem_502() {
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

    let url = format!("http://{addr}/v1/paid/o/estimate?bytes=48");
    let client = reqwest::Client::new();

    let resp = client
        .get(&url)
        .send()
        .await
        .expect("omnigate paid storage estimate response");

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

    let body: Value = resp.json().await.expect("parse omnigate Problem body");
    assert_eq!(body["code"], "upstream_unavailable");
    assert_eq!(body["message"], "storage estimate upstream unavailable");
    assert_eq!(body["retryable"], true);
    assert_eq!(body["reason"], "storage_connect");

    clear_env();
}

fn parse_query(uri: &Uri) -> HashMap<String, String> {
    let mut map = HashMap::new();

    if let Some(query) = uri.query() {
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }

            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            let key = key.trim();

            if key.is_empty() {
                continue;
            }

            map.insert(key.to_owned(), value.trim().to_owned());
        }
    }

    map
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
