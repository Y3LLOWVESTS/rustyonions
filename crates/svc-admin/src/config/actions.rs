// crates/svc-admin/src/config/actions.rs
//
// WHAT: Feature flags for admin actions (reload, shutdown, etc.).

use serde::{Deserialize, Serialize};

/// Admin actions flags.
///
/// In dev-preview we keep these off by default to avoid surprising behavior
/// (e.g. reloading or shutting down a node from the UI by accident).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionsCfg {
    /// Whether "reload config" is exposed.
    #[serde(default)]
    pub enable_reload: bool,

    /// Whether "shutdown node" is exposed.
    #[serde(default)]
    pub enable_shutdown: bool,
}
