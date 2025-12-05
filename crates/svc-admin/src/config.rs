use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerCfg,
    pub auth: AuthCfg,
    pub ui: UiCfg,
    pub nodes: NodesCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCfg {
    pub bind_addr: String,
    pub metrics_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCfg {
    pub mode: String, // "none" | "ingress" | "passport"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiCfg {
    pub default_theme: String,
    pub default_language: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodesCfg {
    // TODO: Fill with node registry schema
}

impl Config {
    pub fn load() -> Result<Self> {
        // TODO: merge env vars, CLI args, and config file per docs/CONFIG.MD
        Err(Error::Config("Config::load() not implemented".into()))
    }
}
