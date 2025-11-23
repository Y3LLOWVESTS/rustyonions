//! Config model + defaults for svc-gateway.
//!
//! RO:WHAT  Minimal config for bind addr, caps, and omnigate upstream.
//! RO:WHY   Keep it small and env-driven for now; TOML/FS loaders can plug in later.

use crate::consts::{
    DEFAULT_BODY_CAP_BYTES, DEFAULT_DECODE_ABS_CAP_BYTES, DEFAULT_MAX_CONNS, DEFAULT_RPS,
};
use serde::Deserialize;

pub mod env;

/// Server-level configuration (bind addr, connection caps, RPS).
#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    /// Socket address to bind the HTTP listener to.
    pub bind_addr: String,
    /// Maximum concurrent connections accepted by the listener.
    pub max_conns: usize,
    /// Target requests-per-second for simple rate limiting.
    pub rps: u64,
}

/// Request/response body limits and decompression caps.
#[derive(Debug, Clone, Deserialize)]
pub struct Limits {
    /// Maximum accepted request body size in bytes.
    pub max_body_bytes: u64,
    /// Absolute cap on decompressed body bytes.
    pub decode_abs_cap_bytes: u64,
}

/// Amnesia / logging / disk toggle placeholder.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Amnesia {
    /// When true, prefer RAM-only behavior and avoid disk where possible.
    pub enabled: bool,
}

/// Upstream service endpoints (omnigate app plane, later storage/overlay/index).
#[derive(Debug, Clone, Deserialize)]
pub struct Upstreams {
    /// Base URL for omnigate app plane (e.g. <http://127.0.0.1:9090>).
    pub omnigate_base_url: String,
}

fn default_bind_addr() -> String {
    "127.0.0.1:5304".to_owned()
}

fn default_omnigate_base_url() -> String {
    "http://127.0.0.1:9090".to_owned()
}

impl Default for Server {
    fn default() -> Self {
        Self {
            bind_addr: default_bind_addr(),
            max_conns: DEFAULT_MAX_CONNS,
            rps: DEFAULT_RPS,
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_body_bytes: DEFAULT_BODY_CAP_BYTES as u64,
            decode_abs_cap_bytes: DEFAULT_DECODE_ABS_CAP_BYTES as u64,
        }
    }
}

impl Default for Upstreams {
    fn default() -> Self {
        Self {
            omnigate_base_url: default_omnigate_base_url(),
        }
    }
}

/// Top-level config for svc-gateway.
///
/// RO:INVARS
/// - Defaults are safe for local dev.
/// - Env overrides are applied via `Config::load()`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    pub server: Server,
    pub limits: Limits,
    pub amnesia: Amnesia,
    pub upstreams: Upstreams,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if any env value is malformed.
    pub fn load() -> anyhow::Result<Self> {
        let mut cfg = Self::default();
        env::apply_env_overrides(&mut cfg)?;
        Ok(cfg)
    }
}
