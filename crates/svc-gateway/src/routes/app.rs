//! /app/* proxy surface — forwards to omnigate /v1/app/*.
//! RO:WHAT  Accept `/app/...` from external clients and proxy to omnigate app plane.
//! RO:WHY   Make svc-gateway the single public ingress while omnigate stays internal.
//! RO:INVARS
//!   - Preserve method, headers (minus `Host` and hop-by-hop), query, and body.
//!   - Apply the same ingress middleware stack as other routes.
//!   - Pass through omnigate's Problem JSON on HTTP success; only map transport failures.

use axum::{
    body::{to_bytes, Body},
    extract::{Path, State},
    http::{self, Request, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use reqwest::header as req_header;

use crate::{config::Config, errors::Problem, state::AppState};

/// Build router for `/app/*` proxy (mounted under `/app` in `routes::mod`).
pub fn router() -> Router<AppState> {
    Router::new().route("/*tail", any(app_proxy))
}

/// Proxy handler for `/app/*` → omnigate `/v1/app/*`.
pub async fn app_proxy(
    State(state): State<AppState>,
    Path(tail): Path<String>,
    req: Request<Body>,
) -> Response {
    let cfg: &Config = &state.cfg;

    // Build target URL: <omnigate_base>/v1/app/<tail>?<query>
    let base = cfg.upstreams.omnigate_base_url.trim_end_matches('/');
    let mut url = format!("{base}/v1/app/{tail}");

    if let Some(q) = req.uri().query() {
        url.push('?');
        url.push_str(q);
    }

    let method = req.method().clone();
    let headers = req.headers().clone();

    // Buffer the incoming body; limit is aligned with configured max_body_bytes.
    let limit = cfg.limits.max_body_bytes;
    let Ok(body_bytes) = to_bytes(req.into_body(), limit).await else {
        return transport_failure(StatusCode::BAD_GATEWAY);
    };

    // Build reqwest request.
    let client = &state.omnigate_client;
    let mut rb = client.request(method, &url);

    // Copy headers, excluding `Host` (reqwest sets that) and hop-by-hop headers.
    let mut out_headers = req_header::HeaderMap::new();
    for (name, value) in &headers {
        if name == http::header::HOST || name == http::header::ACCEPT_ENCODING {
            continue;
        }
        out_headers.insert(name.clone(), value.clone());
    }
    rb = rb.headers(out_headers);

    // Send upstream; transport failures are mapped to 502 via Problem envelope.
    let Ok(upstream_res) = rb.body(body_bytes).send().await else {
        return transport_failure(StatusCode::BAD_GATEWAY);
    };

    let status = upstream_res.status();
    let resp_headers = upstream_res.headers().clone();

    // If omnigate responded, we just pass its body and status straight through.
    let Ok(body_bytes) = upstream_res.bytes().await else {
        return transport_failure(StatusCode::BAD_GATEWAY);
    };

    // Build downstream response. We deliberately do not wrap omnigate's Problem JSON;
    // it flows through unchanged.
    let mut builder = Response::builder().status(status);
    if let Some(headers_map) = builder.headers_mut() {
        for (name, value) in &resp_headers {
            // Skip hop-by-hop headers.
            if name == req_header::TRANSFER_ENCODING || name == req_header::CONNECTION {
                continue;
            }
            headers_map.insert(name, value.clone());
        }
    }

    match builder.body(Body::from(body_bytes)) {
        Ok(r) => r,
        Err(_) => transport_failure(StatusCode::BAD_GATEWAY),
    }
}

/// Map a pure transport failure to a 502 Problem response.
///
/// This is only used when we cannot talk to omnigate at all (connect timeout,
/// DNS error, refused connection, etc.).
fn transport_failure(status: StatusCode) -> Response {
    Problem {
        code: "upstream_unavailable",
        message: "Upstream omnigate unavailable",
        retryable: true,
        retry_after_ms: None,
        reason: None,
    }
    .into_response_with(status)
}
