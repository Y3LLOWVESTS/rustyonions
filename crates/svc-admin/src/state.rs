use crate::config::Config;
use crate::nodes::registry::NodeRegistry;

/// Shared application state for svc-admin.
///
/// This gets wrapped in an Arc and shared with all HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub nodes: NodeRegistry,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let registry = NodeRegistry::new(&config.nodes);
        Self {
            config,
            nodes: registry,
        }
    }
}
