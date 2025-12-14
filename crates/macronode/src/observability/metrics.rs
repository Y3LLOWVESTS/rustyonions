// crates/macronode/src/observability/metrics.rs

//! RO:WHAT — Metrics plumbing for Macronode.
//! RO:WHY  — Keep a home for Prometheus registration and HTTP-layer metrics.
//! RO:INVARIANTS —
//!   - Metric families are registered against the default Prometheus registry.
//!   - This module is safe to call from multiple threads; registration is
//!     guarded so we only build metric handles once.
//!
//! RO:FACETS —
//!   - Exposes `ron_macronode_uptime_seconds` and `ron_macronode_ready`.
//!   - Exposes `ron_facet_requests_total{facet, result}` so svc-admin can
//!     aggregate per-facet RPS/error rates for this node.

use std::sync::OnceLock;

use prometheus::{Encoder, Gauge, IntCounterVec, Opts, TextEncoder};

/// Simple macronode metric set.
///
/// We keep this intentionally small but *expressive*:
///   - `ron_macronode_uptime_seconds`
///   - `ron_macronode_ready`
///   - `ron_facet_requests_total{facet, result}`
///
/// The facet counter is what svc-admin consumes when building facet cards
/// and per-node RPS graphs.
struct MacronodeMetrics {
    uptime_seconds: Gauge,
    ready: Gauge,
    facet_requests_total: IntCounterVec,
}

static METRICS: OnceLock<MacronodeMetrics> = OnceLock::new();

fn metrics() -> &'static MacronodeMetrics {
    METRICS.get_or_init(|| {
        //
        // 1) Uptime + readiness gauges
        //
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

        let uptime_seconds =
            Gauge::with_opts(uptime_opts).expect("macronode_uptime_seconds gauge");
        let ready = Gauge::with_opts(ready_opts).expect("macronode_ready gauge");

        //
        // 2) Facet request counter
        //
        // Full name: `ron_facet_requests_total`
        //
        // Labels:
        //   - facet  — logical facet name ("admin.status", "admin.healthz", "gateway.app", ...).
        //   - result — "ok" | "error" (room to extend later if we want finer granularity).
        //
        let facet_opts = Opts::new(
            "facet_requests_total",
            "Total facet requests observed by this macronode, labeled by facet and result.",
        )
        .namespace("ron");

        let facet_requests_total = IntCounterVec::new(facet_opts, &["facet", "result"])
            .expect("facet_requests_total counter vec");

        // Register with the default registry; failures here are fatal because
        // they indicate programmer error (duplicate names, etc.).
        prometheus::register(Box::new(uptime_seconds.clone()))
            .expect("register macronode_uptime_seconds");
        prometheus::register(Box::new(ready.clone()))
            .expect("register macronode_ready");
        prometheus::register(Box::new(facet_requests_total.clone()))
            .expect("register ron_facet_requests_total");

        MacronodeMetrics {
            uptime_seconds,
            ready,
            facet_requests_total,
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

/// Internal helper to record a facet event with an arbitrary result label.
fn observe_facet(facet: &str, result: &str) {
    let m = metrics();
    m.facet_requests_total
        .with_label_values(&[facet, result])
        .inc();
}

/// Record a successful request for a given facet.
///
/// Example facets in this slice:
///   - "admin.status"  — GET `/api/v1/status`
///   - "admin.healthz" — GET `/healthz`
pub fn observe_facet_ok(facet: &str) {
    observe_facet(facet, "ok");
}

/// Record a failed request for a given facet.
///
/// Not heavily used in this thin slice yet, but the hook is here so future
/// handlers (or middleware) can record error paths consistently.
pub fn observe_facet_error(facet: &str) {
    observe_facet(facet, "error");
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
