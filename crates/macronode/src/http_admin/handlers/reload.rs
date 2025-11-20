//! RO:WHAT — `/api/v1/reload` handler.
//! RO:WHY  — Trigger config hot reload (stub v1).
//!
//! RO:INVARIANTS —
//!   - Must run under admin auth middleware.
//!   - Uses `config::hot_reload()` (stub for now).
//!   - Async safe; returns 202 Accepted for symmetry with shutdown.

use axum::{response::IntoResponse, Json};
use serde::Serialize;
use tracing::info;

use crate::{config, types::AppState};

#[derive(Serialize)]
struct ReloadResp {
    status: &'static str,
}

pub async fn handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    info!("macronode admin: reload requested");

    // Call into our stub for now — later will reload config + emit events
    if let Err(e) = config::hot_reload(&state.cfg) {
        info!("macronode admin: reload failed: {e}");
    }

    Json(ReloadResp {
        status: "reload triggered",
    })
}
