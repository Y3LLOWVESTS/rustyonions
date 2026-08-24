//! RO:WHAT — Truthful dependency/capability readiness for Macronode/CrabNode.
//! RO:WHY — Operators and clients need usable-service truth rather than process/port truth.
//! RO:INTERACTS — ReadyProbes, ReadyDeps, ReadyCapabilities, admin `/readyz`.
//! RO:INVARIANTS — required product core gates ready=true; dev override remains visibly dev-forced.
//! RO:METRICS — read-only projection.
//! RO:CONFIG — MACRONODE_DEV_READY is explicit development-only override.
//! RO:SECURITY — readiness contains no secrets or authority material.
//! RO:TEST — readiness unit test and live CN-3 ingress acceptance.

mod deps;
mod probes;

pub use probes::ReadyProbes;

use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};

use std::sync::Arc;

use self::deps::{ReadyBody, ReadyCapabilities, ReadyDeps};

fn dev_override_enabled() -> bool {
    matches!(
        std::env::var("MACRONODE_DEV_READY",).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("on") | Ok("ON")
    )
}

pub async fn handler(probes: Arc<ReadyProbes>) -> impl IntoResponse {
    if dev_override_enabled() {
        let snap = probes.snapshot();

        let deps = ReadyDeps::from_snapshot(&snap);

        let capabilities = ReadyCapabilities::from_snapshot(&snap);

        let body = ReadyBody::new(true, deps, capabilities, "dev-forced");

        return (StatusCode::OK, Json(body)).into_response();
    }

    let snap = probes.snapshot();

    let ok = snap.required_ready();

    let deps = ReadyDeps::from_snapshot(&snap);

    let capabilities = ReadyCapabilities::from_snapshot(&snap);

    let mut headers = HeaderMap::new();

    if !ok {
        headers.insert("Retry-After", HeaderValue::from_static("5"));
    }

    let body = ReadyBody::new(ok, deps, capabilities, "truthful");

    let status = if ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, headers, Json(body)).into_response()
}
