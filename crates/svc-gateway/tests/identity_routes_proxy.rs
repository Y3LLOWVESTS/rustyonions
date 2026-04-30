//! Identity route proxy tests for CrabLink passport bootstrap.
//!
//! RO:WHAT — Spin up dummy `omnigate` and real `svc-gateway`; assert identity/wallet edge proxy behavior.
//! RO:WHY — CrabLink must call gateway-only routes for passport and wallet display flows.
//! RO:INTERACTS — `svc_gateway::routes`, `Config`, `AppState`, `SVC_GATEWAY_OMNIGATE_BASE_URL`.
//! RO:INVARIANTS — gateway remains proxy-only; it forwards identity headers and idempotency keys.
//! RO:METRICS — exercises gateway correlation/HTTP metric layers.
//! RO:CONFIG — `SVC_GATEWAY_OMNIGATE_BASE_URL`, `SVC_GATEWAY_BIND_ADDR`.
//! RO:SECURITY — no direct `svc-passport`, `svc-wallet`, or ledger calls.
//! RO:TEST — `cargo test -p svc-gateway --test identity_routes_proxy`.

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

#[derive(Debug, Serialize)]
struct IdentityEcho {
    method: String,
    path: String,
    query: HashMap<String, String>,
    body_len: usize,
    authorization: Option<String>,
    passport: Option<String>,
    wallet_account: Option<String>,
    idempotency_key: Option<String>,
    x_request_id: Option<String>,
    x_correlation_id: Option<String>,
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
                    reason: "identity route rejected by omnigate",
                })),
            );
        }

        (
            StatusCode::OK,
            Json(serde_json::json!(IdentityEcho {
                method: method.as_str().to_owned(),
                path: uri.path().to_owned(),
                query: parse_query(&uri),
                body_len: body.len(),
                authorization: grab(&headers, "authorization"),
                passport: grab(&headers, "x-ron-passport"),
                wallet_account: grab(&headers, "x-ron-wallet-account"),
                idempotency_key: grab(&headers, "idempotency-key"),
                x_request_id: grab(&headers, "x-request-id"),
                x_correlation_id: grab(&headers, "x-correlation-id"),
                connection: grab(&headers, "connection"),
            })),
        )
    }

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/identity/me", get(echo_handler))
        .route("/v1/identity/passport/bootstrap", post(echo_handler))
        .route("/v1/wallet/:account/balance", get(echo_handler));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy omnigate");
    let addr = listener.local_addr().expect("dummy local addr");

    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("dummy omnigate serve");
    });

    wait_for_health(format!("http://{addr}/healthz")).await;
    addr
}

async fn start_gateway(omnigate_addr: SocketAddr) -> SocketAddr {
    clear_gateway_env();

    std::env::set_var(
        "SVC_GATEWAY_OMNIGATE_BASE_URL",
        format!("http://{omnigate_addr}"),
    );
    std::env::set_var("SVC_GATEWAY_BIND_ADDR", "127.0.0.1:0");

    let cfg = Config::load().expect("load config with omnigate env override");
    let metrics_handles = test_metrics_handles();
    let state = AppState::new(cfg.clone(), metrics_handles);
    let router = routes::build_router(&state);

    let listener = TcpListener::bind(&cfg.server.bind_addr)
        .await
        .expect("bind gateway");
    let gateway_addr = listener.local_addr().expect("gateway local addr");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("gateway serve");
    });

    wait_for_health(format!("http://{gateway_addr}/healthz")).await;
    gateway_addr
}

