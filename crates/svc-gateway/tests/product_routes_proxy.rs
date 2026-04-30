//! Product route proxy tests for WEB3_2 gateway exposure.
//!
//! RO:WHAT — Spin up dummy `omnigate` and real `svc-gateway`; assert product route proxy behavior.
//! RO:WHY — Browser extension and clients need stable edge paths for `crab`, `b3`, assets, and sites.
//! RO:INTERACTS — `svc_gateway::routes`, `Config`, `AppState`, `SVC_GATEWAY_OMNIGATE_BASE_URL`.
//! RO:INVARIANTS — gateway is proxy-only; it preserves selected headers/body/query and filters hop-by-hop headers.
//! RO:METRICS — exercises gateway correlation/HTTP metric layers.
//! RO:CONFIG — `SVC_GATEWAY_OMNIGATE_BASE_URL`, `SVC_GATEWAY_BIND_ADDR`.
//! RO:SECURITY — no wallet/ledger/storage mutation; forwards only allowlisted headers.
//! RO:TEST — `cargo test -p svc-gateway --test product_routes_proxy`.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    body::Bytes,
    http::{HeaderMap, Method, StatusCode, Uri},
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::OnceCell;
use serde::Serialize;
use serde_json::Value;
use svc_gateway::{config::Config, observability::metrics, routes, state::AppState};
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[derive(Debug, Serialize)]
struct ProductEcho {
    method: String,
    path: String,
    query: HashMap<String, String>,
    body_len: usize,
    authorization: Option<String>,
    passport: Option<String>,
    wallet_account: Option<String>,
    permission: Option<String>,
    spend_limit: Option<String>,
    idempotency_key: Option<String>,
    x_request_id: Option<String>,
    x_correlation_id: Option<String>,
    host: Option<String>,
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

