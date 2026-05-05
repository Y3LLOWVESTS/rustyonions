//! `GET/HEAD /o/{addr}` raw object proxy.
//!
//! RO:WHAT — Public gateway read path for immutable CAS bytes by canonical `b3:<64hex>` CID.
//! RO:WHY — `WEB3_2/CrabLink` image previews need browser-safe raw bytes after asset-page resolution.
//! RO:INTERACTS — `svc-storage` `/o/:cid`, `state::AppState`, `CrabLink` asset preview cards.
//! RO:INVARIANTS — proxy-only; no storage writes; no wallet/ledger/accounting mutation; storage verifies CID.
//! RO:METRICS — route inherits gateway HTTP metrics/correlation layers when mounted.
//! RO:CONFIG — `SVC_GATEWAY_STORAGE_BASE_URL` via `cfg.upstreams.storage_base_url`.
//! RO:SECURITY — forwards selected headers only; filters hop-by-hop headers.
//! RO:TEST — manual `curl /o/b3:<hash>`; future object proxy test should pin headers/status/body.

use crate::{errors, state::AppState};
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, HeaderMap, HeaderName, Method},
    response::Response,
};

/// Proxy `GET /o/:addr` to `svc-storage /o/:addr`.
///
/// This is the raw byte path used by `CrabLink` previews after an asset page
/// exposes a `links.raw` route such as `/o/b3:<hash>`.
pub async fn get_object(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    headers: HeaderMap,
) -> Response {
    let upstream_path = format!("/o/{addr}");

    proxy_to_storage(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

/// Proxy `HEAD /o/:addr` to `svc-storage /o/:addr`.
///
/// This gives browser/client code a cheap way to check that a CID exists and to
/// inspect storage-provided headers without fetching bytes.
pub async fn head_object(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    headers: HeaderMap,
) -> Response {
    let upstream_path = format!("/o/{addr}");

    proxy_to_storage(&state, Method::HEAD, &upstream_path, headers, Bytes::new()).await
}

async fn proxy_to_storage(
    state: &AppState,
    method: Method,
    upstream_path: &str,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let base = state.cfg.upstreams.storage_base_url.trim_end_matches('/');
    let upstream_url = format!("{base}{upstream_path}");

    let Ok(reqwest_method) = reqwest::Method::from_bytes(method.as_str().as_bytes()) else {
        return errors::upstream_unavailable("bad_method");
    };

    let mut req_builder = state.storage_client.request(reqwest_method, &upstream_url);

    for (name, value) in &headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let Ok(upstream_res) = req_builder.body(body).send().await else {
        return errors::upstream_unavailable("storage_connect");
    };

    let status = upstream_res.status();
    let upstream_headers = upstream_res.headers().clone();

    let Ok(body_bytes) = upstream_res.bytes().await else {
        return errors::upstream_unavailable("storage_read");
    };

    let mut response = Response::new(Body::from(body_bytes));
    *response.status_mut() = status;

    let response_headers = response.headers_mut();

    for (name, value) in &upstream_headers {
        if should_copy_response_header(name) {
            response_headers.insert(name.clone(), value.clone());
        }
    }

    if !response_headers.contains_key(header::CONTENT_TYPE) && method == Method::GET {
        response_headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/octet-stream"),
        );
    }

    response
}

#[must_use]
fn should_forward_header(name: &HeaderName) -> bool {
    !is_hop_by_hop_or_host(name)
}

#[must_use]
fn should_copy_response_header(name: &HeaderName) -> bool {
    !matches!(
        name,
        &header::CONNECTION
            | &header::PROXY_AUTHENTICATE
            | &header::PROXY_AUTHORIZATION
            | &header::TE
            | &header::TRAILER
            | &header::TRANSFER_ENCODING
            | &header::UPGRADE
    )
}

#[must_use]
fn is_hop_by_hop_or_host(name: &HeaderName) -> bool {
    matches!(
        name,
        &header::HOST
            | &header::CONNECTION
            | &header::PROXY_AUTHORIZATION
            | &header::TE
            | &header::TRAILER
            | &header::TRANSFER_ENCODING
            | &header::UPGRADE
    )
}
