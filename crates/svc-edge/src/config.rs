//! Typed config with strict defaults and validation.

use serde::Deserialize;
use std::{fs, net::SocketAddr};

/// Service configuration (first increment: admin plane and reserved API bind).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// API bind address (reserved for next increment; logged only right now).
    #[serde(default = "default_api")]
    pub bind_addr: SocketAddr,
    /// Metrics/health/ready bind (admin plane).
    #[serde(default = "default_metrics")]
    pub metrics_addr: SocketAddr,
    /// Security posture. Additional knobs will arrive as the feature set grows.
    #[serde(default)]
    pub security: Security,
}

/// Security posture for the service.
///
/// In this increment we only expose the `amnesia` toggle to signal ephemeral
/// mode. Later, persistence/backends will tie into readiness if `amnesia` is on.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Security {
    /// Whether the node runs in “amnesia mode” (strong ephemerality, no
    /// persistent state). This currently only drives metrics/readiness semantics.
    pub amnesia: bool,
}

fn default_api() -> SocketAddr {
    "0.0.0.0:8080".parse().unwrap()
}
fn default_metrics() -> SocketAddr {
    "127.0.0.1:9909".parse().unwrap()
}

impl Config {
    /// Load configuration from a TOML file if provided, otherwise from
    /// environment variables with baked-in defaults.
    pub fn from_sources(toml_path: Option<&str>) -> anyhow::Result<Self> {
        if let Some(p) = toml_path {
            let text = fs::read_to_string(p)?;
            let cfg: Config = toml::from_str(&text)?;
            cfg.validate()?;
            return Ok(cfg);
        }
        // Minimal env reading (future: env prefix SVC_EDGE_* → fields)
        let cfg = Config {
            bind_addr: std::env::var("SVC_EDGE_BIND_ADDR")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_api),
            metrics_addr: std::env::var("SVC_EDGE_METRICS_ADDR")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_metrics),
            security: Security {
                amnesia: std::env::var("SVC_EDGE_SECURITY__AMNESIA")
                    .ok()
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false),
            },
        };
        cfg.validate()?;
        Ok(cfg)
    }

    /// Validate configuration invariants.
    ///
    /// In this initial increment, there are no cross-field constraints yet.
    fn validate(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
