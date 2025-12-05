use serde::{Deserialize, Serialize};
use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfigDto {
    pub default_theme: String,
    pub available_themes: Vec<String>,
    pub default_language: String,
    pub available_languages: Vec<String>,
    pub read_only: bool,
}

impl UiConfigDto {
    pub fn from_cfg(cfg: &Config) -> Self {
        Self {
            default_theme: cfg.ui.default_theme.clone(),
            available_themes: vec!["light".into(), "dark".into()],
            default_language: cfg.ui.default_language.clone(),
            available_languages: vec!["en-US".into(), "es-ES".into()],
            read_only: cfg.ui.read_only,
        }
    }
}
