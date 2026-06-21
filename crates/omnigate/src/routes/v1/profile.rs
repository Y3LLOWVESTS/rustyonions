//! RO:WHAT — Omnigate façade/proxy routes for svc-passport public profile claims and reads.
//! RO:WHY — NEXT_LEVEL Phase 4 requires profile exposure without CrabLink calling svc-passport directly.
//! RO:INTERACTS — svc-passport `/v1/passport/profile/*`, svc-gateway future `/identity/passport/profile/*`, CrabLink.
//! RO:INVARIANTS — proxy only; no private keys; no wallet/ledger mutation; no public main↔alt linkage.
//! RO:METRICS — covered by Omnigate route middleware when mounted through app bootstrap.
//! RO:CONFIG — `OMNIGATE_PASSPORT_BASE_URL` or `OMNIGATE_DOWNSTREAM_PASSPORT_BASE_URL`.
//! RO:SECURITY — forwards selected request context; filters hop-by-hop headers; upstream failures map to structured 502.
//! RO:TEST — `tests/passport_profile_routes.rs`.

use axum::{
    body::Bytes,
    extract::Path,
    http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::{env, time::Duration};

const DEFAULT_PASSPORT_BASE_URL: &str = "http://127.0.0.1:5307";

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate passport profile reqwest client should build")
});

/// POST /v1/identity/passport/profile/claim
///
/// Proxies a username/profile claim request to svc-passport. Omnigate does not
/// persist username claims and does not mint wallet authority.
pub async fn claim_profile(headers: HeaderMap, body: Bytes) -> Response {
    proxy_to_passport(
        Method::POST,
        "/v1/passport/profile/claim".to_owned(),
        headers,
        Some(body),
    )
    .await
}

/// GET /v1/identity/passport/profile/:username
///
/// Proxies a read-only public profile lookup to svc-passport.
pub async fn get_profile(Path(username): Path<String>, headers: HeaderMap) -> Response {
    let Some(encoded_username) = safe_profile_path_segment(&username) else {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_username_path",
            "profile username path segment is invalid",
            false,
            "username_path_invalid",
        );
    };

    proxy_to_passport(
        Method::GET,
        format!("/v1/passport/profile/{encoded_username}"),
        headers,
        None,
    )
    .await
}

async fn proxy_to_passport(
    method: Method,
    path: String,
    headers: HeaderMap,
    body: Option<Bytes>,
) -> Response {
    let url = format!("{}{}", passport_base_url().trim_end_matches('/'), path);
    let mut request = HTTP_CLIENT
        .request(method, url)
        .timeout(Duration::from_secs(5));

    for (name, value) in forwarded_headers(&headers) {
        request = request.header(name, value);
    }

    if let Some(body) = body {
        request = request.body(body.to_vec());
    }

    let upstream = match request.send().await {
        Ok(response) => response,
        Err(err) => {
            let reason = if err.is_timeout() {
                "passport_upstream_timeout"
            } else if err.is_connect() {
                "passport_upstream_connect"
            } else {
                "passport_upstream_request"
            };

            return problem(
                StatusCode::BAD_GATEWAY,
                "passport_upstream",
                "svc-passport upstream request failed",
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
                "passport_upstream_body",
                "svc-passport upstream response body could not be read",
                true,
                "passport_upstream_body_read",
            );
        }
    };

    let mut response = Response::builder().status(status);
    if let Some(headers) = response.headers_mut() {
        headers.insert(header::CONTENT_TYPE, content_type);
    }

    response
        .body(axum::body::Body::from(body))
        .unwrap_or_else(|_| {
            problem(
                StatusCode::BAD_GATEWAY,
                "passport_response_build",
                "omnigate could not build passport proxy response",
                true,
                "response_build_failed",
            )
        })
}

fn passport_base_url() -> String {
    env::var("OMNIGATE_PASSPORT_BASE_URL")
        .or_else(|_| env::var("OMNIGATE_DOWNSTREAM_PASSPORT_BASE_URL"))
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_PASSPORT_BASE_URL.to_owned())
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
        || name == header::CONTENT_TYPE
        || name.as_str().eq_ignore_ascii_case("idempotency-key")
        || name.as_str().eq_ignore_ascii_case("x-correlation-id")
        || name.as_str().eq_ignore_ascii_case("x-request-id")
        || super::header_policy::is_allowed_ron_context_header(name)
}

fn safe_profile_path_segment(value: &str) -> Option<String> {
    let value = value.trim();

    if value.is_empty() {
        return None;
    }

    if value.contains('/')
        || value.contains('\\')
        || value.contains('?')
        || value.contains('#')
        || value.contains('%')
        || value.chars().any(char::is_control)
    {
        return None;
    }

    Some(value.to_owned())
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
