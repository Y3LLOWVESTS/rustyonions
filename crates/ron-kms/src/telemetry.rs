//! RO:WHAT  Lazy, single global access to KMS Prometheus metrics.
//! RO:WHY   Avoid passing metrics handles everywhere; keep it opt-in via the `with-metrics` feature.

#![cfg(feature = "with-metrics")]

use crate::metrics::KmsMetrics;
use once_cell::sync::OnceCell;

static METRICS: OnceCell<KmsMetrics> = OnceCell::new();

/// Get the process-global metrics set (registers on first use).
#[must_use]
pub fn metrics() -> &'static KmsMetrics {
    METRICS.get_or_init(KmsMetrics::register)
}
