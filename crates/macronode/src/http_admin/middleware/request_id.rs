//! RO:WHAT — X-Request-Id middleware.
//! RO:WHY  — Give every request/response a stable request ID for tracing.
//!
//! RO:INVARIANTS —
//!   - If the client sends `x-request-id`, we preserve it.
//!   - Otherwise we generate a simple process-unique ID.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    body::Body,
    http::{header::HeaderName, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::trace;

const X_REQUEST_ID: &str = "x-request-id";

pub async fn layer(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let header_name = HeaderName::from_static(X_REQUEST_ID);

    // If there is no request-id, generate one and attach it to the request.
    if !req.headers().contains_key(&header_name) {
        let id = gen_request_id();
        if let Ok(v) = HeaderValue::from_str(&id) {
            req.headers_mut().insert(&header_name, v);
        }
    }

    // Grab an OWNED copy of the request-id for logging and response echo.
    let id_for_log: String = req
        .headers()
        .get(&header_name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "<missing>".to_string());

    trace!(request_id = %id_for_log, "macronode admin: handling request");

    // Move the request into the next layer/handler.
    let mut res = next.run(req).await;

    // Echo the request-id back on the response if not already set.
    if !res.headers().contains_key(&header_name) {
        if let Ok(v) = HeaderValue::from_str(&id_for_log) {
            res.headers_mut().insert(&header_name, v);
        }
    }

    Ok(res)
}

fn gen_request_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Not cryptographically strong — just unique-ish for tracing.
    format!("macronode-{}", now.as_nanos())
}
