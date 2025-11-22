//! RO:WHAT   Process state container passed to handlers and layers.
//! RO:WHY    Centralizes config, metrics handles, readiness gate, and shared
//!           HTTP client (omnigate) so handlers stay lightweight.
//! RO:INVARS Send + Sync; cheap to clone via Arcs.

use std::{sync::Arc, time::Duration};

use crate::config::Config;
use crate::observability::metrics::{self, MetricsHandles};
use crate::readiness::ReadyState;

/// Application state shared across handlers and layers.
#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub metrics: MetricsHandles,
    pub readiness: Arc<ReadyState>,
    /// Shared HTTP client for talking to omnigate (app plane).
    pub omnigate_client: reqwest::Client,
}

impl AppState {
    /// Construct a new state bag from config + metrics.
    ///
    /// # Panics
    ///
    /// Panics if the omnigate HTTP client cannot be built (should not happen).
    #[must_use]
    pub fn new(cfg: Config, metrics: MetricsHandles) -> Self {
        let timeout_s = cfg
            .server
            .read_timeout_secs
            .max(cfg.server.write_timeout_secs)
            .max(1);
        let timeout = Duration::from_secs(timeout_s);

        let omnigate_client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("build omnigate client");

        Self {
            cfg,
            metrics,
            readiness: Arc::new(ReadyState::new()),
            omnigate_client,
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