    async fn echo_handler(
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Bytes,
    ) -> (StatusCode, Json<Value>) {
        if uri.path().contains("problem400") {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!(ErrorBody {
                    error: "bad_request",
                    reason: "product route rejected by omnigate",
                })),
            );
        }

        (
            StatusCode::OK,
            Json(serde_json::json!(ProductEcho {
                method: method.as_str().to_owned(),
                path: uri.path().to_owned(),
                query: parse_query(&uri),
                body_len: body.len(),
                authorization: grab(&headers, "authorization"),
                passport: grab(&headers, "x-ron-passport"),
                wallet_account: grab(&headers, "x-ron-wallet-account"),
                permission: grab(&headers, "x-ron-permission"),
                spend_limit: grab(&headers, "x-ron-spend-limit"),
                idempotency_key: grab(&headers, "idempotency-key"),
                x_request_id: grab(&headers, "x-request-id"),
                x_correlation_id: grab(&headers, "x-correlation-id"),
                host: grab(&headers, "host"),
                connection: grab(&headers, "connection"),
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/crab/resolve", get(echo_handler))
        .route("/v1/b3/:asset", get(echo_handler))
        .route("/v1/paid/o/prepare", post(echo_handler))
        .route("/v1/assets/image/prepare", post(echo_handler))
        .route("/v1/assets/image", post(echo_handler))
        .route("/v1/sites/prepare", post(echo_handler))
        .route("/v1/sites", post(echo_handler))
        .route("/v1/sites/:name", get(echo_handler))
        .route("/v1/assets/problem400", post(echo_handler));

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

    tokio::time::sleep(Duration::from_millis(50)).await;
    gateway_addr
}

#[tokio::test]
async fn crab_resolve_proxy_preserves_query_and_headers() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let crab_url = format!("crab://{HASH}.image");
    let url = format!("http://{gateway_addr}/crab/resolve?url={crab_url}&view=asset");

    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:alice")
        .header("x-ron-wallet-account", "acct_creator_alice")
        .header("x-ron-permission", "asset:view")
        .header("x-ron-spend-limit", "100")
        .header("connection", "close")
        .send()
        .await
        .expect("gateway crab resolve response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse crab resolve JSON");
    assert_eq!(body["method"], "GET");
    assert_eq!(body["path"], "/v1/crab/resolve");
    assert_eq!(body["query"]["url"], crab_url);
    assert_eq!(body["query"]["view"], "asset");
    assert_eq!(body["authorization"], "Bearer dev");
    assert_eq!(body["passport"], "passport:main:alice");
    assert_eq!(body["wallet_account"], "acct_creator_alice");
    assert_eq!(body["permission"], "asset:view");
    assert_eq!(body["spend_limit"], "100");
    assert!(body["x_request_id"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(body["connection"].is_null());

    clear_gateway_env();
}

#[tokio::test]
async fn b3_asset_proxy_targets_omnigate_b3_route() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let url = format!("http://{gateway_addr}/b3/{HASH}.image");
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .expect("gateway b3 asset response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse b3 JSON");
    assert_eq!(body["method"], "GET");
    assert_eq!(body["path"], format!("/v1/b3/{HASH}.image"));
    assert_eq!(body["body_len"], 0);

    clear_gateway_env();
}

#[tokio::test]
async fn paid_prepare_proxy_preserves_body_and_idempotency() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let payload = serde_json::json!({
        "bytes": 12345,
        "content_type": "image/png",
        "action": "paid_storage_put"
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{gateway_addr}/paid/o/prepare"))
        .header("authorization", "Bearer dev")
        .header("content-type", "application/json")
        .header("idempotency-key", "idem-prepare-1")
        .json(&payload)
        .send()
        .await
        .expect("gateway paid prepare response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse paid prepare JSON");
    assert_eq!(body["method"], "POST");
    assert_eq!(body["path"], "/v1/paid/o/prepare");
    assert_eq!(body["idempotency_key"], "idem-prepare-1");
    assert!(body["body_len"].as_u64().unwrap_or_default() > 0);

    clear_gateway_env();
}

#[tokio::test]
async fn image_and_site_product_routes_target_omnigate() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let cases = [
        ("/assets/image/prepare", "/v1/assets/image/prepare"),
        ("/assets/image", "/v1/assets/image"),
        ("/sites/prepare", "/v1/sites/prepare"),
        ("/sites", "/v1/sites"),
    ];

    let client = reqwest::Client::new();

    for (gateway_path, upstream_path) in cases {
        let resp = client
            .post(format!("http://{gateway_addr}{gateway_path}"))
            .header("content-type", "application/json")
            .body(r#"{"demo":true}"#)
            .send()
            .await
            .expect("gateway product route response");

        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = resp.json().await.expect("parse product route JSON");
        assert_eq!(body["method"], "POST");
        assert_eq!(body["path"], upstream_path);
        assert!(body["body_len"].as_u64().unwrap_or_default() > 0);
    }

    clear_gateway_env();
}

#[tokio::test]
async fn site_resolve_proxy_targets_omnigate_site_hydrator() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{gateway_addr}/sites/SeaLobsta.COM"))
        .header("authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:alice")
        .header("x-ron-wallet-account", "acct_site_owner")
        .header("x-ron-permission", "site:view")
        .header("connection", "close")
        .send()
        .await
        .expect("gateway site resolve response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse site resolve JSON");
    assert_eq!(body["method"], "GET");
    assert_eq!(body["path"], "/v1/sites/SeaLobsta.COM");
    assert_eq!(body["body_len"], 0);
    assert_eq!(body["authorization"], "Bearer dev");
    assert_eq!(body["passport"], "passport:main:alice");
    assert_eq!(body["wallet_account"], "acct_site_owner");
    assert_eq!(body["permission"], "site:view");
    assert!(body["connection"].is_null());

    clear_gateway_env();
}

#[tokio::test]
async fn product_route_upstream_connect_failure_yields_problem_502() {
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

    let url = format!("http://{gateway_addr}/b3/{HASH}.image");
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .expect("gateway product failure response");

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
