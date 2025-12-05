// crates/svc-admin/src/config/log.rs
//
// WHAT: Logging configuration for svc-admin.

use serde::{Deserialize, Serialize};

/// Logging config (very simple for now).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogCfg {
    /// "compact" | "pretty"
    pub format: String,

    /// log level (e.g. "info", "debug", "trace").
    pub level: String,
}

impl Default for LogCfg {
    fn default() -> Self {
        Self {
            format: "compact".to_string(),
            level: "info".to_string(),
        }
    }
}
