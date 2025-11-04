//! Correlation ID middleware (route-scoped).
//! RO:WHAT  Ensure each request has an `x-request-id`; echo it on the response.
//! RO:WHY   Stable correlation for tracing/logs with zero deps.
//! RO:NOTE  Axum 0.7 `Next` has no generic parameter; `Request` must be `Request<Body>`.

use axum::{
    body::Body,
    http::{HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use std::sync::atomic::{AtomicU64, Ordering};

static REQ_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Axum 0.7 middleware entry point.
///
/// # Behavior
/// * Respects incoming `x-request-id` if present and valid UTF-8.
/// * Otherwise synthesizes `r-<hex>` from a monotonic counter (cheap, unique-enough for dev).
/// * Echoes the final id back on the response header.
///
/// # Errors
/// Never errors; always returns a `Response`.
pub async fn mw(mut req: Request<Body>, next: Next) -> Response {
    // Try to get an existing request id.
    let maybe_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned); // clippy: method reference instead of redundant closure

    let id = maybe_id.unwrap_or_else(|| {
        let n = REQ_COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("r-{n:016x}")
    });

    // Ensure header is present for downstream handlers.
    if !req.headers().contains_key("x-request-id") {
        if let Ok(v) = HeaderValue::from_str(&id) {
            req.headers_mut().insert("x-request-id", v);
        }
    }

    let mut resp = next.run(req).await;

    // Always mirror the id back on the response.
    if let Ok(v) = HeaderValue::from_str(&id) {
        resp.headers_mut().insert("x-request-id", v);
    }

    resp
}
