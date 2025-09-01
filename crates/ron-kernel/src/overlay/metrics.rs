#![forbid(unsafe_code)]

use prometheus::{IntCounter, IntGauge, register_int_counter, register_int_gauge};

#[derive(Clone)]
pub struct OverlayMetrics {
    pub accepted_total: IntCounter,
    pub rejected_total: IntCounter,
    pub active_conns: IntGauge,
    pub handshake_failures_total: IntCounter,
    pub read_timeouts_total: IntCounter,
    pub idle_timeouts_total: IntCounter,
    pub cfg_version: IntGauge,
    pub max_conns_gauge: IntGauge,
}

pub fn init_overlay_metrics() -> OverlayMetrics {
    let accepted_total = register_int_counter!("overlay_accepted_total", "Total accepted overlay connections")
        .expect("register overlay_accepted_total");
    let rejected_total = register_int_counter!("overlay_rejected_total", "Total rejected overlay connections (at capacity)")
        .expect("register overlay_rejected_total");
    let active_conns   = register_int_gauge!("overlay_active_connections", "Current active overlay connections")
        .expect("register overlay_active_connections");
    let handshake_failures_total = register_int_counter!("overlay_handshake_failures_total", "TLS handshake failures (timeout or error)")
        .expect("register overlay_handshake_failures_total");
    let read_timeouts_total = register_int_counter!("overlay_read_timeouts_total", "Read timeouts before idle budget exhausted")
        .expect("register overlay_read_timeouts_total");
    let idle_timeouts_total = register_int_counter!("overlay_idle_timeouts_total", "Connections closed due to idle timeout")
        .expect("register overlay_idle_timeouts_total");
    let cfg_version = register_int_gauge!("overlay_config_version", "Last applied ConfigUpdated version for overlay")
        .expect("register overlay_config_version");
    let max_conns_gauge = register_int_gauge!("overlay_max_conns", "Current overlay max connections")
        .expect("register overlay_max_conns");

    OverlayMetrics {
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
