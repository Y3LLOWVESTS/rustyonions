// RO:WHAT — Focused public-edge proxy proof for CrabLink Explore discovery.
// RO:WHY — FINAL_BETA Phase 10 requires clients to reach Omnigate discovery through svc-gateway.
// RO:INTERACTS — real svc-gateway product router plus dummy Omnigate /v1/explore.
// RO:INVARIANTS — gateway preserves query, selected headers, upstream response, and remains proxy-only.
// RO:SECURITY — no discovery ranking, social graph, wallet, ledger, receipt, entitlement, QuickChain, ROX, or Solana authority.

// FINAL_BETA_PHASE10A3D_SVC_GATEWAY_EXPLORE_DISCOVERY_PROXY_V1

use std::{
    collections::HashMap,
    error::Error,
    net::SocketAddr,
    time::Duration,
};

use axum::{
    body::Bytes,
    http::{
        HeaderMap,
        Method,
        StatusCode,
        Uri,
    },
    routing::get,
    Json,
    Router,
};

use once_cell::sync::OnceCell;
use serde_json::Value;
use svc_gateway::{
    config::Config,
    observability::metrics,
    routes,
    state::AppState,
};
use tokio::{
    net::TcpListener,
    sync::Mutex,
};

static ENV_LOCK:
    Mutex<()> =
        Mutex::const_new(
            (),
        );

type TestResult =
    Result<
        (),
        Box<dyn Error>,
    >;

fn ensure(
    condition: bool,
    message: &str,
) -> TestResult {
    if condition {
        return Ok(
            (),
        );
    }

    Err(
        std::io::Error::other(
            message,
        )
        .into(),
    )
}

fn test_metrics_handles(
) -> metrics::MetricsHandles {
    static CELL:
        OnceCell<
            metrics::MetricsHandles,
        > =
        OnceCell::new();

    CELL
        .get_or_init(
            || {
                metrics::register()
                    .expect(
                        "register gateway metrics once",
                    )
            },
        )
        .clone()
}

async fn dummy_explore(
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    _body: Bytes,
) -> (
    StatusCode,
    Json<Value>,
) {
    let query =
        parse_query(
            &uri,
        );

    if query
        .get(
            "mode",
        )
        .map(
            String::as_str,
        )
        == Some(
            "problem400",
        )
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({
                    "code":
                        "invalid_explore_query",
                    "message":
                        "dummy Omnigate rejection",
                    "retryable":
                        false
                }),
            ),
        );
    }

    (
        StatusCode::OK,
        Json(
            serde_json::json!({
                "schema":
                    "crablink.explore-discovery.v1",
                "recentPublications":
                    [],
                "publicCreators":
                    [],
                "templateSites":
                    [],
                "observed": {
                    "method":
                        method.as_str(),
                    "path":
                        uri.path(),
                    "query":
                        query,
                    "authorization":
                        grab(
                            &headers,
                            "authorization",
                        ),
                    "correlationId":
                        grab(
                            &headers,
                            "x-correlation-id",
                        ),
                    "connection":
                        grab(
                            &headers,
                            "connection",
                        )
                }
            }),
        ),
    )
}

async fn start_dummy_omnigate(
) -> SocketAddr {
    let router =
        Router::new()
            .route(
                "/v1/explore",
                get(
                    dummy_explore,
                ),
            );

    let listener =
        TcpListener::bind(
            "127.0.0.1:0",
        )
        .await
        .expect(
            "bind dummy Omnigate",
        );

    let address =
        listener
            .local_addr()
            .expect(
                "dummy Omnigate address",
            );

    tokio::spawn(
        async move {
            axum::serve(
                listener,
                router,
            )
            .await
            .expect(
                "serve dummy Omnigate",
            );
        },
    );

    address
}

async fn start_gateway(
    omnigate_address:
        SocketAddr,
) -> SocketAddr {
    std::env::set_var(
        "SVC_GATEWAY_OMNIGATE_BASE_URL",
        [
            "http://",
            &omnigate_address
                .to_string(),
        ]
        .concat(),
    );

    std::env::set_var(
        "SVC_GATEWAY_BIND_ADDR",
        "127.0.0.1:0",
    );

    let config =
        Config::load()
            .expect(
                "load gateway config",
            );

    let state =
        AppState::new(
            config.clone(),
            test_metrics_handles(),
        );

    let router =
        routes::build_router(
            &state,
        );

    let listener =
        TcpListener::bind(
            &config
                .server
                .bind_addr,
        )
        .await
        .expect(
            "bind gateway",
        );

    let address =
        listener
            .local_addr()
            .expect(
                "gateway address",
            );

    tokio::spawn(
        async move {
            axum::serve(
                listener,
                router,
            )
            .await
            .expect(
                "serve gateway",
            );
        },
    );

    wait_for_health(
        [
            "http://",
            &address
                .to_string(),
            "/healthz",
        ]
        .concat(),
    )
    .await;

    address
}

