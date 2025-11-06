#![cfg(feature = "with-metrics")]

use once_cell::sync::OnceCell;
use crate::metrics::KmsMetrics;

static METRICS: OnceCell<KmsMetrics> = OnceCell::new();

#[must_use]
pub fn metrics() -> &'static KmsMetrics {
    METRICS.get_or_init(KmsMetrics::register)
}
