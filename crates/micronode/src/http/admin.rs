//! RO:WHAT — Admin plane: /metrics, /healthz, /readyz, /version.
//! RO:WHY  — Golden surfaces; shared by all RON services.
//! RO:INVARIANTS — Truthful readyz; explicit dev override in handler.

use crate::{observability, state::AppState};
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use prometheus::{Encoder, TextEncoder};

pub async fn metrics(_: State<AppState>) -> impl IntoResponse {
    let families = prometheus::gather();
    let mut buf = Vec::new();
    let _ = TextEncoder::new().encode(&families, &mut buf);
    (StatusCode::OK, buf)
}

pub async fn healthz() -> impl IntoResponse {
    observability::health::handler().await
}

pub async fn readyz(State(st): State<AppState>) -> impl IntoResponse {
    observability::ready::handler(st.probes.clone()).await
}

pub async fn version() -> impl IntoResponse {
    observability::version::handler().await
}
