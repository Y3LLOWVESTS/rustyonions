//! /app/* proxy to omnigate app plane.
//!
//! RO:WHAT  Forward `/app/{tail}` to omnigate `/v1/app/{tail}`.
//! RO:WHY   First-hop app-plane gateway for micronode/macronode apps.
//! RO:CONF  `SVC_GATEWAY_OMNIGATE_BASE_URL` controls the upstream base URL.

use crate::state::AppState;
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
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
pub async fn proxy(
    State(state): State<AppState>,
    method: Method,
    Path(tail): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let base = &state.cfg.upstreams.omnigate_base_url;
    // Simple join; omnigate_base_url is expected to have no trailing slash.
    let url = format!("{base}/v1/app/{tail}");

    let mut req_builder = state.omnigate_client.request(method, &url);

    // Forward headers, skipping Host (reqwest sets its own).
    for (name, value) in &headers {
        if name == header::HOST {
            continue;
        }
        req_builder = req_builder.header(name, value);
    }

    // Send upstream request.
    let Ok(upstream_res) = req_builder.body(body).send().await else {
        return (StatusCode::BAD_GATEWAY, "upstream connect error").into_response();
    };

    let status = upstream_res.status();
    let upstream_headers = upstream_res.headers().clone();

    // Extract body bytes from upstream (this consumes `upstream_res`).
    let Ok(body_bytes) = upstream_res.bytes().await else {
        return (StatusCode::BAD_GATEWAY, "upstream read error").into_response();
    };

    // Build downstream response.
    let mut resp = Response::new(Body::from(body_bytes));
    *resp.status_mut() = status;

    let resp_headers = resp.headers_mut();
    for (name, value) in &upstream_headers {
        resp_headers.insert(name.clone(), value.clone());
    }

    resp
}
