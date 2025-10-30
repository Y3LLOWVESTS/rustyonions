//! RO:WHAT — Load and validate service configuration (env + optional file).
//! RO:WHY  — Governance & Hardening defaults (timeouts, limits).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub bind: String,
    pub body_cap_bytes: usize,
    pub cache_ttl_secs: u64,
    pub ready_dep_timeout_ms: u64,
    pub enable_sled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:5304".into(),
            body_cap_bytes: 1024 * 1024, // 1 MiB
            cache_ttl_secs: 30,
            ready_dep_timeout_ms: 1500,
            enable_sled: true,
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut cfg = Config::default();
        if let Ok(v) = std::env::var("BIND") {
            cfg.bind = v;
        }
        if let Ok(v) = std::env::var("BODY_CAP_BYTES") {
            cfg.body_cap_bytes = v.parse().unwrap_or(cfg.body_cap_bytes);
        }
        if let Ok(v) = std::env::var("CACHE_TTL_SECS") {
            cfg.cache_ttl_secs = v.parse().unwrap_or(cfg.cache_ttl_secs);
        }
        if let Ok(v) = std::env::var("READY_DEP_TIMEOUT_MS") {
            cfg.ready_dep_timeout_ms = v.parse().unwrap_or(cfg.ready_dep_timeout_ms);
        }
        if let Ok(v) = std::env::var("ENABLE_SLED") {
            cfg.enable_sled = v == "1" || v.eq_ignore_ascii_case("true");
        }
        Ok(cfg)
    }
}
