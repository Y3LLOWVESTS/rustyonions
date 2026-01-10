//! RO:WHAT — Admin-plane request counting middleware (by facet, low-cardinality).
//! RO:WHY  — svc-admin needs request rollups (min/hour/day/month) alongside bandwidth.
//! RO:INVARIANTS — no lock across .await; never allocates per request beyond path borrow

#![forbid(unsafe_code)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::observability::net_accounting;

pub async fn layer(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    net_accounting::record_request(path);
    Ok(next.run(req).await)
}
