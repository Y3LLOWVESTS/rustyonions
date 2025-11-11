//! AppState — shared state for handlers (Arc by Axum state).

use crate::{config::Config, metrics::EdgeMetrics};
use ron_kernel::HealthState;
use std::sync::Arc;

/// Shared application state used by Axum handlers.
///
/// Must be `Clone + Send + Sync + 'static` for Axum. We achieve this with
/// `Arc<...>` for reference-counted, thread-safe ownership.
#[derive(Clone)]
pub struct AppState {
    /// Immutable configuration for the running process.
    pub cfg: Arc<Config>,
    /// Metrics handle for recording service metrics.
    pub metrics: EdgeMetrics,
    /// Health state used by /readyz and other liveness gates.
    pub health: Arc<HealthState>,
}

impl AppState {
    /// Construct a new `AppState` from config, metrics, and health.
    pub fn new(cfg: Config, metrics: EdgeMetrics, health: Arc<HealthState>) -> Self {
        Self {
            cfg: Arc::new(cfg),
            metrics,
            health,
        }
    }
}
