//! RO:WHAT — Slow-loris protection (stub).
//! RO:WHY  — Future: per-read header/body timeouts, idle caps; now identity.
//! RO:INVARIANTS — Avoid false positives under load when enabled.

use tower::layer::Layer;

/// Identity layer placeholder (no-op).
#[derive(Clone, Copy, Default)]
pub struct NopLayer;

impl<S> Layer<S> for NopLayer {
    type Service = S;
    fn layer(&self, inner: S) -> Self::Service {
        inner
    }
}

pub fn layer() -> NopLayer {
    NopLayer
}
