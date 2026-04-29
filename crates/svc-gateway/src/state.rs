//! RO:WHAT — Process state container passed to handlers and layers.
//! RO:WHY — Centralizes config, metrics handles, readiness gate, and shared downstream clients.
//! RO:INTERACTS — `config::Config`, `observability::metrics`, `readiness::ReadyState`, and proxy routes.
//! RO:INVARIANTS — `Send` + `Sync`; cheap to clone via `Arc`/`reqwest`; no locks across `.await`.
//! RO:METRICS — owns gateway metric handles.
//! RO:CONFIG — stores loaded gateway config.
//! RO:SECURITY — clients carry no default auth; handlers forward allowed request headers explicitly.
//! RO:TEST — `app_proxy.rs`, `paid_storage_estimate_proxy.rs`.

use std::sync::Arc;

use crate::config::Config;
use crate::observability::metrics::{self, MetricsHandles};
use crate::readiness::ReadyState;
use reqwest::Client;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub metrics: MetricsHandles,
    pub readiness: Arc<ReadyState>,
    /// Shared HTTP client for talking to omnigate app plane.
    pub omnigate_client: Client,
    /// Shared HTTP client for talking to svc-storage.
    pub storage_client: Client,
}

impl AppState {
    /// Build a new state from provided parts.
    #[must_use]
    pub fn new(cfg: Config, metrics: MetricsHandles) -> Self {
        Self {
            cfg,
            metrics,
            readiness: Arc::new(ReadyState::new()),
            omnigate_client: Client::new(),
            storage_client: Client::new(),
        }
    }

    /// Convenience ctor for early bring-up until real loaders are wired.
    ///
    /// # Panics
    ///
    /// Panics if metric registration fails, which should not happen under normal
    /// process startup.
    #[must_use]
    pub fn new_default() -> Self {
        let cfg = Config::default();
        let metrics = metrics::register().expect("register metrics");
        Self::new(cfg, metrics)
    }
}
