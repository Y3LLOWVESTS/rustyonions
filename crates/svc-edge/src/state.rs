//! Process-wide shared state for svc-edge.
//
// RO:WHAT
// - Holds immutable runtime config, metrics handle, and health gate.
// - Must be `Clone + Send + Sync + 'static` to satisfy axum 0.7 router state bounds.
//
// RO:INVARIANTS
// - No interior mutability hidden here; mutable bits live behind explicit APIs
//   (metrics handles, health gates) or are wrapped in Arcs.

use std::sync::Arc;

use crate::{Config, EdgeMetrics, HealthState};

/// Application state shared across axum handlers.
///
/// This type **must** be `Clone + Send + Sync + 'static` so that
/// `Router<AppState>::into_make_service()` is available in axum 0.7.
#[derive(Clone)]
pub struct AppState {
    /// Effective runtime configuration (immutable at runtime).
    pub cfg: Arc<Config>,
    /// Metrics handle (counters/histograms are internally synchronized).
    pub metrics: EdgeMetrics,
    /// Process health/readiness gates.
    pub health: Arc<HealthState>,
}

impl AppState {
    /// Construct a new [`AppState`].
    ///
    /// The `cfg` is wrapped into an `Arc` to make cloning cheap and to satisfy
    /// axum's `Clone + Send + Sync + 'static` bounds for router state.
    pub fn new(cfg: Config, metrics: EdgeMetrics, health: Arc<HealthState>) -> Self {
        Self {
            cfg: Arc::new(cfg),
            metrics,
            health,
        }
    }
}
