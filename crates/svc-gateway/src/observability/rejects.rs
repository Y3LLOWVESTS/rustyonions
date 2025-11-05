//! Shared “rejections” counter handle.
//! RO:WHAT   Central place for `gateway_rejections_total{reason}`.
//! RO:WHY    Avoid double-register; give callers a tiny, stable API.

use once_cell::sync::OnceCell;
use prometheus::{IntCounterVec, Opts};

const NAME: &str = "gateway_rejections_total";

/// Get the shared rejections counter (`reason` label).
///
/// # Panics
/// Panics once at process start if Prometheus registration fails. This
/// indicates a programmer error such as attempting to re-register the
/// same metric name with a different type/help text.
pub fn counter() -> &'static IntCounterVec {
    static CTR: OnceCell<IntCounterVec> = OnceCell::new();
    CTR.get_or_init(|| {
        let vec = IntCounterVec::new(Opts::new(NAME, "Gateway rejections by reason"), &["reason"])
            .expect("IntCounterVec");
        prometheus::register(Box::new(vec.clone())).expect("register gateway_rejections_total");
        vec
    })
}
