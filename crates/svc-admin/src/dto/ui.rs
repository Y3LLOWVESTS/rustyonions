// crates/svc-admin/src/dto/ui.rs
//
// RO:WHAT — UI config DTOs exposed to the svc-admin SPA.
// RO:WHY — Provide a stable, documented JSON contract for theme, locale,
//          read-only mode, and dev-only toggles, driven by Config/UiCfg.
// RO:INTERACTS — config::UiCfg, server/router (ui_config handler),
//                crates/svc-admin/ui/src/api/adminClient.ts,
//                i18n/index.ts, theme/ThemeProvider.tsx.
// RO:INVARIANTS — JSON field names are camelCase and must stay in sync with
//                 ALL_DOCS / API.MD and the TypeScript UiConfigDto. Values
//                 are derived from Config and must reflect actual behavior
//                 (e.g., readOnly must match ui.read_only).

use serde::{Deserialize, Serialize};

use crate::config::Config;

/// UI config consumed by the SPA via `GET /api/ui-config`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiConfigDto {
    /// Default theme ("light", "dark", "system", etc.).
    pub default_theme: String,

    /// Themes the operator can select in the UI.
    pub available_themes: Vec<String>,

    /// Default language / locale ("en-US", "es-ES", ...).
    pub default_language: String,

    /// Languages the operator can select in the UI.
    pub available_languages: Vec<String>,

    /// When true, the SPA should not render any mutating controls.
    pub read_only: bool,

    /// Dev-only configuration for the UI, safe to expose to the browser.
    pub dev: UiDevDto,
}

/// Dev-only UI config surfaced to the SPA.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiDevDto {
    /// Whether the App Playground client should be available in the UI.
    pub enable_app_playground: bool,
}

impl UiConfigDto {
    pub fn from_cfg(cfg: &Config) -> Self {
        Self {
            default_theme: cfg.ui.default_theme.clone(),
            // For now this is a fixed set; later we can plumb this from config.
            available_themes: vec!["light".into(), "dark".into()],
            default_language: cfg.ui.default_language.clone(),
            // Likewise, a fixed set for the developer preview.
            available_languages: vec!["en-US".into(), "es-ES".into()],
            read_only: cfg.ui.read_only,
            dev: UiDevDto {
                enable_app_playground: cfg.ui.dev.enable_app_playground,
            },
        }
    }
}
