//! Concurrency guard (route-scoped).
//! RO:WHAT  Cheap backpressure using a `Semaphore` permit per in-flight request.
//! RO:WHY   Fail fast with `503 Service Unavailable` instead of tail spikes.
//! RO:NOTE  Axum 0.7 middleware shape (`Next` has no generics). Global static,
//!          but applied only to selected routes (e.g., `/readyz`).

use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use once_cell::sync::OnceCell;
use tokio::sync::{Semaphore, SemaphorePermit};

fn semaphore() -> &'static Semaphore {
    static SEM: OnceCell<Semaphore> = OnceCell::new();
    SEM.get_or_init(|| {
        // ENV knob, default small to make behavior easy to see locally.
        let max = std::env::var("SVC_GATEWAY_READY_MAX_INFLIGHT")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(64);
        Semaphore::new(max)
    })
}

/// Acquire a single permit or return 503 when saturated.
pub async fn ready_concurrency_mw(req: Request<Body>, next: Next) -> Response {
    // Fast-fail if saturated (non-blocking).
    let permit: Option<SemaphorePermit<'_>> = semaphore().try_acquire().ok();
    if permit.is_none() {
        return (axum::http::StatusCode::SERVICE_UNAVAILABLE, "busy").into_response();
    }

    // Hold the permit for the duration of the downstream call.
    let _permit = permit;
    next.run(req).await
}
