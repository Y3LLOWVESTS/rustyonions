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

fn default_admin_ui_bind() -> SocketAddr {
    "127.0.0.1:5300"
        .parse()
        .expect("default 127.0.0.1:5300 must parse")
}

fn default_headless_mode() -> bool {
    true
}

fn default_operator_ui_profile() -> String {
    "service_node_local".to_string()
}

fn default_admin_ui_runtime_required() -> bool {
    false
}

fn default_admin_setup_token_ttl() -> Duration {
    Duration::from_secs(15 * 60)
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

    /// Whether the optional service-node admin UI is enabled.
    ///
    /// BUILD_PLAN_Z invariant: the service-node daemon must remain runnable
    /// without the UI. The default is therefore disabled/headless.
    #[serde(default)]
    pub admin_ui_enabled: bool,

    /// Bind address for the optional local operator UI.
    ///
    /// This must stay loopback-only in the CrabLink service-node profile.
    #[serde(default = "default_admin_ui_bind")]
    pub admin_ui_bind: SocketAddr,

    /// Whether the service-node daemon is headless-operable.
    ///
    /// For the CrabLink service-node profile this must remain true even when
    /// the optional UI is enabled on demand.
    #[serde(default = "default_headless_mode")]
    pub headless_mode: bool,

    /// Operator UI profile label surfaced to svc-admin / CrabLink controllers.
    #[serde(default = "default_operator_ui_profile")]
    pub operator_ui_profile: String,

    /// Whether the runtime requires the admin UI to operate.
    ///
    /// Must remain false for service nodes; this field exists so status checks
    /// can prove the UI is optional instead of implicit runtime authority.
    #[serde(default = "default_admin_ui_runtime_required")]
    pub admin_ui_runtime_required: bool,

    /// Lifetime of one runtime-local, one-use setup token.
    ///
    /// The default remains 15 minutes. Phase 23 chaos drills may use a
    /// shorter validated value, but production configuration cannot make
    /// setup credentials unbounded or effectively permanent.
    #[serde(default = "default_admin_setup_token_ttl", with = "humantime_serde")]
    pub admin_setup_token_ttl: Duration,

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
            admin_ui_enabled: false,
            admin_ui_bind: default_admin_ui_bind(),
            headless_mode: default_headless_mode(),
            operator_ui_profile: default_operator_ui_profile(),
            admin_ui_runtime_required: default_admin_ui_runtime_required(),
            admin_setup_token_ttl: default_admin_setup_token_ttl(),
            log_level: default_log_level(),
            read_timeout: default_read_timeout(),
            write_timeout: default_write_timeout(),
            idle_timeout: default_idle_timeout(),
        }
    }
}
