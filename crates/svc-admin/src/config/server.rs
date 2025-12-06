// crates/svc-admin/src/config/server.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

fn default_bind_addr() -> String {
    "127.0.0.1:5300".to_string()
}

fn default_metrics_addr() -> String {
    "127.0.0.1:5310".to_string()
}

fn default_max_conns() -> usize {
    1024
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

fn default_tls_enabled() -> bool {
    false
}

fn default_cert_path() -> Option<PathBuf> {
    None
}

fn default_key_path() -> Option<PathBuf> {
    None
}

/// TLS settings for the svc-admin HTTP server.
///
/// For the dev-preview we typically run behind a reverse proxy or on localhost
/// over plain HTTP. TLS is wired but defaults to disabled.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsCfg {
    /// Whether TLS is enabled for the UI/API port.
    #[serde(default = "default_tls_enabled")]
    pub enabled: bool,

    /// Path to the PEM-encoded certificate.
    #[serde(default = "default_cert_path")]
    pub cert_path: Option<PathBuf>,

    /// Path to the PEM-encoded private key.
    #[serde(default = "default_key_path")]
    pub key_path: Option<PathBuf>,
}

/// Top-level server configuration.
///
/// This mirrors the invariants described in the svc-admin IDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCfg {
    /// Bind address for the UI/API listener.
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,

    /// Bind address for the health/metrics listener.
    #[serde(default = "default_metrics_addr")]
    pub metrics_addr: String,

    /// Maximum concurrent connections accepted by the server.
    #[serde(default = "default_max_conns")]
    pub max_conns: usize,

    /// Per-connection read timeout.
    #[serde(default = "default_read_timeout")]
    pub read_timeout: Duration,

    /// Per-connection write timeout.
    #[serde(default = "default_write_timeout")]
    pub write_timeout: Duration,

    /// Idle timeout before closing connections.
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout: Duration,

    /// TLS settings for the UI/API port.
    #[serde(default)]
    pub tls: TlsCfg,
}

impl Default for ServerCfg {
    fn default() -> Self {
        Self {
            bind_addr: default_bind_addr(),
            metrics_addr: default_metrics_addr(),
            max_conns: default_max_conns(),
            read_timeout: default_read_timeout(),
            write_timeout: default_write_timeout(),
            idle_timeout: default_idle_timeout(),
            tls: TlsCfg::default(),
        }
    }
}
