//! Read-only and write-through paid-storage gateway routes.
//!
//! RO:WHAT — Proxy paid-storage estimate/write requests from gateway to omnigate.
//! RO:WHY — WEB3 product UX needs edge-facing price preflight and paid write submission.
//! RO:INTERACTS — omnigate `/v1/paid/o/estimate`, omnigate `/v1/paid/o`, and `svc-storage` paid routes.
//! RO:INVARIANTS — estimate is read-only; write is proxy-only; no wallet, ledger, accounting, or storage mutation here.
//! RO:METRICS — route inherits gateway HTTP metrics/correlation layers.
//! RO:CONFIG — `SVC_GATEWAY_OMNIGATE_BASE_URL`.
//! RO:SECURITY — forwards selected request headers; skips hop-by-hop headers.
//! RO:TEST — `tests/paid_storage_estimate_proxy.rs`, `tests/paid_storage_write_proxy.rs`.

use crate::{errors, state::AppState};
use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{header, HeaderMap, HeaderName, Method, Uri},
    response::Response,
    routing::{get, post},
    Router,
};

/// Router for `/paid/*` subtree.
///
/// Mounted from `routes::build_router` as:
/// `router.nest("/paid", paid_storage::router())`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/o/estimate", get(estimate))
        .route("/o", post(write).put(write))
}

/// Proxy `GET /paid/o/estimate?bytes=N` to omnigate.
///
/// Gateway keeps the public edge route short while omnigate remains the BFF.
pub async fn estimate(State(state): State<AppState>, uri: Uri, headers: HeaderMap) -> Response {
    let upstream_path = with_query("/v1/paid/o/estimate", uri.query());
    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

/// Proxy `POST/PUT /paid/o` to omnigate.
///
/// Gateway is only an edge/BFF ingress here. The actual write semantics remain:
///
/// ```text
/// client → svc-gateway /paid/o
///        → omnigate /v1/paid/o
///        → svc-storage /paid/o
/// ```
pub async fn write(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, method, "/v1/paid/o", headers, body).await
}

async fn proxy_to_omnigate(
    state: &AppState,
    method: Method,
    upstream_path: &str,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let base = state.cfg.upstreams.omnigate_base_url.trim_end_matches('/');
    let upstream_url = format!("{base}{upstream_path}");

    let Ok(reqwest_method) = reqwest::Method::from_bytes(method.as_str().as_bytes()) else {
        return errors::upstream_unavailable("bad_method");
    };

    let mut req_builder = state.omnigate_client.request(reqwest_method, &upstream_url);

    for (name, value) in &headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let Ok(upstream_res) = req_builder.body(body).send().await else {
        return errors::upstream_unavailable("omnigate_connect");
    };

    let status = upstream_res.status();
    let upstream_headers = upstream_res.headers().clone();

    let Ok(body_bytes) = upstream_res.bytes().await else {
        return errors::upstream_unavailable("omnigate_read");
    };

    let mut resp = Response::new(Body::from(body_bytes));
    *resp.status_mut() = status;

    let resp_headers = resp.headers_mut();
    for (name, value) in &upstream_headers {
        if should_copy_response_header(name) {
            resp_headers.insert(name.clone(), value.clone());
        }
    }

    resp
}

fn with_query(path: &str, query: Option<&str>) -> String {
    match query {
        Some(query) if !query.is_empty() => format!("{path}?{query}"),
        _ => path.to_string(),
    }
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
