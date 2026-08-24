// RO:WHAT — Omnigate read-only facade for FINAL_BETA Explore discovery.
// RO:WHY — CrabLink must consume discovery through the reviewed product coordinator rather than call svc-index directly.
// RO:INTERACTS — svc-index GET /v1/index/explore, later svc-gateway Explore proxy, desktop discovery adapter.
// RO:INVARIANTS — proxy only; strict bounded query fields; upstream status and body pass through unchanged.
// RO:SECURITY — public read projection only; no graph, ranking, wallet, ledger, receipt, entitlement, QuickChain, ROX, or Solana authority.
// RO:TEST — final_beta_phase10_explore_discovery_route.rs.

// FINAL_BETA_PHASE10A3C_OMNIGATE_EXPLORE_DISCOVERY_ROUTE_V1

use axum::{
    body::{Body, Bytes},
    extract::Query,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{env, time::Duration};

const DEFAULT_INDEX_BASE_URL: &str = "http://127.0.0.1:5304";

const DEFAULT_PUBLICATION_LIMIT: usize = 12;

const MAX_PUBLICATION_LIMIT: usize = 24;

const DEFAULT_CREATOR_LIMIT: usize = 12;

const MAX_CREATOR_LIMIT: usize = 24;

const DEFAULT_SITE_LIMIT: usize = 8;

const MAX_SITE_LIMIT: usize = 16;

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate Explore client should build")
});

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExploreDiscoveryQuery {
    #[serde(default)]
    pub publication_limit: Option<usize>,

    #[serde(default)]
    pub creator_limit: Option<usize>,

    #[serde(default)]
    pub site_limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpstreamProblem {
    code: &'static str,

    message: &'static str,

    retryable: bool,
}

pub async fn get_explore_discovery(Query(query): Query<ExploreDiscoveryQuery>) -> Response {
    let publication_limit = match normalize_limit(
        query.publication_limit,
        DEFAULT_PUBLICATION_LIMIT,
        MAX_PUBLICATION_LIMIT,
    ) {
        Ok(value) => value,

        Err(()) => {
            return bad_request("publicationLimit");
        }
    };

    let creator_limit = match normalize_limit(
        query.creator_limit,
        DEFAULT_CREATOR_LIMIT,
        MAX_CREATOR_LIMIT,
    ) {
        Ok(value) => value,

        Err(()) => {
            return bad_request("creatorLimit");
        }
    };

    let site_limit = match normalize_limit(query.site_limit, DEFAULT_SITE_LIMIT, MAX_SITE_LIMIT) {
        Ok(value) => value,

        Err(()) => {
            return bad_request("siteLimit");
        }
    };

    let base_url = index_base_url();

    let upstream_url = [
        base_url.trim_end_matches('/'),
        "/v1/index/explore",
        "?publicationLimit=",
        &publication_limit.to_string(),
        "&creatorLimit=",
        &creator_limit.to_string(),
        "&siteLimit=",
        &site_limit.to_string(),
    ]
    .concat();

    let upstream = match HTTP_CLIENT.get(upstream_url).send().await {
        Ok(response) => response,

        Err(_) => {
            return upstream_problem();
        }
    };

    let status = match StatusCode::from_u16(upstream.status().as_u16()) {
        Ok(status) => status,

        Err(_) => {
            return upstream_problem();
        }
    };

    let content_type = upstream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    let bytes = match upstream.bytes().await {
        Ok(bytes) => bytes,

        Err(_) => {
            return upstream_problem();
        }
    };

    passthrough_response(status, bytes, content_type)
}

fn normalize_limit(value: Option<usize>, fallback: usize, maximum: usize) -> Result<usize, ()> {
    let value = value.unwrap_or(fallback);

    if (1..=maximum).contains(&value) {
        return Ok(value);
    }

    Err(())
}

fn index_base_url() -> String {
    env::var("OMNIGATE_INDEX_BASE_URL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            env::var("OMNIGATE_DOWNSTREAM_INDEX_BASE_URL")
                .ok()
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| DEFAULT_INDEX_BASE_URL.to_owned())
}

fn passthrough_response(
    status: StatusCode,
    bytes: Bytes,
    content_type: Option<String>,
) -> Response {
    let mut response = Response::new(Body::from(bytes));

    *response.status_mut() = status;

    if let Some(content_type) = content_type {
        if let Ok(header_value) = HeaderValue::from_str(&content_type) {
            response
                .headers_mut()
                .insert(header::CONTENT_TYPE, header_value);
        }
    }

    response
}

fn bad_request(field: &'static str) -> Response {
    let message = match field {
        "publicationLimit" => "publicationLimit is outside the reviewed Explore bound",

        "creatorLimit" => "creatorLimit is outside the reviewed Explore bound",

        _ => "siteLimit is outside the reviewed Explore bound",
    };

    (
        StatusCode::BAD_REQUEST,
        Json(UpstreamProblem {
            code: "invalid_explore_query",

            message,

            retryable: false,
        }),
    )
        .into_response()
}

fn upstream_problem() -> Response {
    (
        StatusCode::BAD_GATEWAY,
        Json(UpstreamProblem {
            code: "index_upstream",

            message: "svc-index Explore discovery is unavailable",

            retryable: true,
        }),
    )
        .into_response()
}