#[tokio::test]
async fn phase10a3d_public_explore_targets_omnigate(
) -> TestResult {
    let _guard =
        ENV_LOCK
            .lock()
            .await;

    clear_gateway_env();

    let omnigate_address =
        start_dummy_omnigate()
            .await;

    let gateway_address =
        start_gateway(
            omnigate_address,
        )
        .await;

    let response =
        reqwest::Client::new()
            .get(
                [
                    "http://",
                    &gateway_address
                        .to_string(),
                    "/explore",
                ]
                .concat(),
            )
            .send()
            .await?;

    ensure(
        response.status()
            == StatusCode::OK,
        "public Explore route must succeed",
    )?;

    let body:
        Value =
            response
                .json()
                .await?;

    ensure(
        body.get(
            "schema",
        )
        .and_then(
            Value::as_str,
        )
        == Some(
            "crablink.explore-discovery.v1",
        ),
        "Explore schema changed through gateway",
    )?;

    ensure(
        body.pointer(
            "/observed/path",
        )
        .and_then(
            Value::as_str,
        )
        == Some(
            "/v1/explore",
        ),
        "gateway did not target Omnigate Explore",
    )?;

    clear_gateway_env();

    Ok(())
}

#[tokio::test]
async fn phase10a3d_explore_query_is_preserved_for_omnigate_validation(
) -> TestResult {
    let _guard =
        ENV_LOCK
            .lock()
            .await;

    clear_gateway_env();

    let omnigate_address =
        start_dummy_omnigate()
            .await;

    let gateway_address =
        start_gateway(
            omnigate_address,
        )
        .await;

    let response =
        reqwest::Client::new()
            .get(
                [
                    "http://",
                    &gateway_address
                        .to_string(),
                    "/explore",
                    "?publicationLimit=24",
                    "&creatorLimit=20",
                    "&siteLimit=16",
                ]
                .concat(),
            )
            .send()
            .await?;

    ensure(
        response.status()
            == StatusCode::OK,
        "bounded Explore query must reach Omnigate",
    )?;

    let body:
        Value =
            response
                .json()
                .await?;

    ensure(
        body.pointer(
            "/observed/query/publicationLimit",
        )
        .and_then(
            Value::as_str,
        )
        == Some(
            "24",
        ),
        "publicationLimit changed in gateway",
    )?;

    ensure(
        body.pointer(
            "/observed/query/creatorLimit",
        )
        .and_then(
            Value::as_str,
        )
        == Some(
            "20",
        ),
        "creatorLimit changed in gateway",
    )?;

    ensure(
        body.pointer(
            "/observed/query/siteLimit",
        )
        .and_then(
            Value::as_str,
        )
        == Some(
            "16",
        ),
        "siteLimit changed in gateway",
    )?;

    clear_gateway_env();

    Ok(())
}

#[tokio::test]
async fn phase10a3d_selected_headers_forward_and_hop_by_hop_header_does_not(
) -> TestResult {
    let _guard =
        ENV_LOCK
            .lock()
            .await;

    clear_gateway_env();

    let omnigate_address =
        start_dummy_omnigate()
            .await;

    let gateway_address =
        start_gateway(
            omnigate_address,
        )
        .await;

    let response =
        reqwest::Client::new()
            .get(
                [
                    "http://",
                    &gateway_address
                        .to_string(),
                    "/explore",
                ]
                .concat(),
            )
            .header(
                "authorization",
                "Bearer dev",
            )
            .header(
                "x-correlation-id",
                "phase10a3d-correlation",
            )
            .header(
                "connection",
                "close",
            )
            .send()
            .await?;

    ensure(
        response.status()
            == StatusCode::OK,
        "Explore header proxy request failed",
    )?;

    let body:
        Value =
            response
                .json()
                .await?;

    ensure(
        body.pointer(
            "/observed/authorization",
        )
        .and_then(
            Value::as_str,
        )
        == Some(
            "Bearer dev",
        ),
        "authorization header was not forwarded",
    )?;

    ensure(
        body.pointer(
            "/observed/correlationId",
        )
        .and_then(
            Value::as_str,
        )
        .is_some(),
        "correlation header missing upstream",
    )?;

    ensure(
        body.pointer(
            "/observed/connection",
        )
        .map(
            Value::is_null,
        )
        == Some(
            true,
        ),
        "hop-by-hop connection header reached Omnigate",
    )?;

    clear_gateway_env();

    Ok(())
}

#[tokio::test]
async fn phase10a3d_omnigate_error_status_and_body_pass_through(
) -> TestResult {
    let _guard =
        ENV_LOCK
            .lock()
            .await;

    clear_gateway_env();

    let omnigate_address =
        start_dummy_omnigate()
            .await;

    let gateway_address =
        start_gateway(
            omnigate_address,
        )
        .await;

    let response =
        reqwest::Client::new()
            .get(
                [
                    "http://",
                    &gateway_address
                        .to_string(),
                    "/explore?mode=problem400",
                ]
                .concat(),
            )
            .send()
            .await?;

    ensure(
        response.status()
            == StatusCode::BAD_REQUEST,
        "Omnigate rejection status changed at gateway",
    )?;

    let body:
        Value =
            response
                .json()
                .await?;

    ensure(
        body.get(
            "code",
        )
        .and_then(
            Value::as_str,
        )
        == Some(
            "invalid_explore_query",
        ),
        "Omnigate rejection body changed at gateway",
    )?;

    clear_gateway_env();

    Ok(())
}

