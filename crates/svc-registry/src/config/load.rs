//! Config loading from env + optional file (TOML). Precedence aligns to docs.
//! Order of precedence (highest → lowest):
//!   1) Explicit function arg `explicit_path`
//!   2) Env var `SVCR_CONFIG_FILE`
//!   3) Workspace default: `crates/svc-registry/config/default.toml` (if exists)
//!   4) Built-in defaults (see `model.rs`)
use super::{model::Config, validate::validate_config};
use std::{env, fs, path::Path};

/// Load config following the documented precedence and apply `SVCR_*` env overrides.
pub fn load_config(explicit_path: Option<&str>) -> anyhow::Result<Config> {
    // 1) explicit path
    let mut cfg = if let Some(path) = explicit_path {
        load_if_exists(path)?.unwrap_or_default()
    } else {
        // 2) env var
        if let Ok(path) = env::var("SVCR_CONFIG_FILE") {
            load_if_exists(&path)?.unwrap_or_default()
        } else {
            // 3) workspace default file (best-effort)
            let default_path = "crates/svc-registry/config/default.toml";
            load_if_exists(default_path)?.unwrap_or_default()
        }
    };

    // 4) apply env overrides (scoped, explicit & bounded)
    apply_env_overrides(&mut cfg);

    validate_config(&cfg)?;
    Ok(cfg)
}

fn load_if_exists(path: &str) -> anyhow::Result<Option<Config>> {
    if Path::new(path).exists() {
        let s = fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&s)?;
        Ok(Some(cfg))
    } else {
        Ok(None)
    }
}

fn apply_env_overrides(cfg: &mut Config) {
    // Flat keys
    if let Ok(v) = std::env::var("SVCR_BIND_ADDR") {
        cfg.bind_addr = v;
    }
    if let Ok(v) = std::env::var("SVCR_METRICS_ADDR") {
        cfg.metrics_addr = v;
    }
    if let Ok(v) = std::env::var("SVCR_MAX_CONNS") {
        if let Ok(n) = v.parse::<u32>() {
            cfg.max_conns = n;
        }
    }
    if let Ok(v) = std::env::var("SVCR_READ_TIMEOUT") {
        cfg.read_timeout = v;
    }
    if let Ok(v) = std::env::var("SVCR_WRITE_TIMEOUT") {
        cfg.write_timeout = v;
    }
    if let Ok(v) = std::env::var("SVCR_IDLE_TIMEOUT") {
        cfg.idle_timeout = v;
    }

    // Storage
    if let Ok(v) = std::env::var("SVCR_STORAGE__KIND") {
        cfg.storage.kind = v;
    }
    if let Ok(v) = std::env::var("SVCR_STORAGE__DATA_DIR") {
        cfg.storage.data_dir = v;
    }
    if let Ok(v) = std::env::var("SVCR_STORAGE__FSYNC") {
        match v.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => cfg.storage.fsync = true,
            "0" | "false" | "no" | "off" => cfg.storage.fsync = false,
            _ => {}
        }
    }

    // Limits
    if let Ok(v) = std::env::var("SVCR_LIMITS__MAX_REQUEST_BYTES") {
        if let Ok(n) = v.parse::<usize>() {
            cfg.limits.max_request_bytes = n;
        }
    }

    // Timeouts (structured)
    if let Ok(v) = std::env::var("SVCR_TIMEOUTS__REQUEST_MS") {
        if let Ok(n) = v.parse::<u64>() {
            cfg.timeouts.request_ms = n;
        }
    }

    // SSE
    if let Ok(v) = std::env::var("SVCR_SSE__HEARTBEAT_MS") {
        if let Ok(n) = v.parse::<u64>() {
            cfg.sse.heartbeat_ms = n;
        }
    }
    if let Ok(v) = std::env::var("SVCR_SSE__MAX_CLIENTS") {
        if let Ok(n) = v.parse::<usize>() {
            cfg.sse.max_clients = n;
        }
    }
    if let Ok(v) = std::env::var("SVCR_SSE__DROP_POLICY") {
        cfg.sse.drop_policy = v;
    }

    // CORS
    if let Ok(v) = std::env::var("SVCR_CORS__ALLOWED_ORIGINS") {
        // Comma-separated list.
        let list = v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        if !list.is_empty() {
            cfg.cors.allowed_origins = list;
        }
    }
}
