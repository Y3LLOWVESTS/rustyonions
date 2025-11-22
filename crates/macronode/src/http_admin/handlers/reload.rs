//! RO:WHAT — `/api/v1/reload` handler.
//! RO:WHY  — Trigger config hot reload (stub v1) and emit a bus event.
//!
//! RO:INVARIANTS —
//!   - Must run under admin auth middleware.
//!   - Uses `config::hot_reload()` (stub for now).
//!   - Async safe; returns 202-style semantics (we currently reply 200 OK).

use axum::{response::IntoResponse, Json};
use serde::Serialize;
use tracing::info;

use crate::{bus::NodeEvent, config, types::AppState};

#[derive(Serialize)]
struct ReloadResp {
    status: &'static str,
}

pub async fn handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    info!("macronode admin: reload requested");

    // Call into our stub for now — later will reload config + emit events.
    if let Err(e) = config::hot_reload(&state.cfg) {
        info!("macronode admin: reload failed: {e}");
    }

    // Emit a ConfigUpdated event on the intra-node bus.
    //
    // NOTE: Version is currently stubbed as 0. Once we track config epochs
    // or generation IDs, this should carry the real version.
    if let Err(send_err) = state.bus.publish(NodeEvent::ConfigUpdated { version: 0 }) {
        info!("macronode admin: failed to publish ConfigUpdated event on bus: {send_err:?}");
    }

    Json(ReloadResp {
        status: "reload triggered",
    })
}
