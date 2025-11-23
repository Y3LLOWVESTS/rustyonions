//! /app/* proxy to omnigate app plane.
//!
//! RO:WHAT  Forward `/app/{tail}` to omnigate `/v1/app/{tail}`.
//! RO:WHY   First-hop app-plane gateway for micronode/macronode apps.
//! RO:CONF  `SVC_GATEWAY_OMNIGATE_BASE_URL` controls the upstream base URL.

use crate::{errors, state::AppState};
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, HeaderMap, Method, Uri},
    response::Response,
    Router,
};

/// Router for `/app/*` subtree.
///
/// Mounted from `routes::build_router` as:
/// `router.nest("/app", app::router())`
pub fn router() -> Router<AppState> {
    use axum::routing::any;
    Router::new().route("/*tail", any(proxy))
}

/// Proxy handler: `/app/{tail}` → `{omnigate_base}/v1/app/{tail}`.
///
/// RO:INVARS
///   * Preserve method, path tail, query string, and body bytes.
///   * Preserve critical headers (Authorization, X-RON-*, correlation/request IDs).
///   * Do not forward hop-by-hop headers (Host, Connection, TE, etc.).
///   * Pass upstream 4xx/5xx bodies unchanged.
///   * Map transport failures to 502 Problem JSON via `errors::upstream_unavailable`.
pub async fn proxy(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    Path(tail): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let base = &state.cfg.upstreams.omnigate_base_url;

    // Preserve original query string, if any.
    let query_suffix = match uri.query() {
        Some(q) => {
            let mut s = String::with_capacity(1 + q.len());
            s.push('?');
            s.push_str(q);
            s
        }
        None => String::new(),
    };

    // Simple join; omnigate_base_url is expected to have no trailing slash.
    let upstream_url = format!("{base}/v1/app/{tail}{query_suffix}");

    let mut req_builder = state.omnigate_client.request(method, &upstream_url);

    // Forward headers, skipping Host and hop-by-hop headers that must not be proxied.
    //
    // We explicitly *do not* touch:
    //   * Authorization
    //   * X-RON-Token
    //   * X-RON-Passport
    //   * X-Correlation-ID
    //   * X-Request-Id (corr layer ensures one exists if missing)
    for (name, value) in &headers {
        if name == header::HOST
            || name == header::CONNECTION
            || name == header::PROXY_AUTHORIZATION
            || name == header::TE
            || name == header::TRAILER
            || name == header::TRANSFER_ENCODING
            || name == header::UPGRADE
        {
            continue;
        }
        req_builder = req_builder.header(name, value);
    }

    // Send upstream request.
    let Ok(upstream_res) = req_builder.body(body).send().await else {
        // Transport/connect failure → 502 Problem JSON.
        return errors::upstream_unavailable("connect");
    };

    let status = upstream_res.status();
    let upstream_headers = upstream_res.headers().clone();

    // Extract body bytes from upstream (this consumes `upstream_res`).
    let Ok(body_bytes) = upstream_res.bytes().await else {
        // Transport/read failure → 502 Problem JSON.
        return errors::upstream_unavailable("read");
    };

    // Build downstream response.
    let mut resp = Response::new(Body::from(body_bytes));
    *resp.status_mut() = status;

    // Copy upstream headers to downstream, skipping hop-by-hop and length headers that
    // are either re-computed or not meaningful to forward.
    let resp_headers = resp.headers_mut();
    for (name, value) in &upstream_headers {
        if name == header::TRANSFER_ENCODING || name == header::CONTENT_LENGTH {
            continue;
        }
        resp_headers.insert(name.clone(), value.clone());
    }

    resp
}
