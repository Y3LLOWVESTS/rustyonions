// crates/omnigate/src/middleware/mod.rs
//! RO:WHAT  Shared middleware stack for the app.
//! RO:WHY   Keep router layering in one place (except admission which is cfg-driven).
//! RO:INVARS Low overhead; stable label cardinality; no blocking across .await.

mod body_caps;
mod classify;
mod corr_id;
mod decompress_guard;
mod policy;
mod slow_loris;

use crate::config::Admission;
use axum::Router;

/// Canonical middleware stack **with config** (recommended).
/// Order: classify -> corr_id -> policy -> body caps -> default body limit -> decompress guard -> slow-loris
pub fn apply_with_cfg<S>(router: Router<S>, adm: &Admission) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    // Cheap gates first (no body access)
    let router = router
        .layer(classify::layer())
        .layer(corr_id::layer())
        .layer(policy::layer());

    // Preflight length + Axum's DefaultBodyLimit pair
    let (preflight_len_guard, default_body_limit) = body_caps::layer();

    // Attach caps/limit, then cfg-driven decompression guard, then slow-loris
    let router = router.layer(preflight_len_guard).layer(default_body_limit);

    // Config-driven decompression guard; this call ensures the symbol is referenced in this module.
    let router = decompress_guard::attach_with_cfg(router, adm);

    router.layer(slow_loris::layer())
}

/// Legacy shim kept for tests/back-compat (no config at callsite).
/// Internally calls `apply_with_cfg` using `Admission::default()`.
pub fn apply<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let defaults = crate::config::Admission::default();
    apply_with_cfg(router, &defaults)
}
