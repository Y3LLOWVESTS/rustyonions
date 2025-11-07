#![cfg(feature = "with-metrics")]

// RO:WHAT  Tiny shim so existing call-sites can do `telemetry::metrics()`.
// RO:WHY   Keep API stable; consumers can also use `metrics::GLOBAL`.

use crate::metrics::KmsMetrics;
use once_cell::sync::OnceCell;

static METRICS: OnceCell<KmsMetrics> = OnceCell::new();

#[must_use]
pub fn metrics() -> &'static KmsMetrics {
    METRICS.get_or_init(KmsMetrics::register)
}
