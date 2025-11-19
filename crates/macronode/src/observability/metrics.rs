//! RO:WHAT — Metrics plumbing for Macronode (stub v1).
//! RO:WHY  — Keep a home for Prometheus registration and HTTP-layer metrics.
//! RO:INVARIANTS —
//!   - Module exists so tests can evolve without touching the rest of the app.
//!   - Metric families are all registered against the default Prometheus registry.

use prometheus::{Encoder, TextEncoder};

/// Encode all registered metrics in Prometheus text format.
///
/// This is intentionally minimal for the first pass; we will wire
/// HTTP-layer counters/histograms in a follow-up.
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
