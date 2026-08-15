//! RO:WHAT — Focused Omnigate tests for durable publication-relation reads.
//! RO:WHY — Proves strict bounded forwarding to svc-index without giving Omnigate relation-storage or economic authority.

use std::{
    collections::HashMap,
    net::SocketAddr,
    time::Duration,
};

use axum::{
    extract::Query,
    http::{
        HeaderMap,
        StatusCode,
    },
    routing::get,
    Json,
    Router,
};
use serde_json::Value;
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

async fn start_dummy_index(
) -> SocketAddr {
    async fn healthz(
    ) -> &'static str {
        "ok"
    }

    async fn relation_handler(
        Query(
            query,
        ):
            Query<
                HashMap<
                    String,
                    String,
                >,
            >,

        headers:
            HeaderMap,
    ) -> (
        StatusCode,
        Json<Value>,
    ) {
        if query
            .get(
                "parentCrabUrl",
            )
            .is_none()
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({
                        "code":
                            "invalid_relation_parent",
                        "message":
                            "parent is required",
                        "retryable":
                            false,
                        "reason":
                            "missing_parent"
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

                    "items": [
                        {
                            "schema":
                                "crablink.publication-relation.v1",

                            "publication": {
                                "publicationId":
                                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",

                                "kind":
                                    "comment",

                                "crabUrl":
                                    "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.comment",

                                "title":
                                    "Comment",

                                "summary":
                                    "Durable imageboard reply",

                                "creatorDisplay":
                                    "@alice",

                                "createdAtMs":
                                    1001,

                                "visibility":
                                    "public_preview",

                                "references": {
                                    "manifestCid":
                                        "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",

                                    "contentCid":
                                        "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",

                                    "siteUrl":
                                        "crab://picture-board"
                                }
                            },

                            "parentCrabUrl":
                                query.get(
                                    "parentCrabUrl",
                                ),

                            "threadCrabUrl":
                                query.get(
                                    "parentCrabUrl",
                                ),

                            "siteCrabUrl":
                                "crab://picture-board"
                        }
                    ],

                    "nextCursor":
                        query.get(
                            "cursor",
                        ),

                    "hasMore":
                        false,

                    "receivedLimit":
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
                "/v1/index/publication-relations",
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
            "bind dummy index",
        );

    let address =
        listener
            .local_addr()
            .expect(
                "dummy index address",
            );

    tokio::spawn(
        async move {
            axum::serve(
                listener,
                router,
            )
            .await
            .expect(
                "serve dummy relation index",
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

async fn start_omnigate(
    index_address:
        SocketAddr,
) -> SocketAddr {
    std::env::set_var(
        "OMNIGATE_INDEX_BASE_URL",
        format!(
            "http://{index_address}",
        ),
    );

    let router =
        Router::new()
            .nest(
                "/v1",
                omnigate::routes::v1::router(),
            );

    let listener =
        TcpListener::bind(
            "127.0.0.1:0",
        )
        .await
        .expect(
            "bind omnigate",
        );

    let address =
        listener
            .local_addr()
            .expect(
                "omnigate address",
            );

    tokio::spawn(
        async move {
            axum::serve(
                listener,
                router,
            )
            .await
            .expect(
                "serve omnigate",
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
async fn phase14a6d_omnigate_proxies_parent_cursor_limit_and_headers() {
    let _guard =
        ENV_LOCK
            .lock()
            .await;

    clear_env();

    let index_address =
        start_dummy_index()
            .await;

    let omnigate_address =
        start_omnigate(
            index_address,
        )
        .await;

    let response =
        reqwest::Client::new()
            .get(
                format!(
                    "http://{omnigate_address}/v1/publication-relations",
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
                        "r_00000001",
                    ),
                    (
                        "limit",
                        "25",
                    ),
                ],
            )
            .header(
                "authorization",
                "Bearer relation-read",
            )
            .header(
                "x-correlation-id",
                "corr-phase14a6d",
            )
            .header(
                "x-request-id",
                "req-phase14a6d",
            )
            .header(
                "connection",
                "close",
            )
            .send()
            .await
            .expect(
                "omnigate relation read",
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
                "relation page JSON",
            );

    assert_eq!(
        body[
            "schema"
        ],
        "crablink.publication-relation-page.v1",
    );

    assert_eq!(
        body[
            "items"
        ][
            0
        ][
            "parentCrabUrl"
        ],
        IMAGE_URL,
    );

    assert_eq!(
        body[
            "items"
        ][
            0
        ][
            "threadCrabUrl"
        ],
        IMAGE_URL,
    );

    assert_eq!(
        body[
            "nextCursor"
        ],
        "r_00000001",
    );

    assert_eq!(
        body[
            "receivedLimit"
        ],
        "25",
    );

    assert_eq!(
        body[
            "authorization"
        ],
        "Bearer relation-read",
    );

    assert_eq!(
        body[
            "xCorrelationId"
        ],
        "corr-phase14a6d",
    );

    assert_eq!(
        body[
            "xRequestId"
        ],
        "req-phase14a6d",
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
async fn phase14a6d_unknown_query_fields_fail_before_index() {
    let _guard =
        ENV_LOCK
            .lock()
            .await;

    clear_env();

    std::env::set_var(
        "OMNIGATE_INDEX_BASE_URL",
        "http://127.0.0.1:9",
    );

    let router =
        Router::new()
            .nest(
                "/v1",
                omnigate::routes::v1::router(),
            );

    let listener =
        TcpListener::bind(
            "127.0.0.1:0",
        )
        .await
        .expect(
            "bind omnigate",
        );

    let address =
        listener
            .local_addr()
            .expect(
                "omnigate address",
            );

    tokio::spawn(
        async move {
            axum::serve(
                listener,
                router,
            )
            .await
            .expect(
                "serve omnigate",
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
                format!(
                    "http://{address}/v1/publication-relations",
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
                "unknown relation query response",
            );

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
    );

    clear_env();
}

#[tokio::test]
async fn phase14a6d_parent_query_is_required() {
    let _guard =
        ENV_LOCK
            .lock()
            .await;

    clear_env();

    std::env::set_var(
        "OMNIGATE_INDEX_BASE_URL",
        "http://127.0.0.1:9",
    );

    let router =
        Router::new()
            .nest(
                "/v1",
                omnigate::routes::v1::router(),
            );

    let listener =
        TcpListener::bind(
            "127.0.0.1:0",
        )
        .await
        .expect(
            "bind omnigate",
        );

    let address =
        listener
            .local_addr()
            .expect(
                "omnigate address",
            );

    tokio::spawn(
        async move {
            axum::serve(
                listener,
                router,
            )
            .await
            .expect(
                "serve omnigate",
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
                format!(
                    "http://{address}/v1/publication-relations?limit=25",
                ),
            )
            .send()
            .await
            .expect(
                "missing relation parent response",
            );

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
    );

    clear_env();
}

#[tokio::test]
async fn phase14a6d_index_transport_failure_is_structured_bad_gateway() {
    let _guard =
        ENV_LOCK
            .lock()
            .await;

    clear_env();

    std::env::set_var(
        "OMNIGATE_INDEX_BASE_URL",
        "http://127.0.0.1:9",
    );

    let router =
        Router::new()
            .nest(
                "/v1",
                omnigate::routes::v1::router(),
            );

    let listener =
        TcpListener::bind(
            "127.0.0.1:0",
        )
        .await
        .expect(
            "bind omnigate",
        );

    let address =
        listener
            .local_addr()
            .expect(
                "omnigate address",
            );

    tokio::spawn(
        async move {
            axum::serve(
                listener,
                router,
            )
            .await
            .expect(
                "serve omnigate",
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
                format!(
                    "http://{address}/v1/publication-relations",
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
                "relation transport response",
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
                "relation bad gateway JSON",
            );

    assert_eq!(
        body[
            "code"
        ],
        "index_upstream",
    );

    assert_eq!(
        body[
            "retryable"
        ],
        true,
    );

    clear_env();
}

#[test]
fn phase14a6d_omnigate_route_surface_is_read_only_proxy() {
    let route_source =
        include_str!(
            "../src/routes/v1/publication_relations.rs",
        );

    let router_source =
        include_str!(
            "../src/routes/v1/mod.rs",
        );

    let compact_router:
        String =
        router_source
            .split_whitespace()
            .collect();

    for required in [
        "pubmodpublication_relations;",
        "/publication-relations",
        "publication_relations::list_publication_relations",
    ] {
        assert!(
            compact_router.contains(
                required,
            ),
            "missing Omnigate relation route fragment: {required}",
        );
    }

    for required in [
        "/v1/index/publication-relations",
        "parentCrabUrl",
        "cursor",
        "limit",
        "proxy only",
    ] {
        assert!(
            route_source.contains(
                required,
            ),
            "missing Omnigate relation proxy fragment: {required}",
        );
    }

    for forbidden in [
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
        "quickchain_finality",
        "rox_settlement",
        "solana_mutation",
    ] {
        assert_eq!(
            route_source.contains(
                forbidden,
            ),
            false,
            "forbidden Omnigate relation authority: {forbidden}",
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
        "dummy index did not become healthy at {url}",
    );
}

fn clear_env(
) {
    std::env::remove_var(
        "OMNIGATE_INDEX_BASE_URL",
    );

    std::env::remove_var(
        "OMNIGATE_DOWNSTREAM_INDEX_BASE_URL",
    );
}
