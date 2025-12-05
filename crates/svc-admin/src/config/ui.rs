// crates/svc-admin/src/config/ui.rs
//
// WHAT: UI and dev-only UI configuration for svc-admin.

use serde::{Deserialize, Serialize};

/// UI defaults and dev options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiCfg {
    /// Default theme ("light", "dark", etc.).
    pub default_theme: String,

    /// Default language ("en-US", etc.).
    pub default_language: String,

    /// If true, disables mutating admin actions in the UI.
    pub read_only: bool,

    /// Dev-only config.
    pub dev: UiDevCfg,
}

/// Dev-only UI configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiDevCfg {
    /// Whether the App Playground client is available in the UI.
    pub enable_app_playground: bool,
}

impl Default for UiDevCfg {
    fn default() -> Self {
        Self {
            enable_app_playground: false,
        }
    }
}

impl Default for UiCfg {
    fn default() -> Self {
        Self {
            default_theme: "light".to_string(),
            default_language: "en-US".to_string(),
            read_only: true,
            dev: UiDevCfg::default(),
        }
    }
}
