//! RO:WHAT
//! Omnigate middleware stack assembly.
//!
//! Order matters — inexpensive, shedding layers first.

use axum::Router;

pub mod body_caps;
pub mod classify;
pub mod corr_id;
pub mod decompress_guard;
pub mod slow_loris;

pub fn apply(router: Router) -> Router {
    // NOTE: Admission is currently a no-op shim to keep bounds simple.
    // When fair_queue/quotas are ready, we can call `crate::admission::attach(router)`
    // *before* the rest of the layers.
    router
        // Correlation ID before responses are built.
        .layer(corr_id::layer())
        // Gentle early classification (currently NOP placeholder).
        .layer(classify::layer())
        // Guard against content-encoding pitfalls.
        .layer(decompress_guard::layer())
        // Body size caps & preflight length checks.
        .layer(body_caps::layer())
        // Slow-loris / header timeouts (placeholder for now).
        .layer(slow_loris::layer())
}
