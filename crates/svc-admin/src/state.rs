// crates/svc-admin/src/state.rs
//
// RO:WHAT — Shared application state for svc-admin (config, node registry, metrics, auth).
// RO:WHY  — Single Arc<AppState> used by Axum handlers + middleware for consistent wiring.
// RO:INVARIANTS —
//   - Local auth is initialized iff auth.mode == "local".
//   - No blocking work in handlers; all IO async (RBAC file IO only during init/bootstrap).
//   - State is cheap to clone via Arc and interior types are thread-safe.

#![forbid(unsafe_code)]

use crate::auth::local::{LocalAuth, LocalAuthCfg};
use crate::config::Config;
use crate::metrics::facet::FacetMetrics;
use crate::nodes::registry::NodeRegistry;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Shared application state for svc-admin.
///
/// This gets wrapped in an Arc and shared with all HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    /// Static/slow-changing config for svc-admin itself.
    pub config: Config,

    /// Registry of known nodes + their admin plane connection info.
    pub nodes: NodeRegistry,

    /// Short-horizon facet metrics store populated by background samplers.
    ///
    /// This is intentionally in-memory only and bounded by
    /// `config.polling.metrics_window`.
    pub facet_metrics: FacetMetrics,

    /// Local auth backend (cookie sessions + RBAC) when `auth.mode == "local"`.
    ///
    /// None in other auth modes.
    pub local_auth: Option<Arc<LocalAuth>>,
}

impl AppState {
    /// Construct application state from config.
    ///
    /// This is the single entry point used by `server::run` when wiring
    /// the Axum router.
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let registry = NodeRegistry::new(&config.nodes);

        // Use the configured rolling window for facet metrics.
        let facet_window = config.polling.metrics_window;
        let facet_metrics = FacetMetrics::new(facet_window);

        // Initialize local auth iff requested.
        let local_auth = if config.auth.is_local() {
            let cfg = LocalAuthCfg {
                rbac_path: PathBuf::from(config.auth.rbac_path_or_default()),
                cookie_name: config.auth.cookie_name_or_default().to_string(),
                cookie_secure: config.auth.cookie_secure_or_default(),

                // Not currently exposed via Config helpers in the snippets we have;
                // keep sane defaults for local-dev.
                cookie_domain: None,
                cookie_path: "/".to_string(),

                session_ttl: Duration::from_secs(config.auth.session_ttl_sec_or_default()),
                // LocalAuthCfg calls this `session_idle` (not session_idle_ttl).
                session_idle: Duration::from_secs(config.auth.session_idle_ttl_sec_or_default()),

                bootstrap_admin_username: config
                    .auth
                    .bootstrap_admin_username_or_default()
                    .to_string(),
                bootstrap_admin_password_env: config
                    .auth
                    .bootstrap_admin_password_env_or_default()
                    .to_string(),
            };

            Some(Arc::new(LocalAuth::new(cfg)?))
        } else {
            None
        };

        Ok(Self {
            config,
            nodes: registry,
            facet_metrics,
            local_auth,
        })
    }
}
