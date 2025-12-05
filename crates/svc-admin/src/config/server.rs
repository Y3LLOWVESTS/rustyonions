// crates/svc-admin/src/config/server.rs
//
// WHAT: Server / listener configuration for svc-admin.
// WHY: Keeps network-related knobs isolated from other config concerns.

use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

/// HTTP listener + runtime tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCfg {
    /// Primary UI/API bind address (host:port).
    pub bind_addr: String,

    /// Health/metrics bind address (host:port).
    pub metrics_addr: String,

    /// Maximum number of concurrent connections the server will accept.
    pub max_conns: usize,

    /// Read timeout for upstream/admin HTTP calls.
    #[serde(skip)]
    pub read_timeout: Duration,

    /// Write timeout for upstream/admin HTTP calls.
    #[serde(skip)]
    pub write_timeout: Duration,

    /// Idle timeout for upstream/admin HTTP calls.
    #[serde(skip)]
    pub idle_timeout: Duration,

    /// TLS settings for the admin service itself.
    pub tls: TlsCfg,
}

/// TLS config for svc-admin’s own listeners.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsCfg {
    /// Whether TLS is enabled for the public/admin listener.
    pub enabled: bool,

    /// Path to certificate PEM (if TLS enabled).
    pub cert_path: Option<PathBuf>,

    /// Path to private key PEM (if TLS enabled).
    pub key_path: Option<PathBuf>,
}

impl Default for TlsCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            cert_path: None,
            key_path: None,
        }
    }
}

impl Default for ServerCfg {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:5300".to_string(),
            metrics_addr: "127.0.0.1:5310".to_string(),
            max_conns: 1024,
            read_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(60),
            tls: TlsCfg::default(),
        }
    }
}
