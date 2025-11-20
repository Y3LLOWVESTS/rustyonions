//! RO:WHAT — Simple per-request timeout middleware.
//! RO:WHY  — Prevent hung /admin calls from blocking probes forever.
//!
//! RO:INVARIANTS —
//!   - Uses a conservative fixed timeout for now.
//!   - Returns `504 Gateway Timeout` on expiry.

use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use tokio::time::timeout;
use tracing::warn;

// For now this is a static admin timeout. We can later wire this to
// `Config` (e.g., `admin_timeout`) if we want it to be configurable.
const ADMIN_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn layer(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    match timeout(ADMIN_TIMEOUT, next.run(req)).await {
        Ok(resp) => Ok(resp),
        Err(_) => {
            warn!(
                "macronode admin: request timed out after {:?}",
                ADMIN_TIMEOUT
            );
            Err(StatusCode::GATEWAY_TIMEOUT)
        }
    }
}
