//! High-level config helpers for ron-app-sdk.
//!
//! RO:WHAT — Glue for `SdkConfig`: validation + environment loader + helpers.
//! RO:WHY  — Central place for config rules so the rest of the SDK can stay
//!           boring (just read fields).
//! RO:INTERACTS — Used by `RonAppSdk::new`, examples, and tests; reads env
//!                via `SdkConfig::from_env()`.
//! RO:INVARIANTS — No panics; invalid configs surface as errors
//!                 (`anyhow::Result`); env parsing is explicit, no silent
//!                 fallbacks.

mod types;

pub use types::{
    CacheCfg, IdemCfg, Jitter, PqMode, Redaction, RetryCfg, SdkConfig, Timeouts, TorCfg,
    TracingCfg, Transport,
};

use std::{collections::HashMap, time::Duration};

use anyhow::{anyhow, bail, Result as AnyResult};

impl SdkConfig {
    /// Validate semantic invariants for this configuration.
    ///
    /// This is called automatically from `from_env` and should also be
    /// invoked by host apps that construct configs programmatically.
    pub fn validate(&self) -> AnyResult<()> {
        if self.gateway_addr.trim().is_empty() {
            bail!("gateway_addr must not be empty");
        }

        if self.retry.max_attempts == 0 {
            bail!("retry.max_attempts must be at least 1");
        }

        if self.retry.factor < 1.0 {
            bail!("retry.factor must be >= 1.0");
        }

        if self.retry.base.is_zero() {
            bail!("retry.base must be > 0");
        }

        if self.retry.cap < self.retry.base {
            bail!("retry.cap must be >= retry.base");
        }

        if self.overall_timeout < Duration::from_secs(1) {
            bail!("overall_timeout must be >= 1s");
        }

        if self.overall_timeout < self.timeouts.read
            || self.overall_timeout < self.timeouts.write
        {
            bail!("overall_timeout must be >= read/write timeouts");
        }

        if self.cache.enabled {
            if self.cache.max_entries == 0 {
                bail!("cache.max_entries must be >= 1 when cache.enabled=true");
            }
            if self.cache.ttl < Duration::from_secs(1) {
                bail!("cache.ttl must be >= 1s when cache.enabled=true");
            }
        }

        // PQ mode + Tor reachability checks can be extended later; for now
        // we just ensure the socks address is non-empty when Tor is chosen.
        if matches!(self.transport, Transport::Tor) && self.tor.socks5_addr.trim().is_empty() {
            bail!("tor.socks5_addr must not be empty when transport=tor");
        }

        Ok(())
    }

    /// Build a config from environment variables, with safe defaults.
    ///
    /// Mapping roughly follows `docs/CONFIG.md`:
    ///
    /// - `RON_SDK_TRANSPORT`              → `transport`
    /// - `RON_SDK_GATEWAY_ADDR`           → `gateway_addr`
    /// - `RON_SDK_OVERALL_TIMEOUT_MS`     → `overall_timeout`
    /// - `RON_SDK_CONNECT_TIMEOUT_MS`     → `timeouts.connect`
    /// - `RON_SDK_READ_TIMEOUT_MS`        → `timeouts.read`
    /// - `RON_SDK_WRITE_TIMEOUT_MS`       → `timeouts.write`
    /// - `RON_SDK_RETRY_BASE_MS`          → `retry.base`
    /// - `RON_SDK_RETRY_FACTOR`           → `retry.factor`
    /// - `RON_SDK_RETRY_CAP_MS`           → `retry.cap`
    /// - `RON_SDK_RETRY_MAX_ATTEMPTS`     → `retry.max_attempts`
    /// - `RON_SDK_RETRY_JITTER`           → `retry.jitter`
    /// - `RON_SDK_IDEM_ENABLED`           → `idempotency.enabled`
    /// - `RON_SDK_IDEM_PREFIX`            → `idempotency.key_prefix`
    /// - `RON_SDK_CACHE_ENABLED`          → `cache.enabled`
    /// - `RON_SDK_CACHE_MAX_ENTRIES`      → `cache.max_entries`
    /// - `RON_SDK_CACHE_TTL_MS`           → `cache.ttl`
    /// - `RON_SDK_TRACING_SPANS`          → `tracing.spans`
    /// - `RON_SDK_TRACING_METRICS`        → `tracing.metrics`
    /// - `RON_SDK_TRACING_REDACTION`      → `tracing.redaction`
    /// - `RON_SDK_PQ_MODE`                → `pq_mode`
    /// - `RON_SDK_TOR_SOCKS5_ADDR`        → `tor.socks5_addr`
    pub fn from_env() -> AnyResult<SdkConfig> {
        // FIX: pass `std::env::vars()` directly; it already implements
        // `IntoIterator<Item = (String, String)>`.
        Self::from_env_with(std::env::vars())
    }

