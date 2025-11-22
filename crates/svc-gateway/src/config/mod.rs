//! RO:WHAT   Config model + loaders (env/file) with hard defaults.
//! RO:WHY    Keep caps & readiness guards aligned with blueprint.
//! Env prefix `SVC_GATEWAY`_. Docs show precedence + examples.

pub mod amnesia;
pub mod env;
pub mod safety;

use crate::consts::{
    DEFAULT_BODY_CAP_BYTES, DEFAULT_DECODE_ABS_CAP_BYTES, DEFAULT_DECODE_RATIO_MAX,
    DEFAULT_IDLE_TIMEOUT_SECS, DEFAULT_MAX_CONNS, DEFAULT_READ_TIMEOUT_SECS, DEFAULT_RPS,
    DEFAULT_WRITE_TIMEOUT_SECS,
};

use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: Server,
    pub limits: Limits,
    pub drr: Drr,
    pub amnesia: Amnesia,
    pub pq: Pq,
    pub safety: Safety,
    pub log: Log,
    pub upstreams: Upstreams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    pub bind_addr: SocketAddr,
    pub metrics_addr: SocketAddr,
    pub max_conns: usize,
    pub read_timeout_secs: u64,
    pub write_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Limits {
    pub max_body_bytes: usize,
    pub decode_abs_cap_bytes: usize,
    pub decode_ratio_max: usize,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Drr {
    pub default_quantum: u32,
    pub rate_limit_rps: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Amnesia {
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pq {
    pub mode: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Safety {
    pub danger_ok: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Log {
    pub format: String,
    pub level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Upstreams {
    /// Base URL for omnigate app plane (e.g. `<http://127.0.0.1:9090>`).
    pub omnigate_base_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: Server {
                bind_addr: "127.0.0.1:5304".parse().expect("static bind addr"),
                metrics_addr: "127.0.0.1:0".parse().expect("static metrics addr"),
                max_conns: DEFAULT_MAX_CONNS,
                read_timeout_secs: DEFAULT_READ_TIMEOUT_SECS,
                write_timeout_secs: DEFAULT_WRITE_TIMEOUT_SECS,
                idle_timeout_secs: DEFAULT_IDLE_TIMEOUT_SECS,
            },
            limits: Limits {
                max_body_bytes: DEFAULT_BODY_CAP_BYTES,
                decode_abs_cap_bytes: DEFAULT_DECODE_ABS_CAP_BYTES,
                decode_ratio_max: DEFAULT_DECODE_RATIO_MAX,
            },
            drr: Drr {
                default_quantum: 1,
                rate_limit_rps: DEFAULT_RPS,
            },
            amnesia: Amnesia { enabled: false },
            pq: Pq {
                mode: "off".to_owned(),
            },
            safety: Safety { danger_ok: false },
            log: Log {
                format: "json".to_owned(),
                level: "info".to_owned(),
            },
            upstreams: Upstreams {
                omnigate_base_url: "http://127.0.0.1:9090".to_owned(),
            },
        }
    }
}

impl Config {
    /// Load configuration using env overlays.
    ///
    /// # Errors
    ///
    /// Propagates parse/validation errors from the env loader.
    pub fn load() -> anyhow::Result<Self> {
        env::load()
    }
}
