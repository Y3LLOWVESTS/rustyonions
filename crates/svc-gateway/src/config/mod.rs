//! Config model + defaults for `svc-gateway`.
//!
//! RO:WHAT — Minimal gateway config for bind addr, caps, and downstream upstreams.
//! RO:WHY — Keep ingress config explicit while TOML/file loaders mature.
//! RO:INTERACTS — `routes::app`, `routes::paid_storage`, `state::AppState`, `config::env`.
//! RO:INVARIANTS — defaults are local-dev safe; env overrides are validated where shape matters.
//! RO:METRICS — none.
//! RO:CONFIG — `SVC_GATEWAY_*` env vars through `Config::load()`.
//! RO:SECURITY — upstream URLs are operator-controlled; no secrets stored in this config model.
//! RO:TEST — `app_proxy.rs`, `paid_storage_estimate_proxy.rs`.

use crate::consts::{
    DEFAULT_BODY_CAP_BYTES, DEFAULT_DECODE_ABS_CAP_BYTES, DEFAULT_MAX_CONNS, DEFAULT_RPS,
};
use serde::Deserialize;

pub mod env;

/// Server-level configuration: bind address, connection caps, and RPS.
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

/// Upstream service endpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct Upstreams {
    /// Base URL for omnigate app plane, for example `http://127.0.0.1:9090`.
    pub omnigate_base_url: String,
    /// Base URL for `svc-storage`, for example `http://127.0.0.1:15303`.
    pub storage_base_url: String,
}

fn default_bind_addr() -> String {
    "127.0.0.1:5304".to_owned()
}

fn default_omnigate_base_url() -> String {
    "http://127.0.0.1:9090".to_owned()
}

fn default_storage_base_url() -> String {
    "http://127.0.0.1:15303".to_owned()
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
            storage_base_url: default_storage_base_url(),
        }
    }
}

/// Top-level config for `svc-gateway`.
///
/// RO:INVARIANTS
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
