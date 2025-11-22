//! RO:WHAT — Metrics plumbing for Macronode.
//! RO:WHY  — Keep a home for Prometheus registration and HTTP-layer metrics.
//! RO:INVARIANTS —
//!   - Metric families are registered against the default Prometheus registry.
//!   - This module is safe to call from multiple threads; registration is
//!     guarded so we only build metric handles once.

use std::sync::OnceLock;

use prometheus::{Encoder, Gauge, Opts, TextEncoder};

/// Simple macronode metric set.
///
/// We keep this intentionally tiny for now: just uptime + ready flag.
/// This is enough to make `/metrics` non-empty and to give operators
/// a quick at-a-glance signal without pulling `/api/v1/status`.
struct MacronodeMetrics {
    uptime_seconds: Gauge,
    ready: Gauge,
}

static METRICS: OnceLock<MacronodeMetrics> = OnceLock::new();

fn metrics() -> &'static MacronodeMetrics {
    METRICS.get_or_init(|| {
        let uptime_opts = Opts::new(
            "macronode_uptime_seconds",
            "Seconds since this macronode process started.",
        )
        .namespace("ron");

        let ready_opts = Opts::new(
            "macronode_ready",
            "1 if macronode reports ready=true, 0 otherwise.",
        )
        .namespace("ron");

        let uptime_seconds = Gauge::with_opts(uptime_opts).expect("macronode_uptime_seconds gauge");
        let ready = Gauge::with_opts(ready_opts).expect("macronode_ready gauge");

        // Register with the default registry; failures here are fatal because
        // they indicate programmer error (duplicate names, etc.).
        prometheus::register(Box::new(uptime_seconds.clone()))
            .expect("register macronode_uptime_seconds");
        prometheus::register(Box::new(ready.clone())).expect("register macronode_ready");

        MacronodeMetrics {
            uptime_seconds,
            ready,
        }
    })
}

/// Update macronode-local metrics.
///
/// This is cheap enough to call whenever we build `/api/v1/status`, so we
/// keep the call surface simple: the admin path passes in its computed
/// uptime + readiness bit.
pub fn update_macronode_metrics(uptime_seconds: u64, ready: bool) {
    let m = metrics();
    m.uptime_seconds.set(uptime_seconds as f64);
    m.ready.set(if ready { 1.0 } else { 0.0 });
}

/// Encode all registered metrics in Prometheus text format.
///
/// This is intentionally minimal for the first pass; other crates in the
/// workspace may also register metrics against the default registry.
pub fn encode_prometheus() -> String {
    let metric_families = prometheus::gather();
    let encoder = TextEncoder::new();
    let mut buf = Vec::new();
    if let Err(err) = encoder.encode(&metric_families, &mut buf) {
        eprintln!("[macronode-metrics] encode error: {err}");
        return String::new();
    }

    String::from_utf8(buf).unwrap_or_default()
}
