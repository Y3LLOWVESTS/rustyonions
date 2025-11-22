// crates/macronode/src/readiness/mod.rs

//! RO:WHAT — Readiness probes and `/readyz` handler for Macronode.
//! RO:WHY  — Truthful, operator-friendly readiness for orchestration
//!           (K8s/systemd/CI) with a clean separation of concerns.
//!
//! RO:INVARIANTS —
//!   Essential gates for ready=true: listeners_bound && cfg_loaded && deps_ok && gateway_bound.
//!   Per-service bits (index/overlay/mailbox/dht) are tracked and exposed in the JSON `deps`
//!   payload but do not gate readiness yet.
//!   Dev override: MACRONODE_DEV_READY=1 forces `ready=true` while still exposing actual
//!   dependency states in the body.

mod deps;
mod probes;

pub use probes::ReadyProbes;

use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use self::deps::{ReadyBody, ReadyDeps};

/// Check whether the dev override is enabled via `MACRONODE_DEV_READY`.
fn dev_override_enabled() -> bool {
    matches!(
        std::env::var("MACRONODE_DEV_READY").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("on") | Ok("ON")
    )
}

/// Axum handler for `/readyz`.
///
/// Responsibilities:
///   - Snapshot probes (cheap, lock-free).
///   - Apply dev override semantics.
///   - Map snapshot → JSON deps/body using `deps` helpers.
///   - Attach `Retry-After` when not ready in truthful mode.
pub async fn handler(probes: Arc<ReadyProbes>) -> impl IntoResponse {
    // Dev override: force ready=true, but still report what Macronode knows
    // about each dependency so operators can see "what's actually happening".
    if dev_override_enabled() {
        let snap = probes.snapshot();
        let deps = ReadyDeps::from_snapshot(&snap);
        let body = ReadyBody::new(true, deps, "dev-forced");

        return (StatusCode::OK, Json(body)).into_response();
    }

    // Truthful mode: rely on the required_ready() invariant and surface
    // dependency states directly.
    let snap = probes.snapshot();
    let ok = snap.required_ready();
    let deps = ReadyDeps::from_snapshot(&snap);

    let mut headers = HeaderMap::new();
    if !ok {
        // Friendly hint to orchestrators / callers to back off before retrying.
        headers.insert("Retry-After", HeaderValue::from_static("5"));
    }

    let body = ReadyBody::new(ok, deps, "truthful");
    let status = if ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, headers, Json(body)).into_response()
}
