// crates/svc-admin/src/config/ui.rs

use serde::{Deserialize, Serialize};

fn default_read_only() -> bool {
    true
}

fn default_ui_theme() -> String {
    "system".to_string()
}

fn default_ui_language() -> String {
    "en-US".to_string()
}

/// Dev-only flags for the admin UI.
///
/// These are explicitly *not* intended for production usage; they gate
/// experimental features like the app playground.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiDevCfg {
    /// Whether to expose the app playground in the UI.
    #[serde(default)]
    pub enable_app_playground: bool,
}

/// Top-level UI configuration used to build `UiConfigDto`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiCfg {
    /// Whether the UI should run in read-only mode (no mutating actions).
    #[serde(default = "default_read_only")]
    pub read_only: bool,

    /// Default theme for the UI (e.g. "system", "light", "dark").
    #[serde(default = "default_ui_theme")]
    pub default_theme: String,

    /// Default language/locale tag (e.g. "en-US").
    #[serde(default = "default_ui_language")]
    pub default_language: String,

    /// Dev-only flags.
    #[serde(default)]
    pub dev: UiDevCfg,
}

impl Default for UiCfg {
    fn default() -> Self {
        Self {
            read_only: default_read_only(),
            default_theme: default_ui_theme(),
            default_language: default_ui_language(),
            dev: UiDevCfg::default(),
        }
    }
}
