//! RO:WHAT — Omnigate read-only façade for canonical creator publication projections.
//! RO:WHY — FINAL_BETA Phase 6 keeps CrabLink and gateway clients away from direct svc-index calls.
//! RO:INTERACTS — svc-index creator publication list/detail routes and later svc-gateway proxy routes.
//! RO:INVARIANTS — proxy only; bounded paths; strict query fields; upstream status and JSON bodies pass through.
//! RO:SECURITY — no wallet, ledger, receipt, entitlement, follow, settlement, key, PIN, recovery, or capability authority.
//! RO:TEST — final_beta_phase6_creator_publication_route.rs.

// FINAL_BETA_PHASE6B3_OMNIGATE_PUBLICATION_READ_ROUTE_V1

use axum::{
    body::Bytes,
    extract::{Path, Query},
    http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{env, time::Duration};

const DEFAULT_INDEX_BASE_URL: &str = "http://127.0.0.1:5304";

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate publication client should build")
});

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreatorPublicationQuery {
    #[serde(default)]
    pub cursor: Option<String>,

    #[serde(default)]
    pub limit: Option<usize>,
}

/// GET /v1/creators/:username/publications
pub async fn list_creator_publications(
    Path(username): Path<String>,
    Query(query): Query<CreatorPublicationQuery>,
    headers: HeaderMap,
) -> Response {
    let Some(username) = safe_username_segment(&username) else {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_creator_username",
            "creator username path segment is invalid",
            false,
            "creator_username_invalid",
        );
    };

    let path = format!("/v1/index/creators/{username}/publications",);

    proxy_to_index(Method::GET, path, Some(query), headers).await
}

/// GET /v1/creators/:username/publications/:publication_id
pub async fn get_creator_publication(
    Path((username, publication_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let Some(username) = safe_username_segment(&username) else {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_creator_username",
            "creator username path segment is invalid",
            false,
            "creator_username_invalid",
        );
    };

    let Some(publication_id) = safe_publication_id_segment(&publication_id) else {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_publication_id",
            "publication identifier path segment is invalid",
            false,
            "publication_id_invalid",
        );
    };

    let path = format!("/v1/index/creators/{username}/publications/{publication_id}",);

    proxy_to_index(Method::GET, path, None, headers).await
}

async fn proxy_to_index(
    method: Method,
    path: String,
    query: Option<CreatorPublicationQuery>,
    headers: HeaderMap,
) -> Response {
    let raw_url = format!("{}{}", index_base_url().trim_end_matches('/'), path,);

    let mut url = match reqwest::Url::parse(&raw_url) {
        Ok(url) => url,
        Err(_) => {
            return problem(
                StatusCode::BAD_GATEWAY,
                "index_upstream_url",
                "svc-index upstream URL is invalid",
                true,
                "index_upstream_url_invalid",
            );
        }
    };

    if let Some(query) = query {
        let mut pairs = url.query_pairs_mut();

        if let Some(cursor) = query.cursor.as_deref() {
            pairs.append_pair("cursor", cursor);
        }

        if let Some(limit) = query.limit {
            pairs.append_pair("limit", &limit.to_string());
        }
    }

    let mut request = HTTP_CLIENT
        .request(method, url)
        .timeout(Duration::from_secs(5));

    for (name, value) in forwarded_headers(&headers) {
        request = request.header(name, value);
    }

    let upstream = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            let reason = if error.is_timeout() {
                "index_upstream_timeout"
            } else if error.is_connect() {
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

    let status = upstream.status();

    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .cloned()
        .unwrap_or_else(|| HeaderValue::from_static("application/json"));

    let body = match upstream.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return problem(
                StatusCode::BAD_GATEWAY,
                "index_upstream_body",
                "svc-index upstream response body could not be read",
                true,
                "index_upstream_body_read",
            );
        }
    };

    proxy_response(status, content_type, body)
}

fn proxy_response(status: StatusCode, content_type: HeaderValue, body: Bytes) -> Response {
    let mut response = Response::builder().status(status);

    if let Some(headers) = response.headers_mut() {
        headers.insert(header::CONTENT_TYPE, content_type);
    }

    response
        .body(axum::body::Body::from(body))
        .unwrap_or_else(|_| {
            problem(
                StatusCode::BAD_GATEWAY,
                "index_response_build",
                "omnigate could not build index proxy response",
                true,
                "response_build_failed",
            )
        })
}

fn index_base_url() -> String {
    env::var("OMNIGATE_INDEX_BASE_URL")
        .or_else(|_| env::var("OMNIGATE_DOWNSTREAM_INDEX_BASE_URL"))
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_INDEX_BASE_URL.to_owned())
}

fn forwarded_headers(headers: &HeaderMap) -> Vec<(HeaderName, HeaderValue)> {
    headers
        .iter()
        .filter(|(name, _)| should_forward_header(name))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

fn should_forward_header(name: &HeaderName) -> bool {
    if name == header::HOST
        || name == header::CONNECTION
        || name == header::CONTENT_LENGTH
        || name == header::TRANSFER_ENCODING
        || name == header::UPGRADE
        || name.as_str().eq_ignore_ascii_case("proxy-authorization")
        || name.as_str().eq_ignore_ascii_case("te")
        || name.as_str().eq_ignore_ascii_case("trailer")
    {
        return false;
    }

    name == header::AUTHORIZATION
        || name == header::ACCEPT
        || name.as_str().eq_ignore_ascii_case("x-correlation-id")
        || name.as_str().eq_ignore_ascii_case("x-request-id")
        || super::header_policy::is_allowed_ron_context_header(name)
}

fn safe_username_segment(value: &str) -> Option<String> {
    let value = value.trim();

    let bytes = value.as_bytes();

    if !(3..=32).contains(&bytes.len()) {
        return None;
    }

    if !(bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit()) {
        return None;
    }

    if bytes.iter().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_' || *byte == b'-'
    }) {
        Some(value.to_owned())
    } else {
        None
    }
}

fn safe_publication_id_segment(value: &str) -> Option<String> {
    let value = value.trim();

    let bytes = value.as_bytes();

    if !(1..=128).contains(&bytes.len()) {
        return None;
    }

    if !bytes[0].is_ascii_alphanumeric() {
        return None;
    }

    if bytes
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b':' | b'-',))
    {
        Some(value.to_owned())
    } else {
        None
    }
}

#[derive(Debug, Serialize)]
struct Problem<'a> {
    code: &'a str,
    message: &'a str,
    retryable: bool,
    reason: &'a str,
}

fn problem(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
) -> Response {
    (
        status,
        Json(Problem {
            code,
            message,
            retryable,
            reason,
        }),
    )
        .into_response()
}
