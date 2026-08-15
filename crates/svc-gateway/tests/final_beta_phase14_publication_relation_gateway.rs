//! RO:WHAT — Focused svc-gateway tests for durable publication-relation reads.
//! RO:WHY — Proves the public edge forwards relation reads through Omnigate without bypassing product orchestration.
//! RO:INVARIANTS — gateway is proxy-only; raw query, allowed headers, status, and body pass through.
//! RO:SECURITY — no direct svc-index access and no relation, wallet, ledger, receipt, entitlement, follow, or settlement authority.

use std::{
    collections::HashMap,
    net::SocketAddr,
    time::Duration,
};

use axum::{
    http::{
        HeaderMap,
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

const IMAGE_URL: &str =
    "crab://8888888888888888888888888888888888888888888888888888888888888888.image";

fn test_metrics_handles(
) -> metrics::MetricsHandles {
    static CELL:
        OnceCell<
            metrics::MetricsHandles,
        > =
        OnceCell::new();

    CELL.get_or_init(
        || {
            metrics::register()
                .expect(
                    "register gateway metrics once",
                )
        },
    )
    .clone()
}

async fn start_dummy_omnigate(
) -> SocketAddr {
    async fn healthz(
    ) -> &'static str {
        "ok"
    }

    fn parse_query(
        uri:
            &Uri,
    ) -> HashMap<
        String,
        String,
    > {
        let mut query =
            HashMap::new();

        let Some(
            raw_query,
        ) =
            uri.query()
        else {
            return query;
        };

        for pair in raw_query
            .split(
                '&',
            )
        {
            if pair
                .is_empty()
            {
                continue;
            }

            let mut parts =
                pair.splitn(
                    2,
                    '=',
                );

            let raw_key =
                parts
                    .next()
                    .unwrap_or_default();

            if raw_key
                .is_empty()
            {
                continue;
            }

            let raw_value =
                parts
                    .next()
                    .unwrap_or_default();

            let key =
                percent_encoding::
                    percent_decode_str(
                        raw_key,
                    )
                    .decode_utf8_lossy()
                    .into_owned();

            let value =
                percent_encoding::
                    percent_decode_str(
                        raw_value,
                    )
                    .decode_utf8_lossy()
                    .into_owned();

            query.insert(
                key,
                value,
            );
        }

        query
    }

    async fn relation_handler(
        uri:
            Uri,

        headers:
            HeaderMap,
    ) -> (
        StatusCode,
        Json<Value>,
    ) {
        let query =
            parse_query(
                &uri,
            );

        if query
            .contains_key(
                "walletBalance",
            )
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({
                        "code":
                            "invalid_query",
                        "message":
                            "unknown query field",
                        "retryable":
                            false,
                        "reason":
                            "unknown_query_field"
                    }),
                ),
            );
        }

        (
            StatusCode::OK,
            Json(
                serde_json::json!({
                    "schema":
                        "crablink.publication-relation-page.v1",

                    "path":
                        "/v1/publication-relations",

                    "parentCrabUrl":
                        query.get(
                            "parentCrabUrl",
                        ),

                    "cursor":
                        query.get(
                            "cursor",
                        ),

                    "limit":
                        query.get(
                            "limit",
                        ),

                    "authorization":
                        header_value(
                            &headers,
                            "authorization",
                        ),

                    "xCorrelationId":
                        header_value(
                            &headers,
                            "x-correlation-id",
                        ),

                    "xRequestId":
                        header_value(
                            &headers,
                            "x-request-id",
                        ),

                    "connection":
                        header_value(
                            &headers,
                            "connection",
                        )
                }),
            ),
        )
    }

    let router =
        Router::new()
            .route(
                "/healthz",
                get(
                    healthz,
                ),
            )
            .route(
                "/v1/publication-relations",
                get(
                    relation_handler,
                ),
            );

    let listener =
        TcpListener::bind(
            "127.0.0.1:0",
        )
        .await
        .expect(
            "bind dummy omnigate",
        );

    let address =
        listener
            .local_addr()
            .expect(
                "dummy omnigate address",
            );

    tokio::spawn(
        async move {
            axum::serve(
                listener,
                router,
            )
            .await
            .expect(
                "serve dummy omnigate",
            );
        },
    );

    wait_for_health(
        format!(
            "http://{address}/healthz",
        ),
    )
    .await;

    address
}