    /// Testable helper behind `from_env` that works off an arbitrary map.
    pub(crate) fn from_env_with<I>(vars: I) -> AnyResult<SdkConfig>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let map: HashMap<_, _> = vars.into_iter().collect();
        let get = |key: &str| map.get(key).map(String::as_str);

        let mut cfg = SdkConfig::default();

        // Transport + gateway
        if let Some(v) = get("RON_SDK_TRANSPORT") {
            cfg.transport = match v.to_ascii_lowercase().as_str() {
                "tls" => Transport::Tls,
                "tor" => Transport::Tor,
                other => {
                    return Err(anyhow!(
                        "invalid RON_SDK_TRANSPORT: {other} (expected `tls` or `tor`)"
                    ));
                }
            };
        }

        if let Some(v) = get("RON_SDK_GATEWAY_ADDR") {
            cfg.gateway_addr = v.to_string();
        }

        // Timeouts (ms)
        if let Some(v) = get("RON_SDK_OVERALL_TIMEOUT_MS") {
            cfg.overall_timeout = parse_ms("RON_SDK_OVERALL_TIMEOUT_MS", v)?;
        }

        if let Some(v) = get("RON_SDK_CONNECT_TIMEOUT_MS") {
            cfg.timeouts.connect = parse_ms("RON_SDK_CONNECT_TIMEOUT_MS", v)?;
        }

        if let Some(v) = get("RON_SDK_READ_TIMEOUT_MS") {
            cfg.timeouts.read = parse_ms("RON_SDK_READ_TIMEOUT_MS", v)?;
        }

        if let Some(v) = get("RON_SDK_WRITE_TIMEOUT_MS") {
            cfg.timeouts.write = parse_ms("RON_SDK_WRITE_TIMEOUT_MS", v)?;
        }

        // Retry
        if let Some(v) = get("RON_SDK_RETRY_BASE_MS") {
            cfg.retry.base = parse_ms("RON_SDK_RETRY_BASE_MS", v)?;
        }

        if let Some(v) = get("RON_SDK_RETRY_FACTOR") {
            cfg.retry.factor = parse_f32("RON_SDK_RETRY_FACTOR", v)?;
        }

        if let Some(v) = get("RON_SDK_RETRY_CAP_MS") {
            cfg.retry.cap = parse_ms("RON_SDK_RETRY_CAP_MS", v)?;
        }

        if let Some(v) = get("RON_SDK_RETRY_MAX_ATTEMPTS") {
            cfg.retry.max_attempts = parse_u32("RON_SDK_RETRY_MAX_ATTEMPTS", v)?;
        }

        if let Some(v) = get("RON_SDK_RETRY_JITTER") {
            cfg.retry.jitter = match v.to_ascii_lowercase().as_str() {
                "full" => Jitter::Full,
                "none" => Jitter::None,
                other => {
                    return Err(anyhow!(
                        "invalid RON_SDK_RETRY_JITTER: {other} (expected `full` or `none`)"
                    ));
                }
            };
        }

        // Idempotency
        if let Some(v) = get("RON_SDK_IDEM_ENABLED") {
            cfg.idempotency.enabled = parse_bool("RON_SDK_IDEM_ENABLED", v)?;
        }

