//! RO:WHAT — Rate limiting middleware (placeholder).
//! RO:WHY  — Anchor point for future per-endpoint throttling.
//!
//! RO:INVARIANTS —
//!   - Currently a no-op pass-through.
//!   - Safe to extend later with token buckets / IP-based limits.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn layer(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    // TODO: Implement per-endpoint/IP rate limiting once we have config knobs.
    Ok(next.run(req).await)
}