async fn start_gateway(
    omnigate_address:
        SocketAddr,
) -> SocketAddr {
    std::env::set_var(
        "SVC_GATEWAY_OMNIGATE_BASE_URL",
        format!(
            "http://{omnigate_address}",
        ),
    );

    std::env::set_var(
        "SVC_GATEWAY_BIND_ADDR",
        "127.0.0.1:0",
    );

    let config =
        Config::load()
            .expect(
                "load gateway configuration",
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

    tokio::time::sleep(
        Duration::from_millis(
            50,
        ),
    )
    .await;

    address
}

#[tokio::test]
async fn phase14a6d_gateway_preserves_relation_query_and_headers() {
    let _guard =
        ENV_LOCK
            .lock()
            .await;

    clear_env();

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
                format!(
                    "http://{gateway_address}/publication-relations",
                ),
            )
            .query(
                &[
                    (
                        "parentCrabUrl",
                        IMAGE_URL,
                    ),
                    (
                        "cursor",
                        "r_00000002",
                    ),
                    (
                        "limit",
                        "30",
                    ),
                ],
            )
            .header(
                "authorization",
                "Bearer gateway-relation-read",
            )
            .header(
                "x-correlation-id",
                "corr-gateway-phase14a6d",
            )
            .header(
                "x-request-id",
                "req-gateway-phase14a6d",
            )
            .header(
                "connection",
                "close",
            )
            .send()
            .await
            .expect(
                "gateway relation response",
            );

    assert_eq!(
        response.status(),
        StatusCode::OK,
    );

    let body:
        Value =
        response
            .json()
            .await
            .expect(
                "gateway relation JSON",
            );

    assert_eq!(
        body[
            "schema"
        ],
        "crablink.publication-relation-page.v1",
    );

    assert_eq!(
        body[
            "path"
        ],
        "/v1/publication-relations",
    );

    assert_eq!(
        body[
            "parentCrabUrl"
        ],
        IMAGE_URL,
    );

    assert_eq!(
        body[
            "cursor"
        ],
        "r_00000002",
    );

    assert_eq!(
        body[
            "limit"
        ],
        "30",
    );

    assert_eq!(
        body[
            "authorization"
        ],
        "Bearer gateway-relation-read",
    );

    assert_eq!(
        body[
            "xCorrelationId"
        ],
        "corr-gateway-phase14a6d",
    );

    assert_eq!(
        body[
            "xRequestId"
        ],
        "req-gateway-phase14a6d",
    );

    assert!(
        body[
            "connection"
        ]
        .is_null(),
    );

    clear_env();
}

#[tokio::test]
async fn phase14a6d_gateway_passes_omnigate_query_rejection_through() {
    let _guard =
        ENV_LOCK
            .lock()
            .await;

    clear_env();

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
                format!(
                    "http://{gateway_address}/publication-relations",
                ),
            )
            .query(
                &[
                    (
                        "parentCrabUrl",
                        IMAGE_URL,
                    ),
                    (
                        "walletBalance",
                        "100",
                    ),
                ],
            )
            .send()
            .await
            .expect(
                "gateway rejected relation query",
            );

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
    );

    let body:
        Value =
        response
            .json()
            .await
            .expect(
                "gateway rejected relation JSON",
            );

    assert_eq!(
        body[
            "reason"
        ],
        "unknown_query_field",
    );

    clear_env();
}

#[tokio::test]
async fn phase14a6d_gateway_transport_failure_is_structured_bad_gateway() {
    let _guard =
        ENV_LOCK
            .lock()
            .await;

    clear_env();

    let gateway_address =
        start_gateway(
            "127.0.0.1:9"
                .parse()
                .expect(
                    "closed local address",
                ),
        )
        .await;

    let response =
        reqwest::Client::new()
            .get(
                format!(
                    "http://{gateway_address}/publication-relations",
                ),
            )
            .query(
                &[
                    (
                        "parentCrabUrl",
                        IMAGE_URL,
                    ),
                ],
            )
            .send()
            .await
            .expect(
                "gateway relation transport response",
            );

    assert_eq!(
        response.status(),
        StatusCode::BAD_GATEWAY,
    );

    let body:
        Value =
        response
            .json()
            .await
            .expect(
                "gateway relation bad gateway JSON",
            );

    assert_eq!(
        body[
            "code"
        ],
        "upstream_unavailable",
    );

    assert_eq!(
        body[
            "reason"
        ],
        "omnigate_connect",
    );

    clear_env();
}

#[test]
fn phase14a6d_gateway_relation_surface_is_proxy_only() {
    let product_source =
        include_str!(
            "../src/routes/product.rs",
        );

    let compact:
        String =
        product_source
            .split_whitespace()
            .collect();

    for required in [
        "/publication-relations",
        "get(publication_relations_list)",
        "/v1/publication-relations",
        "with_query(",
        "proxy_to_omnigate(",
    ] {
        assert!(
            compact.contains(
                required,
            ),
            "missing gateway relation fragment: {required}",
        );
    }

    for forbidden in [
        "/v1/index/publication-relations",
        "svc_index",
        "index_client",
        "wallet_mutation",
        "ledger_mutation",
        "receipt_authority",
        "paid_entitlement_authority",
        "settlement_authority",
        "follow_mutation",
        "private_key",
        "recovery_phrase",
        "pin_value",
        "capability_token",
    ] {
        assert_eq!(
            product_source.contains(
                forbidden,
            ),
            false,
            "gateway relation read must not gain forbidden authority: {forbidden}",
        );
    }
}

fn header_value(
    headers:
        &HeaderMap,

    name:
        &str,
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
            str::trim,
        )
        .filter(
            |value| {
                value
                    .is_empty()
                    == false
            },
        )
        .map(
            ToOwned::to_owned,
        )
}

async fn wait_for_health(
    url:
        String,
) {
    let client =
        reqwest::Client::new();

    for _ in 0..40 {
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
                25,
            ),
        )
        .await;
    }

    panic!(
        "dummy omnigate did not become healthy at {url}",
    );
}

fn clear_env(
) {
    std::env::remove_var(
        "SVC_GATEWAY_OMNIGATE_BASE_URL",
    );

    std::env::remove_var(
        "SVC_GATEWAY_BIND_ADDR",
    );
}
