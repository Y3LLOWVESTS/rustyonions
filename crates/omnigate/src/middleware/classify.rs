//! RO:WHAT — Response classifier (stub).
//! RO:WHY  — Future: classify errors for metrics; integrate with tower_http::classify.
//! RO:INVARIANTS — Bounded label cardinality; now identity.

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
