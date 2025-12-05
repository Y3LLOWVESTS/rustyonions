// crates/svc-admin/src/config/actions.rs
//
// WHAT: Feature flags for admin actions (reload, shutdown, etc.).

use serde::{Deserialize, Serialize};

/// Admin actions flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionsCfg {
    /// Whether "reload config" is exposed.
    pub enable_reload: bool,

    /// Whether "shutdown node" is exposed.
    pub enable_shutdown: bool,
}

impl Default for ActionsCfg {
    fn default() -> Self {
        Self {
            enable_reload: false,
            enable_shutdown: false,
        }
    }
}
