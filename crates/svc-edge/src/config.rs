//! Config for svc-edge.
//!
//! RO:WHAT
//! - Provides bind addresses, security posture, admission caps, and assets root.
//! - Loads from ENV (TOML reader can be added later without changing the API).
//!
//! RO:ENV
//! - `SVC_EDGE_BIND_ADDR`              (default `"0.0.0.0:8080"`)
//! - `SVC_EDGE_METRICS_ADDR`           (default `"127.0.0.1:9909"`)
//! - `SVC_EDGE_SECURITY__AMNESIA`      (default `"0"`)
//! - `SVC_EDGE_ADMISSION_TIMEOUT_MS`   (default `"5000"`)
//! - `SVC_EDGE_ADMISSION_MAX_INFLIGHT` (default `"256"`)
//! - `SVC_EDGE_ASSETS_DIR`             (default `"assets"`)

use std::{net::SocketAddr, path::PathBuf, str::FromStr};

/// Top-level configuration for the svc-edge process.
///
/// Values are derived from environment variables (see module docs).
#[derive(Clone, Debug)]
pub struct Config {
    /// Address for the public API plane (e.g., `/edge/assets/*`, `/cas/*`).
    pub bind_addr: SocketAddr,
    /// Address for the admin plane (e.g., `/healthz`, `/readyz`, `/metrics`).
    pub metrics_addr: SocketAddr,
    /// Security posture settings (e.g., amnesia mode).
    pub security: SecurityCfg,
    /// Admission guard settings (timeouts, inflight caps).
    pub admission: AdmissionCfg,
    /// Asset adapter settings (temporary FS root until pack/CAS adapters land).
    pub assets: AssetsCfg,
}

/// Security posture for the service.
#[derive(Clone, Debug)]
pub struct SecurityCfg {
    /// Amnesia mode (true = prefer RAM/avoid persistence; best-effort on this crate).
    pub amnesia: bool,
}

/// Admission caps for the API plane.
#[derive(Clone, Debug)]
pub struct AdmissionCfg {
    /// Per-request wall-clock timeout in milliseconds (→ HTTP 408 on expiry).
    pub timeout_ms: u64,
    /// Maximum number of inflight requests (→ HTTP 503 when saturated).
    pub max_inflight: usize,
}

/// Asset adapter configuration (temporary filesystem root).
#[derive(Clone, Debug)]
pub struct AssetsCfg {
    /// Filesystem root used by the current asset scaffold.
    pub root: PathBuf,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// The `_maybe_path` parameter is reserved for future TOML loading and is
    /// ignored for now to keep the public API stable.
    pub fn from_sources(_maybe_path: Option<&str>) -> anyhow::Result<Self> {
        let bind_addr = env_parse("SVC_EDGE_BIND_ADDR", "0.0.0.0:8080".to_string());
        let metrics_addr = env_parse("SVC_EDGE_METRICS_ADDR", "127.0.0.1:9909".to_string());
        let amnesia = env_parse_bool("SVC_EDGE_SECURITY__AMNESIA", false);
        let timeout_ms = env_parse("SVC_EDGE_ADMISSION_TIMEOUT_MS", 5_000u64);
        let max_inflight = env_parse("SVC_EDGE_ADMISSION_MAX_INFLIGHT", 256usize);
        let assets_root = std::env::var("SVC_EDGE_ASSETS_DIR").unwrap_or_else(|_| "assets".into());

        Ok(Self {
            bind_addr: parse_addr(bind_addr)?,
            metrics_addr: parse_addr(metrics_addr)?,
            security: SecurityCfg { amnesia },
            admission: AdmissionCfg {
                timeout_ms,
                max_inflight,
            },
            assets: AssetsCfg {
                root: PathBuf::from(assets_root),
            },
        })
    }
}

/// Parse a `SocketAddr` from a string.
fn parse_addr(s: String) -> anyhow::Result<SocketAddr> {
    Ok(SocketAddr::from_str(&s)?)
}

/// Parse an environment variable into a type implementing `FromStr`,
/// falling back to `default` on absence or parse failure.
fn env_parse<T: FromStr>(key: &str, default: T) -> T {
    match std::env::var(key) {
        Ok(v) => v.parse().unwrap_or(default),
        Err(_) => default,
    }
}

/// Parse an environment boolean with common truthy/falsey values
/// (e.g., `1/0`, `true/false`, `yes/no`, `on/off`).
fn env_parse_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => match v.as_str() {
            "1" | "true" | "TRUE" | "yes" | "on" => true,
            "0" | "false" | "FALSE" | "no" | "off" => false,
            _ => default,
        },
        Err(_) => default,
    }
}
