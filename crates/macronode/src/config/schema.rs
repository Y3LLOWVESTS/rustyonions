//! RO:WHAT — Minimal config schema for Macronode.
//! RO:WHY  — Bind HTTP admin, timeouts, and log level with sane defaults.
//! RO:INTERACTS —
//!   - Loaded via `config::load_config()`.
//!   - Passed into runtime state and admin HTTP stack.

use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// HTTP admin bind address (`RON_HTTP_ADDR` / `MACRO_HTTP_ADDR`).
    pub http_addr: SocketAddr,

    /// Log level (fan-out via `RUST_LOG` env in logging bootstrap).
    pub log_level: String,

    /// HTTP read timeout.
    pub read_timeout: Duration,

    /// HTTP write timeout.
    pub write_timeout: Duration,

    /// HTTP idle timeout.
    pub idle_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            http_addr: "127.0.0.1:8080"
                .parse()
                .expect("default 127.0.0.1:8080 must parse"),
            log_level: "info".to_string(),
            read_timeout: Duration::from_secs(10),
            write_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(60),
        }
    }
}
