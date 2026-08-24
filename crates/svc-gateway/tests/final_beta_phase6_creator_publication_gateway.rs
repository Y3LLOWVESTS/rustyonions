//! RO:WHAT — Focused `svc-gateway` proxy tests for creator publication reads.
//! RO:WHY — Proves the public edge preserves Omnigate projection responses without adding authority.
//! RO:INTERACTS — `routes::product`, `Config`, `AppState`, and dummy Omnigate publication routes.
//! RO:INVARIANTS — gateway is proxy-only; query, allowed headers, status, and body pass through.
//! RO:SECURITY — no direct `svc-index`, wallet, ledger, receipt, entitlement, follow, or settlement authority.
//! RO:TEST — cargo test -p svc-gateway --test final_beta_phase6_creator_publication_gateway.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    body::Bytes,
    http::{HeaderMap, Method, StatusCode, Uri},
    routing::get,
    Json, Router,
};
use once_cell::sync::OnceCell;
use serde_json::Value;
use svc_gateway::{config::Config, observability::metrics, routes, state::AppState};
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

fn test_metrics_handles() -> metrics::MetricsHandles {
    static CELL: OnceCell<metrics::MetricsHandles> = OnceCell::new();

    CELL.get_or_init(|| metrics::register().expect("register gateway metrics once"))
        .clone()
}

async fn start_dummy_omnigate() -> SocketAddr {
    async fn healthz() -> &'static str {
        "ok"
    }

    async fn publication_handler(
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Bytes,
    ) -> (StatusCode, Json<Value>) {
        if uri
            .query()
            .is_some_and(|query| query.contains("walletBalance"))
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "code": "invalid_query",
                    "message": "unknown query field",
                    "retryable": false,
                    "reason": "unknown_query_field"
                })),
            );
        }

        if uri.path().ends_with("/missing") {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "code": "not_found",
                    "message": "publication was not found",
                    "retryable": false,
                    "reason": "publication_not_found"
                })),
            );
        }

        let schema = if uri.path().ends_with("/publications") {
            "crablink.publication-page.v1"
        } else {
            "crablink.publication-summary.v1"
        };

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "schema": schema,
                "method": method.to_string(),
                "path": uri.path(),
                "query": parse_query(&uri),
                "bodyLength": body.len(),
                "authorization": grab(
                    &headers,
                    "authorization",
                ),
                "xCorrelationId": grab(
                    &headers,
                    "x-correlation-id",
                ),
                "xRequestId": grab(
                    &headers,
                    "x-request-id",
                ),
                "connection": grab(
                    &headers,
                    "connection",
                )
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route(
            "/v1/creators/:username/publications",
            get(publication_handler),
        )
        .route(
            "/v1/creators/:username/publications/:publication_id",
            get(publication_handler),
        );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy omnigate");

    let address = listener.local_addr().expect("dummy omnigate address");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve dummy omnigate");
    });

    wait_for_health(format!("http://{address}/healthz",)).await;

    address
}

async fn start_gateway(omnigate_address: SocketAddr) -> SocketAddr {
    std::env::set_var(
        "SVC_GATEWAY_OMNIGATE_BASE_URL",
        format!("http://{omnigate_address}",),
    );

    std::env::set_var("SVC_GATEWAY_BIND_ADDR", "127.0.0.1:0");

    let config = Config::load().expect("load gateway configuration");

    let state = AppState::new(config.clone(), test_metrics_handles());

    let router = routes::build_router(&state);

    let listener = TcpListener::bind(&config.server.bind_addr)
        .await
        .expect("bind gateway");

    let address = listener.local_addr().expect("gateway address");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve gateway");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    address
}

#[tokio::test]
async fn phase6b4_list_route_preserves_query_and_headers() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let omnigate_address = start_dummy_omnigate().await;

    let gateway_address = start_gateway(omnigate_address).await;

    let response = reqwest::Client::new()
        .get(format!(
            "http://{gateway_address}/creators/rusty_crab/publications",
        ))
        .query(&[("cursor", "p_00000002"), ("limit", "20")])
        .header("authorization", "Bearer publication-read")
        .header("x-correlation-id", "corr-phase6b4")
        .header("x-request-id", "req-phase6b4")
        .header("connection", "close")
        .send()
        .await
        .expect("gateway publication list");

    assert_eq!(response.status(), StatusCode::OK,);

    let body: Value = response.json().await.expect("publication list JSON");

    assert_eq!(body["schema"], "crablink.publication-page.v1",);

    assert_eq!(body["path"], "/v1/creators/rusty_crab/publications",);

    assert_eq!(body["query"]["cursor"], "p_00000002",);

    assert_eq!(body["query"]["limit"], "20",);

    assert_eq!(body["authorization"], "Bearer publication-read",);

    assert_eq!(body["xCorrelationId"], "corr-phase6b4",);

    assert_eq!(body["xRequestId"], "req-phase6b4",);

    assert!(body["connection"].is_null(),);

    clear_env();
}

