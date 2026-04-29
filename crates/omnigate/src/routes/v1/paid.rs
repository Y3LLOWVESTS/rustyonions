//! RO:WHAT — v1 paid-access routes for WEB3 product UX.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/GOV. Omnigate exposes paid estimate/write BFF routes.
//! RO:INTERACTS — `svc-storage` `/paid/o/estimate` and `/paid/o`, `svc-gateway` paid routes.
//! RO:INVARIANTS — estimate is read-only; write is proxy-only; no wallet, ledger, accounting, or storage mutation here.
//! RO:METRICS — route is covered by omnigate HTTP middleware when mounted through `App::build`.
//! RO:CONFIG — `OMNIGATE_STORAGE_BASE_URL` or `OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL`.
//! RO:SECURITY — forwards selected request headers only; skips hop-by-hop headers and host.
//! RO:TEST — `tests/paid_storage_estimate_proxy.rs`, `tests/paid_storage_write_proxy.rs`.

use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderName, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::time::Duration;

const DEFAULT_STORAGE_BASE_URL: &str = "http://127.0.0.1:15303";

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate paid route reqwest client should build")
});

#[derive(Debug, Serialize)]
struct UpstreamProblem<'a> {
    code: &'a str,
    message: &'a str,
    retryable: bool,
    reason: &'a str,
}

/// Router for `/v1/paid/*`.
///
/// Mounted from `routes::v1::router()` as:
/// `nest("/paid", paid::router())`.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/o/estimate", get(estimate))
        .route("/o", post(write).put(write))
}

/// Proxy `GET /v1/paid/o/estimate?bytes=N` to `svc-storage /paid/o/estimate`.
///
/// This endpoint is intentionally side-effect free. It does not create wallet
/// holds, mutate ledger state, write object bytes, export accounting events, or
/// resolve asset manifests.
pub async fn estimate(uri: Uri, headers: HeaderMap) -> Response {
    let upstream_path = with_query("/paid/o/estimate", uri.query());
    proxy_to_storage(
        Method::GET,
        &upstream_path,
        headers,
        Bytes::new(),
        "storage estimate upstream unavailable",
    )
    .await
}

/// Proxy `POST/PUT /v1/paid/o` to `svc-storage /paid/o`.
///
/// This is intentionally proxy-only. The actual paid-write behavior stays in
/// `svc-storage`, including body hashing, wallet receipt verification,
/// capture/release, usage event creation, and accounting export.
pub async fn write(method: Method, headers: HeaderMap, body: Bytes) -> Response {
    proxy_to_storage(
        method,
        "/paid/o",
        headers,
        body,
        "storage paid route upstream unavailable",
    )
    .await
}

async fn proxy_to_storage(
    method: Method,
    upstream_path: &str,
    headers: HeaderMap,
    body: Bytes,
    unavailable_message: &'static str,
) -> Response {
    let storage_base = storage_base_url();
    let upstream_url = format!("{}{}", storage_base.trim_end_matches('/'), upstream_path);

    let reqwest_method = match reqwest::Method::from_bytes(method.as_str().as_bytes()) {
        Ok(method) => method,
        Err(_) => return upstream_problem(unavailable_message, "bad_method"),
    };

    let mut req_builder = HTTP_CLIENT.request(reqwest_method, &upstream_url);

    for (name, value) in &headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let upstream_res = match req_builder.body(body).send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => return upstream_problem(unavailable_message, "storage_connect"),
    };

    let status = upstream_res.status();
    let upstream_headers = upstream_res.headers().clone();

    let body_bytes = match upstream_res.bytes().await {
        Ok(body_bytes) => body_bytes,
        Err(_) => return upstream_problem(unavailable_message, "storage_read"),
    };

    let mut response = Response::new(Body::from(body_bytes));
    *response.status_mut() = status;

    for (name, value) in &upstream_headers {
        if should_copy_response_header(name) {
            response.headers_mut().insert(name.clone(), value.clone());
        }
    }

    response
}

fn with_query(path: &str, query: Option<&str>) -> String {
    match query {
        Some(query) if !query.is_empty() => format!("{path}?{query}"),
        _ => path.to_string(),
    }
}

fn storage_base_url() -> String {
    std::env::var("OMNIGATE_STORAGE_BASE_URL")
        .or_else(|_| std::env::var("OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_STORAGE_BASE_URL.to_string())
}

fn should_forward_header(name: &HeaderName) -> bool {
    if is_hop_by_hop_or_host(name) || name == header::CONTENT_LENGTH {
        return false;
    }

    name == header::AUTHORIZATION
        || name == header::ACCEPT
        || name == header::CONTENT_TYPE
        || name.as_str().starts_with("x-ron-")
        || name.as_str() == "x-correlation-id"
        || name.as_str() == "x-request-id"
        || name.as_str() == "idempotency-key"
}

fn should_copy_response_header(name: &HeaderName) -> bool {
    name != header::TRANSFER_ENCODING
        && name != header::CONTENT_LENGTH
        && name != header::CONNECTION
}

fn is_hop_by_hop_or_host(name: &HeaderName) -> bool {
    name == header::HOST
        || name == header::CONNECTION
        || name == header::PROXY_AUTHORIZATION
        || name == header::TE
        || name == header::TRAILER
        || name == header::TRANSFER_ENCODING
        || name == header::UPGRADE
}

fn upstream_problem(message: &'static str, reason: &'static str) -> Response {
    (
        StatusCode::BAD_GATEWAY,
        Json(UpstreamProblem {
            code: "upstream_unavailable",
            message,
            retryable: true,
            reason,
        }),
    )
        .into_response()
}
