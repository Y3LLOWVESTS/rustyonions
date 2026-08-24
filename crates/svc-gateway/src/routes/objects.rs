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

use crate::{errors, headers::proxy, state::AppState};
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, HeaderMap, Method},
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

fn copy_storage_response_headers(
    method: &Method,
    upstream_headers: &HeaderMap,
    response_headers: &mut HeaderMap,
) {
    for (name, value) in upstream_headers {
        if proxy::should_copy_response_header(name) {
            response_headers.insert(name.clone(), value.clone());
        }
    }

    // Generic proxy policy intentionally strips Content-Length because most
    // gateway responses rebuild their body. HEAD is different: its empty
    // response body describes an existing stored entity, so the authoritative
    // entity length from svc-storage must survive the public proxy hop.
    if method == Method::HEAD {
        if let Some(content_length) = upstream_headers.get(header::CONTENT_LENGTH) {
            response_headers.insert(header::CONTENT_LENGTH, content_length.clone());
        }
    }
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
        if proxy::should_forward_passthrough_header(name) {
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

    copy_storage_response_headers(&method, &upstream_headers, response_headers);

    if !response_headers.contains_key(header::CONTENT_TYPE) && method == Method::GET {
        response_headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/octet-stream"),
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn head_preserves_storage_entity_length_without_relaxing_shared_policy() {
        assert!(
            !proxy::should_copy_response_header(&header::CONTENT_LENGTH,),
            "generic proxy policy must continue stripping Content-Length",
        );

        let mut upstream_headers = HeaderMap::new();

        upstream_headers.insert(header::CONTENT_LENGTH, HeaderValue::from_static("3"));

        upstream_headers.insert(header::ETAG, HeaderValue::from_static("\"b3:test\""));

        let mut head_headers = HeaderMap::new();

        copy_storage_response_headers(&Method::HEAD, &upstream_headers, &mut head_headers);

        assert_eq!(
            head_headers
                .get(header::CONTENT_LENGTH,)
                .and_then(|value| { value.to_str().ok() },),
            Some("3",),
            "HEAD must preserve svc-storage's authoritative entity length",
        );

        assert_eq!(
            head_headers
                .get(header::ETAG,)
                .and_then(|value| { value.to_str().ok() },),
            Some("\"b3:test\"",),
        );

        let mut get_headers = HeaderMap::new();

        copy_storage_response_headers(&Method::GET, &upstream_headers, &mut get_headers);

        assert!(
            get_headers.get(header::CONTENT_LENGTH,).is_none(),
            "GET must retain generic rebuilt-body Content-Length policy",
        );

        assert_eq!(
            get_headers
                .get(header::ETAG,)
                .and_then(|value| { value.to_str().ok() },),
            Some("\"b3:test\"",),
        );
    }
}
