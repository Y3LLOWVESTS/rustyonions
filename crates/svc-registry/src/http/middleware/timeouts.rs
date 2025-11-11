//! Simple overall request timeout (foundation default).
use std::time::Duration;
// CHANGE: use tower_http's TimeoutLayer, not tower::timeout
use tower_http::timeout::TimeoutLayer;

#[derive(Clone)]
pub struct TimeoutCfg {
    pub overall_ms: u64,
}

impl Default for TimeoutCfg {
    fn default() -> Self {
        // Read-mostly service; 5s overall is generous for foundation.
        Self { overall_ms: 5_000 }
    }
}

pub fn timeouts_layer(cfg: &TimeoutCfg) -> TimeoutLayer {
    TimeoutLayer::new(Duration::from_millis(cfg.overall_ms))
}
