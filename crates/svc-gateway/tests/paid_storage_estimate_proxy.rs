//! paid_storage_estimate_proxy.rs — integration tests for `/paid/o/estimate` → omnigate.
//!
//! RO:WHAT — Spin up dummy omnigate and real svc-gateway; assert estimate proxy behavior.
//! RO:WHY — WEB3 product UX needs edge-facing preflight price estimates before wallet holds.
//! RO:INTERACTS — svc_gateway::routes, Config upstream omnigate base URL, reqwest client.
//! RO:INVARIANTS — estimate proxy is read-only; status/body pass through; transport failures map to 502.
//! RO:METRICS — exercises gateway route metrics/correlation layer.
//! RO:CONFIG — SVC_GATEWAY_OMNIGATE_BASE_URL, SVC_GATEWAY_BIND_ADDR.
//! RO:SECURITY — verifies auth/correlation headers can reach omnigate without forwarding hop-by-hop headers.
//! RO:TEST — cargo test -p svc-gateway --test paid_storage_estimate_proxy.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    http::{HeaderMap, StatusCode, Uri},
    routing::get,
    Json, Router,
};
use once_cell::sync::OnceCell;
use serde::Serialize;
use serde_json::Value;
use svc_gateway::{config::Config, observability::metrics, routes, state::AppState};
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

/// Test-only helper: register gateway metrics once per test binary and return
/// cloned handles for each caller.
fn test_metrics_handles() -> metrics::MetricsHandles {
    static CELL: OnceCell<metrics::MetricsHandles> = OnceCell::new();
    CELL.get_or_init(|| metrics::register().expect("register metrics once for test process"))
        .clone()
}

async fn start_dummy_omnigate() -> SocketAddr {
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
        .route("/v1/paid/o/estimate", get(estimate_handler));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy omnigate");
    let addr = listener.local_addr().expect("dummy omnigate local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("dummy omnigate serve");
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
async fn paid_storage_estimate_proxy_happy_path() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let url = format!("http://{gateway_addr}/paid/o/estimate?bytes=48");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("authorization", "Bearer dev")
        .header("x-ron-token", "ron-token-123")
        .header("x-ron-passport", "passport-abc")
        .header("x-correlation-id", "corr-paid-estimate")
        .send()
        .await
        .expect("gateway paid storage estimate response");

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

    let req_id = body["x_request_id"].as_str().unwrap_or_default();
    assert!(
        !req_id.is_empty(),
        "expected gateway correlation layer to generate x-request-id"
    );

    assert!(body["connection"].is_null());

    clear_gateway_env();
}

#[tokio::test]
async fn paid_storage_estimate_proxy_passes_omnigate_errors_through() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let url = format!("http://{gateway_addr}/paid/o/estimate?bytes=not-a-number");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .expect("gateway paid storage estimate bad request response");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: Value = resp.json().await.expect("parse estimate error JSON body");
    assert_eq!(body["error"], "bad_request");
    assert_eq!(body["reason"], "bytes must be an unsigned integer");

    clear_gateway_env();
}

#[tokio::test]
async fn paid_storage_estimate_upstream_connect_failure_yields_problem_502() {
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

    let url = format!("http://{gateway_addr}/paid/o/estimate?bytes=48");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .expect("gateway paid storage estimate response");

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

    let body: Value = resp.json().await.expect("parse gateway Problem body");
    assert_eq!(body["code"], "upstream_unavailable");
    assert_eq!(body["retryable"], true);

    clear_gateway_env();
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

fn clear_gateway_env() {
    std::env::remove_var("SVC_GATEWAY_OMNIGATE_BASE_URL");
    std::env::remove_var("SVC_GATEWAY_STORAGE_BASE_URL");
    std::env::remove_var("SVC_GATEWAY_BIND_ADDR");
}