#[tokio::test]
async fn phase10a3d_omnigate_transport_failure_is_structured_gateway_502(
) -> TestResult {
    let _guard =
        ENV_LOCK
            .lock()
            .await;

    clear_gateway_env();

    std::env::set_var(
        "SVC_GATEWAY_OMNIGATE_BASE_URL",
        "http://127.0.0.1:1",
    );

    std::env::set_var(
        "SVC_GATEWAY_BIND_ADDR",
        "127.0.0.1:0",
    );

    let config =
        Config::load()
            .expect(
                "load unavailable Omnigate config",
            );

    let state =
        AppState::new(
            config.clone(),
            test_metrics_handles(),
        );

    let listener =
        TcpListener::bind(
            &config
                .server
                .bind_addr,
        )
        .await
        .expect(
            "bind failure-path gateway",
        );

    let gateway_address =
        listener
            .local_addr()
            .expect(
                "failure-path gateway address",
            );

    tokio::spawn(
        async move {
            axum::serve(
                listener,
                routes::build_router(
                    &state,
                ),
            )
            .await
            .expect(
                "serve failure-path gateway",
            );
        },
    );

    tokio::time::sleep(
        Duration::from_millis(
            50,
        ),
    )
    .await;

    let response =
        reqwest::Client::new()
            .get(
                [
                    "http://",
                    &gateway_address
                        .to_string(),
                    "/explore",
                ]
                .concat(),
            )
            .send()
            .await?;

    ensure(
        response.status()
            == StatusCode::BAD_GATEWAY,
        "Omnigate transport failure must become gateway 502",
    )?;

    let body:
        Value =
            response
                .json()
                .await?;

    ensure(
        body.get(
            "code",
        )
        .and_then(
            Value::as_str,
        )
        == Some(
            "upstream_unavailable",
        ),
        "gateway upstream failure code mismatch",
    )?;

    ensure(
        body.get(
            "retryable",
        )
        .and_then(
            Value::as_bool,
        )
        == Some(
            true,
        ),
        "gateway upstream failure retry posture mismatch",
    )?;

    clear_gateway_env();

    Ok(())
}

#[test]
fn phase10a3d_gateway_explore_surface_remains_proxy_only(
) -> TestResult {
    let source =
        std::fs::read_to_string(
            "src/routes/product.rs",
        )?;

    ensure(
        source.contains(
            "FINAL_BETA_PHASE10A3D_SVC_GATEWAY_EXPLORE_DISCOVERY_ROUTE_V1",
        ),
        "Phase 10A3D source marker missing",
    )?;

    let compact_source =
        source
            .split_whitespace()
            .collect::<String>();

    ensure(
        compact_source.contains(
            ".route(\"/explore\",get(explore_discovery),)",
        ),
        "public gateway Explore route missing",
    )?;

    ensure(
        source.contains(
            "\"/v1/explore\"",
        ),
        "Omnigate Explore upstream missing",
    )?;

    ensure(
        source.contains(
            "Omnigate owns the strict reviewed Explore query boundary",
        ),
        "gateway versus Omnigate query authority boundary missing",
    )?;

    ensure(
        source.contains(
            "read `svc-index` directly",
        ),
        "direct svc-index non-authority boundary missing",
    )?;

    Ok(())
}

fn parse_query(
    uri: &Uri,
) -> HashMap<
    String,
    String,
> {
    uri
        .query()
        .unwrap_or_default()
        .split(
            '&',
        )
        .filter(
            |pair| {
                pair
                    .is_empty()
                    == false
            },
        )
        .filter_map(
            |pair| {
                let pair =
                    pair.split_once(
                        '=',
                    )?;

                Some((
                    pair
                        .0
                        .to_owned(),
                    pair
                        .1
                        .to_owned(),
                ))
            },
        )
        .collect()
}

fn grab(
    headers: &HeaderMap,
    name: &str,
) -> Option<String> {
    headers
        .get(
            name,
        )
        .and_then(
            |value| {
                value
                    .to_str()
                    .ok()
            },
        )
        .map(
            str::to_owned,
        )
}

async fn wait_for_health(
    url: String,
) {
    let client =
        reqwest::Client::new();

    for _ in 0..50 {
        if let Ok(
            response,
        ) =
            client
                .get(
                    &url,
                )
                .send()
                .await
        {
            if response
                .status()
                .is_success()
            {
                return;
            }
        }

        tokio::time::sleep(
            Duration::from_millis(
                10,
            ),
        )
        .await;
    }

    panic!(
        "gateway did not become healthy at {url}"
    );
}

fn clear_gateway_env(
) {
    std::env::remove_var(
        "SVC_GATEWAY_OMNIGATE_BASE_URL",
    );

    std::env::remove_var(
        "SVC_GATEWAY_STORAGE_BASE_URL",
    );

    std::env::remove_var(
        "SVC_GATEWAY_BIND_ADDR",
    );
}
