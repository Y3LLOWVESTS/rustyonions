// crates/ron-kernel/src/overlay/metrics.rs
#![forbid(unsafe_code)]
// We allow expect() ONLY during metric construction at startup (never in hot paths).
#![allow(clippy::expect_used)]

use prometheus::{IntCounter, IntGauge};
use prometheus::{register_int_counter, register_int_gauge};

#[derive(Clone)]
pub struct OverlayMetrics {
    /// Total accepted overlay connections
    pub accepted_total: IntCounter,
    /// Total rejected overlay connections (at capacity)
    pub rejected_total: IntCounter,
    /// Current active overlay connections
    pub active_conns: IntGauge,
    /// TLS handshake failures (timeout or error)
    pub handshake_failures_total: IntCounter,
    /// Read timeouts before idle budget exhausted
    pub read_timeouts_total: IntCounter,
    /// Connections closed due to idle timeout
    pub idle_timeouts_total: IntCounter,
    /// Last applied ConfigUpdated version for overlay
    pub cfg_version: IntGauge,
    /// Current overlay max connections
    pub max_conns_gauge: IntGauge,
}

impl OverlayMetrics {
    pub fn new() -> Self {
        let accepted_total = reg_counter(
            "overlay_accepted_total",
            "Total accepted overlay connections",
        );
        let rejected_total = reg_counter(
            "overlay_rejected_total",
            "Total rejected overlay connections (at capacity)",
        );
        let active_conns = reg_gauge(
            "overlay_active_connections",
            "Current active overlay connections",
        );
        let handshake_failures_total = reg_counter(
            "overlay_handshake_failures_total",
            "TLS handshake failures (timeout or error)",
        );
        let read_timeouts_total = reg_counter(
            "overlay_read_timeouts_total",
            "Read timeouts before idle budget exhausted",
        );
        let idle_timeouts_total = reg_counter(
            "overlay_idle_timeouts_total",
            "Connections closed due to idle timeout",
        );
        let cfg_version = reg_gauge(
            "overlay_config_version",
            "Last applied ConfigUpdated version for overlay",
        );
        let max_conns_gauge =
            reg_gauge("overlay_max_conns", "Current overlay max connections");

        Self {
            accepted_total,
            rejected_total,
            active_conns,
            handshake_failures_total,
            read_timeouts_total,
            idle_timeouts_total,
            cfg_version,
            max_conns_gauge,
        }
    }
}

/// Public initializer expected by `overlay/mod.rs`.
/// Safe to call multiple times; the registry tolerates AlreadyRegistered.
pub fn init_overlay_metrics() -> OverlayMetrics {
    OverlayMetrics::new()
}

impl Default for OverlayMetrics {
    fn default() -> Self {
        Self::new()
    }
}

fn reg_counter(name: &'static str, help: &'static str) -> IntCounter {
    // Registration can fail if already registered or due to a bad name; we fall back
    // to a constructed counter so callers can still record metrics (unregistered is fine).
    register_int_counter!(name, help).unwrap_or_else(|e| {
        eprintln!("prometheus: failed to register counter {name}: {e}");
        prometheus::IntCounter::new(format!("{name}_fallback"), help.to_string())
            .expect("fallback IntCounter")
    })
}

fn reg_gauge(name: &'static str, help: &'static str) -> IntGauge {
    register_int_gauge!(name, help).unwrap_or_else(|e| {
        eprintln!("prometheus: failed to register gauge {name}: {e}");
        prometheus::IntGauge::new(format!("{name}_fallback"), help.to_string())
            .expect("fallback IntGauge")
    })
}
