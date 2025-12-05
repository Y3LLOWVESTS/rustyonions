use crate::config::Config;
use crate::metrics::facet::FacetMetrics;
use crate::nodes::registry::NodeRegistry;

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
}

impl AppState {
    /// Construct application state from config.
    ///
    /// This is the single entry point used by `server::run` when wiring
    /// the Axum router.
    pub fn new(config: Config) -> Self {
        let registry = NodeRegistry::new(&config.nodes);

        // Use the configured rolling window for facet metrics.
        let facet_window = config.polling.metrics_window;
        let facet_metrics = FacetMetrics::new(facet_window);

        Self {
            config,
            nodes: registry,
            facet_metrics,
        }
    }
}
