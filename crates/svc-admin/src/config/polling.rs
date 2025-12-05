// crates/svc-admin/src/config/polling.rs
//
// WHAT: Global polling / sampling configuration for metrics.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Global polling / sampling config for metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingCfg {
    /// Interval between node scrape passes.
    #[serde(skip)]
    pub metrics_interval: Duration,

    /// Rolling window used when summarizing facet metrics.
    #[serde(skip)]
    pub metrics_window: Duration,
}

impl Default for PollingCfg {
    fn default() -> Self {
        Self {
            metrics_interval: Duration::from_secs(5),
            metrics_window: Duration::from_secs(300),
        }
    }
}