        if let Some(v) = get("RON_SDK_IDEM_PREFIX") {
            cfg.idempotency.key_prefix = if v.is_empty() {
                None
            } else {
                Some(v.to_string())
            };
        }

        // Cache
        if let Some(v) = get("RON_SDK_CACHE_ENABLED") {
            cfg.cache.enabled = parse_bool("RON_SDK_CACHE_ENABLED", v)?;
        }

        if let Some(v) = get("RON_SDK_CACHE_MAX_ENTRIES") {
            cfg.cache.max_entries = parse_usize("RON_SDK_CACHE_MAX_ENTRIES", v)?;
        }

        if let Some(v) = get("RON_SDK_CACHE_TTL_MS") {
            cfg.cache.ttl = parse_ms("RON_SDK_CACHE_TTL_MS", v)?;
        }

        // Tracing
        if let Some(v) = get("RON_SDK_TRACING_SPANS") {
            cfg.tracing.spans = parse_bool("RON_SDK_TRACING_SPANS", v)?;
        }

        if let Some(v) = get("RON_SDK_TRACING_METRICS") {
            cfg.tracing.metrics = parse_bool("RON_SDK_TRACING_METRICS", v)?;
        }

        if let Some(v) = get("RON_SDK_TRACING_REDACTION") {
            cfg.tracing.redaction = match v.to_ascii_lowercase().as_str() {
                "safe" => Redaction::Safe,
                "none" => Redaction::None,
                other => {
                    return Err(anyhow!(
                        "invalid RON_SDK_TRACING_REDACTION: {other} (expected `safe` or `none`)"
                    ));
                }
            };
        }

        // PQ mode
        if let Some(v) = get("RON_SDK_PQ_MODE") {
            cfg.pq_mode = match v.to_ascii_lowercase().as_str() {
                "off" => PqMode::Off,
                "hybrid" => PqMode::Hybrid,
                other => {
                    return Err(anyhow!(
                        "invalid RON_SDK_PQ_MODE: {other} (expected `off` or `hybrid`)"
                    ));
                }
            };
        }

        // Tor
        if let Some(v) = get("RON_SDK_TOR_SOCKS5_ADDR") {
            cfg.tor.socks5_addr = v.to_string();
        }

        // Final semantic pass.
        cfg.validate()?;
        Ok(cfg)
    }

    /// Convenience helper for tests/examples to override a handful of fields.
    pub fn with_overrides<F>(mut self, f: F) -> SdkConfig
    where
        F: FnOnce(&mut SdkConfig),
    {
        f(&mut self);
        self
    }
}

/// Parse a boolean env value.
///
/// Accepted truthy: `1`, `true`, `yes`, `on` (case-insensitive)  
/// Accepted falsy:  `0`, `false`, `no`, `off` (case-insensitive)
fn parse_bool(key: &str, raw: &str) -> AnyResult<bool> {
    let v = raw.to_ascii_lowercase();
    match v.as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(anyhow!(
            "invalid {key}: {raw} (expected boolean like true/false)"
        )),
    }
}

/// Parse an integer millisecond value.
fn parse_ms(key: &str, raw: &str) -> AnyResult<Duration> {
    let ms: u64 = raw
        .parse()
        .map_err(|e| anyhow!("invalid {key}: {raw} (expected integer ms, err={e})"))?;
    Ok(Duration::from_millis(ms))
}

fn parse_f32(key: &str, raw: &str) -> AnyResult<f32> {
    raw.parse()
        .map_err(|e| anyhow!("invalid {key}: {raw} (expected f32, err={e})"))
}

fn parse_u32(key: &str, raw: &str) -> AnyResult<u32> {
    raw.parse()
        .map_err(|e| anyhow!("invalid {key}: {raw} (expected u32, err={e})"))
}

fn parse_usize(key: &str, raw: &str) -> AnyResult<usize> {
    raw.parse()
        .map_err(|e| anyhow!("invalid {key}: {raw} (expected usize, err={e})"))
}
