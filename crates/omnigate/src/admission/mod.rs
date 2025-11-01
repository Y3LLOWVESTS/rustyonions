//! RO:WHAT   Admission composite attach point.
//! RO:WHY    Single place to enable quotas + fair-queue before handlers.
//! RO:INVARS Layers are low-cardinality; return 429/503 only.

mod fair_queue;
mod quotas;

use axum::Router;

/// Attach admission layers (quotas first, then fairness shed).
///
/// Bounds align with sublayers so `Router::layer` has what it needs.
pub fn attach<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    // Quotas first (fast reject), then fairness gate.
    fair_queue::attach(quotas::attach(router))
}