#[tokio::test]
async fn identity_me_proxy_preserves_passport_headers() {
    let _guard = ENV_LOCK.lock().await;

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{gateway_addr}/identity/me"))
        .header("Authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:dev")
        .header("x-ron-wallet-account", "acct_dev")
        .header("x-correlation-id", "corr-identity")
        .header("Connection", "close")
        .send()
        .await
        .expect("identity/me response");

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("identity echo body");
    assert_eq!(body["method"], "GET");
    assert_eq!(body["path"], "/v1/identity/me");
    assert_eq!(body["authorization"], "Bearer dev");
    assert_eq!(body["passport"], "passport:main:dev");
    assert_eq!(body["wallet_account"], "acct_dev");
    assert_eq!(body["x_correlation_id"], "corr-identity");
    assert!(body["x_request_id"].is_string());
    assert!(body["connection"].is_null());

    clear_gateway_env();
}

#[tokio::test]
async fn passport_bootstrap_proxy_preserves_body_and_idempotency() {
    let _guard = ENV_LOCK.lock().await;

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;
    let request_body = r#"{"kind":"main","label":"Local Dev Passport"}"#;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{gateway_addr}/identity/passport/bootstrap"))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", "idem-passport")
        .header("x-ron-passport", "passport:main:dev")
        .body(request_body)
        .send()
        .await
        .expect("passport bootstrap response");

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("bootstrap echo body");
    assert_eq!(body["method"], "POST");
    assert_eq!(body["path"], "/v1/identity/passport/bootstrap");
    assert_eq!(body["body_len"].as_u64(), Some(request_body.len() as u64));
    assert_eq!(body["idempotency_key"], "idem-passport");
    assert_eq!(body["passport"], "passport:main:dev");

    clear_gateway_env();
}

#[tokio::test]
async fn wallet_balance_proxy_targets_omnigate_wallet_balance() {
    let _guard = ENV_LOCK.lock().await;

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{gateway_addr}/wallet/acct_dev/balance"))
        .header("x-ron-passport", "passport:main:dev")
        .header("x-ron-wallet-account", "acct_dev")
        .send()
        .await
        .expect("wallet balance response");

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("wallet echo body");
    assert_eq!(body["method"], "GET");
    assert_eq!(body["path"], "/v1/wallet/acct_dev/balance");
    assert_eq!(body["passport"], "passport:main:dev");
    assert_eq!(body["wallet_account"], "acct_dev");

    clear_gateway_env();
}

#[tokio::test]
async fn identity_route_upstream_connect_failure_yields_problem_502() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    std::env::set_var("SVC_GATEWAY_OMNIGATE_BASE_URL", "http://127.0.0.1:1");
    std::env::set_var("SVC_GATEWAY_BIND_ADDR", "127.0.0.1:0");

    let cfg = Config::load().expect("load config with bad omnigate override");
    let metrics_handles = test_metrics_handles();
    let state = AppState::new(cfg.clone(), metrics_handles);
    let router = routes::build_router(&state);

    let listener = TcpListener::bind(&cfg.server.bind_addr)
        .await
        .expect("bind gateway");
    let gateway_addr = listener.local_addr().expect("gateway local addr");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("gateway serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{gateway_addr}/identity/me"))
        .send()
        .await
        .expect("identity/me failure response");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

    let body: Value = response.json().await.expect("problem body");
    assert_eq!(body["code"], "upstream_unavailable");
    assert_eq!(body["retryable"], true);

    clear_gateway_env();
}

async fn wait_for_health(url: String) {
    let client = reqwest::Client::new();

    for _ in 0..50 {
        if let Ok(response) = client.get(&url).send().await {
            if response.status().is_success() {
                return;
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    panic!("service did not become healthy at {url}");
}

fn parse_query(uri: &Uri) -> HashMap<String, String> {
    let mut map = HashMap::new();

    if let Some(qs) = uri.query() {
        for pair in qs.split('&') {
            if pair.is_empty() {
                continue;
            }

            let mut parts = pair.splitn(2, '=');
            let key = parts.next().unwrap_or("").trim();
            if key.is_empty() {
                continue;
            }

            let value = parts.next().unwrap_or("").trim();
            map.insert(key.to_owned(), value.to_owned());
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
