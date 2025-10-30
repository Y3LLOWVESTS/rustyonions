//! GET /providers/:cid
//! RO:WHY  Return 404 when no providers are found. Preserve body shape.
//! RO:INVARIANTS Only status code changes on miss; JSON fields unchanged.

use crate::{error::SvcError, pipeline, AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::collections::HashMap;
use std::sync::Arc;

pub async fn providers(
    Path(cid): Path<String>,
    Query(q): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, SvcError> {
    let limit = q
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(5);

    let out = pipeline::providers::run(state, &cid, limit).await?;

    if out.providers.is_empty() {
        return Ok((StatusCode::NOT_FOUND, Json(out)).into_response());
    }

    Ok(Json(out).into_response())
}
