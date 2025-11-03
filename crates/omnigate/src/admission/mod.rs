// crates/omnigate/src/admission/mod.rs
//! RO:WHAT   Admission composite attach point.
//! RO:WHY    Single place to enable quotas + fair-queue before handlers.
//! RO:INVARS Layers are low-cardinality; return 429/503 only.

mod fair_queue;
mod quotas;

use axum::Router;

/// Attach admission layers (quotas first, then fairness shed) using defaults.
/// Kept for tests/back-compat. Prefer `attach_with_cfg`.
pub fn attach<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    // Quotas first (fast reject), then fairness gate.
    fair_queue::attach(quotas::attach(router))
}

/// Attach admission layers using values from Config.
/// Order matters: quotas first, then fair-queue.
/// NOTE: Decompression guard is layered in `middleware::apply_with_cfg`.
pub fn attach_with_cfg<S>(router: Router<S>, cfg: &crate::config::Admission) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    // Pass the whole Admission to quotas (it needs both global & ip slices)
    let r = quotas::attach_with_cfg(router, cfg);
    // Then pass just the FairQueue part to the fairness gate
    fair_queue::attach_with_cfg(r, &cfg.fair_queue)
}
