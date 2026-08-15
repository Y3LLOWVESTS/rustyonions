//! RO:WHAT — Read-only Omnigate façade for durable publication relations.
//! RO:WHY — FINAL_BETA Phase 14 keeps product clients away from direct svc-index access while exposing durable Comment thread projection.
//! RO:INTERACTS — svc-index `/v1/index/publication-relations`, svc-gateway `/publication-relations`, later CrabLink Imageboard reader.
//! RO:INVARIANTS — strict query shape; parent is required; cursor and limit are optional; upstream status and body pass through.
//! RO:SECURITY — proxy only; no publication mutation, wallet, ledger, receipt, entitlement, settlement, follow, key, PIN, recovery, QuickChain, ROX, or Solana authority.
//! RO:TEST — final_beta_phase14_publication_relation_read.rs.

// FINAL_BETA_PHASE14A6D_OMNIGATE_RELATION_READ_V1

use axum::{
    body::Bytes,
    extract::Query,
    http::{
        header,
        HeaderMap,
        HeaderName,
        HeaderValue,
        StatusCode,
    },
    response::{
        IntoResponse,
        Response,
    },
    Json,
};
use once_cell::sync::Lazy;
use serde::{
    Deserialize,
    Serialize,
};
use std::{
    env,
    time::Duration,
};

const DEFAULT_INDEX_BASE_URL: &str =
    "http://127.0.0.1:5304";

static HTTP_CLIENT:
    Lazy<reqwest::Client> =
    Lazy::new(
        || {
            reqwest::Client::builder()
                .pool_idle_timeout(
                    Duration::from_secs(
                        30,
                    ),
                )
                .tcp_keepalive(
                    Duration::from_secs(
                        30,
                    ),
                )
                .use_rustls_tls()
                .build()
                .expect(
                    "omnigate publication relation client should build",
                )
        },
    );

#[derive(
    Debug,
    Clone,
    Deserialize,
    PartialEq,
    Eq,
)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase"
)]
pub struct PublicationRelationQuery {
    pub parent_crab_url:
        String,

    #[serde(default)]
    pub cursor:
        Option<String>,

    #[serde(default)]
    pub limit:
        Option<usize>,
}

/// GET /v1/publication-relations?parentCrabUrl=...
///
/// Omnigate owns only the strict façade contract. svc-index remains the
/// canonical relation validation, visibility-projection, ordering, and
/// pagination owner.
pub async fn list_publication_relations(
    Query(
        query,
    ):
        Query<PublicationRelationQuery>,

    headers:
        HeaderMap,
) -> Response {
    proxy_to_index(
        query,
        headers,
    )
    .await
}

async fn proxy_to_index(
    query:
        PublicationRelationQuery,

    headers:
        HeaderMap,
) -> Response {
    let raw_url =
        format!(
            "{}/v1/index/publication-relations",
            index_base_url()
                .trim_end_matches(
                    '/',
                ),
        );

    let mut url =
        match reqwest::Url::parse(
            &raw_url,
        ) {
            Ok(
                url,
            ) => {
                url
            }

            Err(
                _,
            ) => {
                return problem(
                    StatusCode::BAD_GATEWAY,
                    "index_upstream_url",
                    "svc-index upstream URL is invalid",
                    true,
                    "index_upstream_url_invalid",
                );
            }
        };

    {
        let mut pairs =
            url.query_pairs_mut();

        pairs.append_pair(
            "parentCrabUrl",
            &query.parent_crab_url,
        );

        if let Some(
            cursor,
        ) =
            query.cursor
                .as_deref()
        {
            pairs.append_pair(
                "cursor",
                cursor,
            );
        }

        if let Some(
            limit,
        ) =
            query.limit
        {
            pairs.append_pair(
                "limit",
                &limit.to_string(),
            );
        }
    }

    let mut request =
        HTTP_CLIENT
            .get(
                url,
            )
            .timeout(
                Duration::from_secs(
                    5,
                ),
            );

    for (
        name,
        value,
    ) in forwarded_headers(
        &headers,
    ) {
        request =
            request.header(
                name,
                value,
            );
    }

    let upstream =
        match request
            .send()
            .await
        {
            Ok(
                response,
            ) => {
                response
            }

            Err(
                error,
            ) => {
                let reason =
                    if error
                        .is_timeout()
                    {
                        "index_upstream_timeout"
                    } else if error
                        .is_connect()
                    {
                        "index_upstream_connect"
                    } else {
                        "index_upstream_request"
                    };

                return problem(
                    StatusCode::BAD_GATEWAY,
                    "index_upstream",
                    "svc-index upstream request failed",
                    true,
                    reason,
                );
            }
        };

    let status =
        upstream
            .status();

    let content_type =
        upstream
            .headers()
            .get(
                header::CONTENT_TYPE,
            )
            .cloned()
            .unwrap_or_else(
                || {
                    HeaderValue::from_static(
                        "application/json",
                    )
                },
            );

    let body =
        match upstream
            .bytes()
            .await
        {
            Ok(
                body,
            ) => {
                body
            }

            Err(
                _,
            ) => {
                return problem(
                    StatusCode::BAD_GATEWAY,
                    "index_upstream_body",
                    "svc-index upstream relation body could not be read",
                    true,
                    "index_upstream_body_read",
                );
            }
        };

    proxy_response(
        status,
        content_type,
        body,
    )
}