#[tokio::test]
async fn phase6b4_detail_route_targets_omnigate_publication_detail() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let omnigate_address = start_dummy_omnigate().await;

    let gateway_address = start_gateway(omnigate_address).await;

    let response = reqwest::Client::new()
        .get(format!(
            "http://{gateway_address}/creators/rusty_crab/publications/publication-001",
        ))
        .send()
        .await
        .expect("gateway publication detail");

    assert_eq!(response.status(), StatusCode::OK,);

    let body: Value = response.json().await.expect("publication detail JSON");

    assert_eq!(body["schema"], "crablink.publication-summary.v1",);

    assert_eq!(
        body["path"],
        "/v1/creators/rusty_crab/publications/publication-001",
    );

    clear_env();
}

#[tokio::test]
async fn phase6b4_upstream_not_found_passes_through() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let omnigate_address = start_dummy_omnigate().await;

    let gateway_address = start_gateway(omnigate_address).await;

    let response = reqwest::Client::new()
        .get(format!(
            "http://{gateway_address}/creators/rusty_crab/publications/missing",
        ))
        .send()
        .await
        .expect("gateway not-found response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND,);

    let body: Value = response.json().await.expect("not-found JSON");

    assert_eq!(body["code"], "not_found",);

    assert_eq!(body["reason"], "publication_not_found",);

    clear_env();
}

#[tokio::test]
async fn phase6b4_upstream_query_rejection_passes_through() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let omnigate_address = start_dummy_omnigate().await;

    let gateway_address = start_gateway(omnigate_address).await;

    let response = reqwest::Client::new()
        .get(format!(
            "http://{gateway_address}/creators/rusty_crab/publications?limit=20&walletBalance=100",
        ))
        .send()
        .await
        .expect("gateway rejected query response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST,);

    let body: Value = response.json().await.expect("query rejection JSON");

    assert_eq!(body["reason"], "unknown_query_field",);

    clear_env();
}

#[tokio::test]
async fn phase6b4_transport_failure_returns_structured_bad_gateway() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let gateway_address = start_gateway("127.0.0.1:9".parse().expect("closed local address")).await;

    let response = reqwest::Client::new()
        .get(format!(
            "http://{gateway_address}/creators/rusty_crab/publications",
        ))
        .send()
        .await
        .expect("gateway transport response");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY,);

    let body: Value = response.json().await.expect("bad gateway JSON");

    assert_eq!(body["code"], "upstream_unavailable",);

    assert_eq!(body["retryable"], true,);

    assert_eq!(body["reason"], "omnigate_connect",);

    clear_env();
}

#[test]
fn phase6b4_gateway_route_surface_is_proxy_only() {
    let product_source = include_str!("../src/routes/product.rs",);

    let compact: String = product_source.split_whitespace().collect();

    for required in [
        "/creators/:username/publications",
        "/creators/:username/publications/:publication_id",
        "get(creator_publications_list)",
        "get(creator_publication_get)",
        "/v1/creators/{username}/publications",
        "/v1/creators/{username}/publications/{publication_id}",
        "proxy_to_omnigate(",
        "with_query(",
    ] {
        assert!(
            compact.contains(required,),
            "missing gateway publication fragment: {required}",
        );
    }

    for forbidden in [
        "wallet_mutation",
        "ledger_mutation",
        "receipt_authority",
        "paid_entitlement_authority",
        "follow_mutation",
        "settlement_authority",
        "private_key",
        "recovery_phrase",
        "pin_value",
        "capability_token",
    ] {
        assert!(
            !product_source.contains(forbidden,),
            "forbidden gateway authority: {forbidden}",
        );
    }
}

#[test]
fn phase6b4_gateway_does_not_bypass_omnigate_for_publications() {
    let product_source = include_str!("../src/routes/product.rs",);

    assert!(!product_source.contains("/v1/index/creators/",),);

    assert!(!product_source.contains("svc_index",),);

    assert!(!product_source.contains("index_client",),);

    assert!(product_source.contains("/v1/creators/{username}/publications",),);
}

fn parse_query(uri: &Uri) -> HashMap<String, String> {
    let mut values = HashMap::new();

    let Some(query) = uri.query() else {
        return values;
    };

    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));

        values.insert(key.to_owned(), value.to_owned());
    }

    values
}

fn grab(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

async fn wait_for_health(url: String) {
    let client = reqwest::Client::new();

    for _ in 0..40 {
        if let Ok(response) = client.get(&url).send().await {
            if response.status().is_success() {
                return;
            }
        }

        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    panic!("dummy omnigate did not become healthy at {url}",);
}

fn clear_env() {
    std::env::remove_var("SVC_GATEWAY_OMNIGATE_BASE_URL");

    std::env::remove_var("SVC_GATEWAY_BIND_ADDR");
}
