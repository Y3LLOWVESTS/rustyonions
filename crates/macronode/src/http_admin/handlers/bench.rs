// crates/macronode/src/http_admin/handlers/bench.rs
//
// RO:WHAT  — HTTP handlers for node-executed benchmarks.
// RO:WHY   — Expose /api/v1/bench/* so svc-admin can run/poll benchmarks safely.
// RO:INVARIANTS —
//   - Bounded inputs; node enforces caps (duration, concurrency, active runs).
//   - No locks held across .await in the loadgen engine.
//   - Only curated suites are allowed (no arbitrary URL cannon).
// RO:SECURITY —
//   - Intended to sit behind admin auth middleware.
//   - Error mapping is conservative (400 for bad request, 429 for rejected).

#![forbid(unsafe_code)]

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::{
    bench::{BenchRunReq, BenchRunResp, BenchRunResultDto, BenchRunStatusDto},
    types::AppState,
};

/// POST /api/v1/bench/run
///
/// Starts a new benchmark run on this node. The node enforces caps on
/// duration, concurrency, and number of active runs.
pub async fn run(
    State(state): State<AppState>,
    Json(mut req): Json<BenchRunReq>,
) -> Result<Json<BenchRunResp>, StatusCode> {
    // Default suite if caller omitted/blank; matches svc-admin UI default.
    if req.suite.trim().is_empty() {
        req.suite = "admin_plane".to_string();
    }

    match state.bench.start(req).await {
        Ok(resp) => Ok(Json(resp)),
        Err(err) => {
            let msg = err.to_string();

            // BenchError::Rejected(...)
            if msg.contains("another benchmark is already active")
                || msg.to_lowercase().contains("rejected")
            {
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }

            // BenchError::InvalidSuite(...)
            if msg.to_lowercase().contains("invalid suite") {
                return Err(StatusCode::BAD_REQUEST);
            }

            // Everything else is treated as an internal bench failure.
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v1/bench/runs/:run_id
pub async fn status(
    Path(run_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<BenchRunStatusDto>, StatusCode> {
    match state.bench.status(&run_id).await {
        Some(status) => Ok(Json(status)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// GET /api/v1/bench/runs/:run_id/result
pub async fn result(
    Path(run_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<BenchRunResultDto>, StatusCode> {
    match state.bench.result(&run_id).await {
        Some(result) => Ok(Json(result)),
        None => Err(StatusCode::NOT_FOUND),
    }
}
