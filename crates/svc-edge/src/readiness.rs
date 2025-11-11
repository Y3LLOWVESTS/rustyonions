//! /readyz handler: degrade-first semantics with reasons payload.

use crate::state::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use http::{header, HeaderValue};
use serde::Serialize;

/// JSON payload returned by `/readyz`.
#[derive(Serialize)]
struct ReadyPayload {
    /// Whether the service is ready to serve traffic.
    ready: bool,
    /// Missing gates/conditions preventing readiness.
    missing: Vec<String>,
}

/// Report readiness based on `HealthState` keyed flags.
///
/// Current policy: ready when `services_ok` and `config_loaded` are true.
/// Returns 503 with `Retry-After: 1` when not ready.
pub async fn readiness_handler(State(state): State<AppState>) -> impl IntoResponse {
    // HealthState::snapshot() returns a BTreeMap<&str,bool> in this project.
    let snapshot = state.health.snapshot();
    let services_ok = *snapshot.get("services_ok").unwrap_or(&false);
    let config_loaded = *snapshot.get("config_loaded").unwrap_or(&false);

    if services_ok && config_loaded {
        return (StatusCode::OK, Json(ReadyPayload { ready: true, missing: vec![] }))
            .into_response();
    }

    let mut missing = Vec::new();
    if !services_ok {
        missing.push("services_ok".to_string());
    }
    if !config_loaded {
        missing.push("config_loaded".to_string());
    }

    let payload = ReadyPayload { ready: false, missing };
    let mut res = (StatusCode::SERVICE_UNAVAILABLE, Json(payload)).into_response();
    res.headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
    res
}
