//! RO:WHAT — Builder + environment merge for `RykerConfig` (file optional).
//! RO:WHY  — Enforce builder > env > file > defaults; prod guards.
//! RO:INTERACTS — model (schema), reload (hooks), runtime creates snapshot.
//! RO:INVARIANTS — `RYKER_CONFIG_PATH` ignored in prod unless explicitly allowed.

use super::model::RykerConfig;
use crate::errors::{ConfigError, Result};
use once_cell::sync::Lazy;
use std::env;

static IS_PROD: Lazy<bool> = Lazy::new(|| {
    let app = env::var("APP_ENV").ok();
    let rust = env::var("RUST_ENV").ok();
    matches!(app.or(rust), Some(s) if s == "production")
});

#[derive(Default)]
pub struct RykerConfigBuilder {
    inner: RykerConfig,
}

impl RykerConfigBuilder {
    pub fn new() -> Self {
        Self {
            inner: RykerConfig::default(),
        }
    }
    pub fn build(mut self) -> Result<RykerConfig> {
        merge_env(&mut self.inner)?;
        self.inner.validate()?;
        Ok(self.inner)
    }
}

pub fn from_env_validated() -> Result<RykerConfig> {
    let mut cfg = RykerConfig::default();

    if let Ok(path) = env::var("RYKER_CONFIG_PATH") {
        if *IS_PROD && env::var("RYKER_ALLOW_CONFIG_PATH").as_deref() != Ok("1") {
            return Err(
                ConfigError::ProdGuard("RYKER_CONFIG_PATH rejected in production".into()).into(),
            );
        }
        tracing_log!("dev-cli path provided (host should load file): {}", path);
    }

    merge_env(&mut cfg)?;
    cfg.validate()?;
    Ok(cfg)
}

fn merge_env(cfg: &mut RykerConfig) -> Result<()> {
    let get = |k: &str| env::var(k).ok();

    if let Some(v) = get("RYKER_DEFAULT_MAILBOX_CAPACITY") {
        cfg.defaults.mailbox_capacity = v.parse().map_err(|_| {
            ConfigError::Invalid("RYKER_DEFAULT_MAILBOX_CAPACITY must be int".into())
        })?;
    }
    if let Some(v) = get("RYKER_DEFAULT_MAX_MSG_BYTES") {
        cfg.defaults.max_msg_bytes = parse_size(&v)?;
    }
    if let Some(v) = get("RYKER_DEFAULT_DEADLINE") {
        cfg.defaults.deadline = humantime::parse_duration(&v)
            .map_err(|_| ConfigError::Invalid("bad deadline".into()))?;
    }
    if let Some(v) = get("RYKER_BACKOFF_BASE_MS") {
        cfg.supervisor.backoff_base_ms = v
            .parse()
            .map_err(|_| ConfigError::Invalid("bad backoff_base_ms".into()))?;
    }
    if let Some(v) = get("RYKER_BACKOFF_CAP_MS") {
        cfg.supervisor.backoff_cap_ms = v
            .parse()
            .map_err(|_| ConfigError::Invalid("bad backoff_cap_ms".into()))?;
    }
    if let Some(v) = get("RYKER_BATCH_MESSAGES") {
        cfg.fairness.batch_messages = v
            .parse()
            .map_err(|_| ConfigError::Invalid("bad batch_messages".into()))?;
    }
    if let Some(v) = get("RYKER_YIELD_EVERY_N") {
        cfg.fairness.yield_every_n_msgs = v
            .parse()
            .map_err(|_| ConfigError::Invalid("bad yield_every_n".into()))?;
    }
    if let Some(v) = get("RYKER_ENABLE_METRICS") {
        cfg.observe.queue_depth_sampling = matches!(v.as_str(), "1" | "true" | "TRUE");
    }
    if let Some(v) = get("RYKER_AMNESIA") {
        cfg.amnesia = matches!(v.as_str(), "1" | "true" | "TRUE");
    }
    Ok(())
}

fn parse_size(s: &str) -> Result<usize> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix("KiB") {
        let v: usize = n
            .trim()
            .parse()
            .map_err(|_| ConfigError::Invalid("size".into()))?;
        return Ok(v * 1024);
    }
    if let Some(n) = s.strip_suffix("MiB") {
        let v: usize = n
            .trim()
            .parse()
            .map_err(|_| ConfigError::Invalid("size".into()))?;
        return Ok(v * 1024 * 1024);
    }
    let v: usize = s.parse().map_err(|_| ConfigError::Invalid("size".into()))?;
    Ok(v)
}

// Tiny local macro to avoid requiring tracing at call site when feature disabled
#[inline]
fn tracing_log_(lvl: &str, msg: &str) {
    #[cfg(feature = "tracing")]
    match lvl {
        "info" => tracing::info!("{}", msg),
        _ => tracing::debug!("{}", msg),
    }
}

macro_rules! tracing_log {
    ($($tt:tt)*) => {
        #[allow(unused)]
        {
            let s = format!($($tt)*);
            super::loader::tracing_log_("debug", &s);
        }
    };
}
pub(crate) use tracing_log;
