//! RO:WHAT — Minimal config schema for Macronode.
//! RO:WHY  — Bind HTTP admin, metrics, timeouts, and log level with sane
//!           defaults.
//! RO:INTERACTS —
//!   - Loaded via `config::load_config()` / `load_config_with_file()`.
//!   - Passed into runtime state and admin HTTP stack.

use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};

fn default_http_addr() -> SocketAddr {
    "127.0.0.1:8080"
        .parse()
        .expect("default 127.0.0.1:8080 must parse")
}

fn default_metrics_addr() -> SocketAddr {
    // By default we bind metrics on the same address as the admin HTTP plane.
    default_http_addr()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_read_timeout() -> Duration {
    Duration::from_secs(10)
}

fn default_write_timeout() -> Duration {
    Duration::from_secs(10)
}

fn default_idle_timeout() -> Duration {
    Duration::from_secs(60)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// HTTP admin bind address (`RON_HTTP_ADDR` / `MACRO_HTTP_ADDR`).
    #[serde(default = "default_http_addr")]
    pub http_addr: SocketAddr,

    /// Metrics bind address (`RON_METRICS_ADDR` / `MACRO_METRICS_ADDR`).
    ///
    /// Invariants:
    ///   - Defaults to the same value as `http_addr`.
    ///   - Env/CLI overlays may override it independently.
    #[serde(default = "default_metrics_addr")]
    pub metrics_addr: SocketAddr,

    /// Log level (fan-out via `RUST_LOG` env in logging bootstrap).
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// HTTP read timeout.
    ///
    /// File-config form uses humantime strings like `"5s"`, `"500ms"`, `"1m"`.
    /// Env overlay still respects `RON_READ_TIMEOUT` / `MACRO_READ_TIMEOUT`
    /// with the same humantime semantics.
    #[serde(default = "default_read_timeout", with = "humantime_serde")]
    pub read_timeout: Duration,

    /// HTTP write timeout.
    #[serde(default = "default_write_timeout", with = "humantime_serde")]
    pub write_timeout: Duration,

    /// HTTP idle timeout.
    #[serde(default = "default_idle_timeout", with = "humantime_serde")]
    pub idle_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            http_addr: default_http_addr(),
            metrics_addr: default_metrics_addr(),
            log_level: default_log_level(),
            read_timeout: default_read_timeout(),
            write_timeout: default_write_timeout(),
            idle_timeout: default_idle_timeout(),
        }
    }
}
