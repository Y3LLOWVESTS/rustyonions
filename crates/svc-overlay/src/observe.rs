//! Observability bootstrap + metric helpers.
//! Contract: Prometheus exposition via /metrics with stable names/buckets.
//! See docs/OBSERVABILITY.md and API.MD for the golden set.
//
// NOTE: We intentionally avoid calling `metrics::*` macros here because the
// workspace currently pulls in two different `metrics` versions via
// `metrics-exporter-prometheus`, which causes type/trait conflicts.
// The `emit` helpers are kept as no-ops so call sites compile. Once we unify
// on a single `metrics` version across the workspace, we can flip these back
// on without changing call sites.

use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use once_cell::sync::OnceCell;
use std::net::SocketAddr;
use tracing::info;

// Global handle used by /metrics handler to render a scrape.
static PROM_HANDLE: OnceCell<PrometheusHandle> = OnceCell::new();

/// Install a Prometheus recorder and store a handle for /metrics scraping.
/// Returns the configured bind address (HTTP listener is owned by the exporter).
pub fn init_metrics(addr: SocketAddr) -> anyhow::Result<SocketAddr> {
    // Buckets aligned with docs: latency 5ms..5s, frame sizes up to 1MiB.
    let builder = PrometheusBuilder::new()
        .with_http_listener(addr)
        // metrics-exporter-prometheus 0.15 uses (Matcher, &[f64]) — set each metric separately.
        .set_buckets_for_metric(
            Matcher::Full("request_latency_seconds".into()),
            &[0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0],
        )?
        .set_buckets_for_metric(
            Matcher::Full("overlay_frame_size_bytes".into()),
            &[
                512.0,
                1024.0,
                4096.0,
                16384.0,
                65536.0,
                262_144.0,
                524_288.0,
                1_048_576.0,
            ],
        )?;

    // Start exporter + install recorder.
    let handle = builder.install_recorder()?;

    // Touch to ensure recorder is live (silence unused warnings).
    let _ = handle.render().len();

    // Store global handle for /metrics endpoint.
    let _ = PROM_HANDLE.set(handle);

    info!("metrics recorder installed on {}", addr);
    Ok(addr)
}

/// Render Prometheus metrics as text/plain; used by the HTTP handler.
pub fn render_prometheus() -> String {
    PROM_HANDLE
        .get()
        .map(|h| h.render())
        .unwrap_or_else(|| "# no recorder".to_string())
}

/// Canonical metric helpers (names stabilized here).
/// Currently NO-OPs to avoid `metrics` crate version conflicts.
/// Re-enable by replacing bodies with `metrics::*` macros once the workspace
/// is on a single `metrics` version.
pub mod emit {
    #[inline]
    pub fn http_req_total(_route: &'static str, _method: &'static str, _status: u16) {
        // NO-OP (see module docs)
    }

    #[inline]
    pub fn http_latency(_route: &'static str, _method: &'static str, _secs: f64) {
        // NO-OP (see module docs)
    }

    #[inline]
    pub fn ready_state(_val: i64) {
        // NO-OP (see module docs)
    }
}
