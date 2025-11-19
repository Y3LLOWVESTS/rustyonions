//! RO:WHAT — `/api/v1/status` handler (MVP).
//! RO:WHY  — Give operators a basic runtime snapshot.

use std::time::Instant;

use axum::{response::IntoResponse, Json};
use serde::Serialize;

use crate::types::AppState;

#[derive(Serialize)]
struct StatusBody {
    uptime_seconds: u64,
    profile: &'static str,
    http_addr: String,
    log_level: String,
}

pub async fn handler(state: axum::extract::State<AppState>) -> impl IntoResponse {
    let AppState {
        cfg, started_at, ..
    } = state.0;

    let uptime = Instant::now()
        .saturating_duration_since(started_at)
        .as_secs();

    Json(StatusBody {
        uptime_seconds: uptime,
        profile: "macronode",
        http_addr: cfg.http_addr.to_string(),
        log_level: cfg.log_level.clone(),
    })
}
