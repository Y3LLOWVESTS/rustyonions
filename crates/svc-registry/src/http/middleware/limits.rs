//! Request size limits using Axum's built-in limiter (preserves Response<Body>).

use axum::extract::DefaultBodyLimit;

/// Config for request body size limits.
#[derive(Clone, Copy, Debug)]
pub struct LimitCfg {
    /// Maximum allowed request body in bytes.
    pub max_body_bytes: usize,
}

impl Default for LimitCfg {
    fn default() -> Self {
        // Foundation: small cap; bump later per route if needed.
        Self {
            max_body_bytes: 64 * 1024,
        }
    }
}

/// Build the body-limit layer. Safe to apply at Router::layer with axum 0.7.9.
pub fn limits_layer(cfg: &LimitCfg) -> DefaultBodyLimit {
    DefaultBodyLimit::max(cfg.max_body_bytes)
}
