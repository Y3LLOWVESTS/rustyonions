//! RO:WHAT
//! Admission (pre-routing) attach point.
//!
//! We’ll re-introduce fair-queue and quota layers here once they satisfy
//! Axum’s `Router::layer` bounds (Service<Request<Body>> + Clone + Send + 'static).
//!
//! For now this is a no-op shim to keep the crate compiling cleanly.

use axum::Router;

/// Attach admission layers to a router (currently no-op).
pub fn attach<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    // TODO(admission): when `admission::fair_queue` and `admission::quotas`
    // are ready, do:
    //   router
    //     .layer(fair_queue::layer())
    //     .layer(quotas::layer())
    // For now, just return the router unchanged.
    router
}
