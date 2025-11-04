//! RO:WHAT   Process state container passed to handlers and layers.
//! RO:WHY    Centralizes config, metrics handles, and readiness gate.
//! RO:INVARS Send + Sync; cheap to clone via Arcs.

use std::sync::Arc;

use crate::config::Config;
use crate::observability::metrics::{self, MetricsHandles};
use crate::readiness::ReadyState;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub metrics: MetricsHandles,
    pub readiness: Arc<ReadyState>,
}

impl AppState {
    /// Build a new state from provided parts.
    #[must_use]
    pub fn new(cfg: Config, metrics: MetricsHandles) -> Self {
        Self {
            cfg,
            metrics,
            readiness: Arc::new(ReadyState::new()),
        }
    }

    /// Convenience ctor for early bring-up until real loaders are wired.
    ///
    /// # Panics
    /// Panics if metric registration fails (should not happen under normal conditions).
    #[must_use]
    pub fn new_default() -> Self {
        let cfg = Config::default();
        let metrics = metrics::register().expect("register metrics");
        Self::new(cfg, metrics)
    }
}
