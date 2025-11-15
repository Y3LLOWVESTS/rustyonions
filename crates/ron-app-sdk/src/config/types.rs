//! Config data types for ron-app-sdk.
//!
//! RO:WHAT — Pure data structs/enums for SDK configuration (no I/O).
//! RO:WHY  — Keeps `mod.rs` focused on helpers and env parsing; this file
//!           is just the shape + defaults of configuration.
//! RO:INTERACTS — Re-exported by `config::mod`; consumed across planes,
//!                retry helpers, and examples.
//! RO:INVARIANTS — Serializable with `serde`; defaults are safe/hardened;
//!                 no panics or external side effects.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Wire-protocol transport flavor: direct TLS or Tor via SOCKS5.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    /// Direct TLS over TCP (recommended default).
    Tls,
    /// Tor via SOCKS5 (arti/tor).
    Tor,
}

/// Jitter mode for exponential backoff.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Jitter {
    /// Full jitter: 0..base (random applied later).
    Full,
    /// No jitter: always use the base delay.
    None,
}

/// Log redaction posture for the SDK.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Redaction {
    /// Redact sensitive material where possible.
    Safe,
    /// Do not redact; more verbose but less private.
    None,
}

/// Post-quantum mode for edge → node connections.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PqMode {
    Off,
    Hybrid,
}

/// Per-connection timeout knobs (excluding the global per-call deadline).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Timeouts {
    /// Connect timeout for establishing a new transport connection.
    #[serde(with = "humantime_serde", default = "default_connect")]
    pub connect: Duration,
    /// Read timeout on an established connection.
    #[serde(with = "humantime_serde", default = "default_read")]
    pub read: Duration,
    /// Write timeout on an established connection.
    #[serde(with = "humantime_serde", default = "default_write")]
    pub write: Duration,
}

fn default_connect() -> Duration {
    Duration::from_secs(3)
}

fn default_read() -> Duration {
    Duration::from_secs(30)
}

fn default_write() -> Duration {
    Duration::from_secs(30)
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect: default_connect(),
            read: default_read(),
            write: default_write(),
        }
    }
}

/// Retry/backoff configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RetryCfg {
    /// Base delay for the first retry attempt.
    #[serde(with = "humantime_serde", default = "default_retry_base")]
    pub base: Duration,
    /// Multiplicative factor per attempt.
    #[serde(default = "default_retry_factor")]
    pub factor: f32,
    /// Cap on backoff delay.
    #[serde(with = "humantime_serde", default = "default_retry_cap")]
    pub cap: Duration,
    /// Maximum number of attempts (including the first).
    #[serde(default = "default_retry_max_attempts")]
    pub max_attempts: u32,
    /// Jitter mode.
    #[serde(default = "default_jitter")]
    pub jitter: Jitter,
}

fn default_retry_base() -> Duration {
    Duration::from_millis(100)
}

fn default_retry_factor() -> f32 {
    2.0
}

fn default_retry_cap() -> Duration {
    Duration::from_secs(5)
}

fn default_retry_max_attempts() -> u32 {
    5
}

fn default_jitter() -> Jitter {
    Jitter::Full
}

impl Default for RetryCfg {
    fn default() -> Self {
        Self {
            base: default_retry_base(),
            factor: default_retry_factor(),
            cap: default_retry_cap(),
            max_attempts: default_retry_max_attempts(),
            jitter: default_jitter(),
        }
    }
}

/// Idempotency tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IdemCfg {
    /// Whether to attach idempotency keys by default on mutations.
    #[serde(default = "default_idem_enabled")]
    pub enabled: bool,
    /// Optional key prefix to avoid PII in idempotency keys.
    pub key_prefix: Option<String>,
}

fn default_idem_enabled() -> bool {
    true
}

impl Default for IdemCfg {
    fn default() -> Self {
        Self {
            enabled: default_idem_enabled(),
            key_prefix: None,
        }
    }
}

/// In-memory client-side cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheCfg {
    /// Whether the cache is enabled at all.
    #[serde(default)]
    pub enabled: bool,
    /// Maximum number of entries in the LRU.
    #[serde(default = "default_cache_entries")]
    pub max_entries: usize,
    /// TTL for each entry.
    #[serde(with = "humantime_serde", default = "default_cache_ttl")]
    pub ttl: Duration,
}

fn default_cache_entries() -> usize {
    1024
}

fn default_cache_ttl() -> Duration {
    Duration::from_secs(30)
}

impl Default for CacheCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            max_entries: default_cache_entries(),
            ttl: default_cache_ttl(),
        }
    }
}

/// Tracing and metrics toggles.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TracingCfg {
    /// Emit span events for SDK calls.
    #[serde(default)]
    pub spans: bool,
    /// Emit Prometheus-style metrics from the SDK.
    #[serde(default)]
    pub metrics: bool,
    /// Redaction posture.
    #[serde(default = "default_redaction")]
    pub redaction: Redaction,
}

fn default_redaction() -> Redaction {
    Redaction::Safe
}

impl Default for TracingCfg {
    fn default() -> Self {
        Self {
            spans: true,
            metrics: true,
            redaction: default_redaction(),
        }
    }
}

/// Tor-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TorCfg {
    /// SOCKS5 address for the local Tor daemon.
    #[serde(default = "default_tor_socks")]
    pub socks5_addr: String,
}

fn default_tor_socks() -> String {
    "127.0.0.1:9050".to_string()
}

impl Default for TorCfg {
    fn default() -> Self {
        Self {
            socks5_addr: default_tor_socks(),
        }
    }
}

/// Application-facing configuration for ron-app-sdk.
///
/// This struct is intentionally "boring": just data + serde. All env
/// parsing and validation lives in `config::mod`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SdkConfig {
    /// Transport flavor (`tls` or `tor`).
    pub transport: Transport,
    /// Base gateway address (URL or `.onion`).
    pub gateway_addr: String,
    /// Overall per-call deadline (including retries).
    #[serde(with = "humantime_serde", default = "default_overall_timeout")]
    pub overall_timeout: Duration,
    /// Per-connection timeouts.
    pub timeouts: Timeouts,
    /// Retry/backoff tuning.
    pub retry: RetryCfg,
    /// Idempotency tuning.
    pub idempotency: IdemCfg,
    /// Client-side cache tuning.
    pub cache: CacheCfg,
    /// Tracing/metrics toggles.
    pub tracing: TracingCfg,
    /// Post-quantum mode.
    pub pq_mode: PqMode,
    /// Tor-specific configuration.
    pub tor: TorCfg,
}

fn default_overall_timeout() -> Duration {
    // README.md suggests 5000 ms as the baseline.
    Duration::from_millis(5000)
}

impl Default for SdkConfig {
    fn default() -> Self {
        Self {
            transport: Transport::Tls,
            gateway_addr: "http://127.0.0.1:8080".to_string(),
            overall_timeout: default_overall_timeout(),
            timeouts: Timeouts::default(),
            retry: RetryCfg::default(),
            idempotency: IdemCfg::default(),
            cache: CacheCfg::default(),
            tracing: TracingCfg::default(),
            pq_mode: PqMode::Off,
            tor: TorCfg::default(),
        }
    }
}