fn proxy_response(
    status:
        StatusCode,

    content_type:
        HeaderValue,

    body:
        Bytes,
) -> Response {
    let mut response =
        Response::builder()
            .status(
                status,
            );

    if let Some(
        headers,
    ) =
        response
            .headers_mut()
    {
        headers.insert(
            header::CONTENT_TYPE,
            content_type,
        );
    }

    response
        .body(
            axum::body::Body::from(
                body,
            ),
        )
        .unwrap_or_else(
            |_| {
                problem(
                    StatusCode::BAD_GATEWAY,
                    "index_response_build",
                    "omnigate could not build relation proxy response",
                    true,
                    "response_build_failed",
                )
            },
        )
}

fn index_base_url(
) -> String {
    env::var(
        "OMNIGATE_INDEX_BASE_URL",
    )
    .or_else(
        |_| {
            env::var(
                "OMNIGATE_DOWNSTREAM_INDEX_BASE_URL",
            )
        },
    )
    .ok()
    .map(
        |value| {
            value
                .trim()
                .trim_end_matches(
                    '/',
                )
                .to_owned()
        },
    )
    .filter(
        |value| {
            value
                .is_empty()
                == false
        },
    )
    .unwrap_or_else(
        || {
            DEFAULT_INDEX_BASE_URL
                .to_owned()
        },
    )
}

fn forwarded_headers(
    headers:
        &HeaderMap,
) -> Vec<(
    HeaderName,
    HeaderValue,
)> {
    headers
        .iter()
        .filter(
            |(
                name,
                _,
            )| {
                should_forward_header(
                    name,
                )
            },
        )
        .map(
            |(
                name,
                value,
            )| {
                (
                    name.clone(),
                    value.clone(),
                )
            },
        )
        .collect()
}

fn should_forward_header(
    name:
        &HeaderName,
) -> bool {
    if name
        == header::HOST
        || name
            == header::CONNECTION
        || name
            == header::CONTENT_LENGTH
        || name
            == header::TRANSFER_ENCODING
        || name
            == header::UPGRADE
        || name
            .as_str()
            .eq_ignore_ascii_case(
                "proxy-authorization",
            )
        || name
            .as_str()
            .eq_ignore_ascii_case(
                "te",
            )
        || name
            .as_str()
            .eq_ignore_ascii_case(
                "trailer",
            )
    {
        return false;
    }

    name
        == header::AUTHORIZATION
        || name
            == header::ACCEPT
        || name
            .as_str()
            .eq_ignore_ascii_case(
                "x-correlation-id",
            )
        || name
            .as_str()
            .eq_ignore_ascii_case(
                "x-request-id",
            )
}

#[derive(
    Debug,
    Serialize,
)]
struct Problem<'a> {
    code:
        &'a str,

    message:
        &'a str,

    retryable:
        bool,

    reason:
        &'a str,
}

fn problem(
    status:
        StatusCode,

    code:
        &'static str,

    message:
        &'static str,

    retryable:
        bool,

    reason:
        &'static str,
) -> Response {
    (
        status,
        Json(
            Problem {
                code,
                message,
                retryable,
                reason,
            },
        ),
    )
        .into_response()
}
