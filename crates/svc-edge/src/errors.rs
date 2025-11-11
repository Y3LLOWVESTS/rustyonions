//! Error taxonomy → deterministic HTTP mapping (reserved for later endpoints).

use thiserror::Error;

/// Edge service error kinds used to categorize failures.
///
/// Mapping to HTTP status codes happens in the route layer.
#[derive(Debug, Error)]
pub enum EdgeError {
    /// Service is not yet ready to serve the requested operation.
    #[error("not ready: {0}")]
    NotReady(&'static str),

    /// The request was rejected by rate limiting or admission controls.
    #[error("rate limited")]
    RateLimited,

    /// The request was malformed or violated input constraints.
    #[error("bad request: {0}")]
    BadRequest(&'static str),

    /// An unexpected internal error occurred.
    #[error("internal error")]
    Internal,
}

impl EdgeError {
    /// Short machine-readable reason tag (for logs/metrics labels).
    pub fn reason(&self) -> &'static str {
        match self {
            EdgeError::NotReady(_) => "not_ready",
            EdgeError::RateLimited => "rate_limit",
            EdgeError::BadRequest(_) => "bad_request",
            EdgeError::Internal => "internal",
        }
    }
}
