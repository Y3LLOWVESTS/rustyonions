//! Product proxy tests for paid content_view browser routes.
//!
//! RO:WHAT — Verifies svc-gateway exposes /content/view quote/pay as proxy-only routes.
//! RO:WHY — NEXT_LEVEL paid asset views need stable browser paths while wallet mutation stays downstream.
//! RO:INTERACTS — svc_gateway::routes::product, AppState, dummy omnigate /v1/content/view/*.
//! RO:INVARIANTS — gateway does not price, inspect manifests, or mutate wallet/ledger; it preserves selected headers/body.
//! RO:METRICS — exercises gateway correlation/HTTP metric layers when router is built.
//! RO:CONFIG — SVC_GATEWAY_OMNIGATE_BASE_URL, SVC_GATEWAY_BIND_ADDR.
//! RO:SECURITY — filters hop-by-hop inbound headers; outbound client may synthesize upstream Host as required by HTTP.
//! RO:TEST — cargo test -p svc-gateway --test content_view_routes_proxy.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    body::Bytes,
    http::{HeaderMap, Method, StatusCode, Uri},
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
struct ContentViewEcho {
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
    host: Option<String>,
    connection: Option<String>,
}

fn test_metrics_handles() -> metrics::MetricsHandles {
    static CELL: OnceCell<metrics::MetricsHandles> = OnceCell::new();
    CELL.get_or_init(|| metrics::register().expect("register metrics once for test process"))
        .clone()
}

#[tokio::test]
async fn content_view_quote_and_pay_proxy_to_omnigate_and_preserve_context() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();

    let cases = [
        (
            "/content/view/quote",
            "/v1/content/view/quote",
            r#"{
                "asset_crab_url":"crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.article",
                "payer_account":"acct_visitor_b",
                "viewer_wallet_account":"acct_visitor_b",
                "viewer_passport_subject":"passport:main:visitor-b",
                "recipient_account":"acct_creator",
                "max_amount_minor":"5",
                "client_idempotency_key":"gateway-content-view-quote"
            }"#,
            "gateway-content-view-quote",
        ),
        (
            "/content/view/pay",
            "/v1/content/view/pay",
            r#"{
                "asset_crab_url":"crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.article",
                "payer_account":"acct_visitor_b",
                "viewer_wallet_account":"acct_visitor_b",
                "viewer_passport_subject":"passport:main:visitor-b",
                "recipient_account":"acct_creator",
                "amount_minor":"5",
                "asset":"roc",
                "quote_id":"content-view-test",
                "quote_hash":"quotehash",
                "client_idempotency_key":"gateway-content-view-pay"
            }"#,
            "gateway-content-view-pay",
        ),
    ];

    for (gateway_path, expected_upstream_path, payload, idempotency_key) in cases {
        let response = client
            .post(format!("http://{gateway_addr}{gateway_path}"))
            .header("authorization", "Bearer dev")
            .header("content-type", "application/json")
            .header("x-ron-passport", "passport:main:visitor-b")
            .header("x-ron-wallet-account", "acct_visitor_b")
            .header("idempotency-key", idempotency_key)
            .header("x-request-id", format!("req-{idempotency_key}"))
            .header("x-correlation-id", format!("corr-{idempotency_key}"))
            .header("connection", "close")
            .body(payload)
            .send()
            .await
            .expect("gateway content_view route response");

        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = response
            .json()
            .await
            .expect("parse content_view proxy JSON");
        assert_eq!(body["method"], "POST");
        assert_eq!(body["path"], expected_upstream_path);
        assert_eq!(body["authorization"], "Bearer dev");
        assert_eq!(body["passport"], "passport:main:visitor-b");
        assert_eq!(body["wallet_account"], "acct_visitor_b");
        assert_eq!(body["idempotency_key"], idempotency_key);
        assert_eq!(body["x_request_id"], format!("req-{idempotency_key}"));
        assert_eq!(body["x_correlation_id"], format!("corr-{idempotency_key}"));
        assert!(body["body_len"].as_u64().unwrap_or_default() > 0);

        // The inbound Host header is not forwarded, but reqwest/hyper must send
        // a new valid upstream Host header.
        assert_eq!(body["host"], omnigate_addr.to_string());

        // Connection is hop-by-hop and should not be forwarded by the gateway.
        assert!(body["connection"].is_null());
    }

    clear_gateway_env();
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
        (
            StatusCode::OK,
            Json(serde_json::json!(ContentViewEcho {
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
                host: grab(&headers, "host"),
                connection: grab(&headers, "connection"),
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .fallback(echo_handler);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy omnigate");

    let addr = listener.local_addr().expect("dummy omnigate local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("dummy omnigate serve");
    });

    addr
}

async fn start_gateway(omnigate_addr: SocketAddr) -> SocketAddr {
    std::env::set_var(
        "SVC_GATEWAY_OMNIGATE_BASE_URL",
        format!("http://{omnigate_addr}"),
    );
    std::env::set_var("SVC_GATEWAY_BIND_ADDR", "127.0.0.1:0");

    let cfg = Config::load().expect("load gateway config");
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

    wait_for_health(format!("http://{gateway_addr}/healthz")).await;

    gateway_addr
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
    uri.query()
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
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
