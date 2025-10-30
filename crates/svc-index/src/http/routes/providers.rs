//! GET /providers/:cid
//! RO:WHY  Return 404 when no real providers exist. If a synthetic "local://stub"
//!         is present as the only entry, treat as empty and 404. Preserve body shape.
//! RO:INVARIANTS Keep field names as-is; only status changes on miss.

use crate::{error::SvcError, pipeline, AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};
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

    // Get the pipeline output in its native type.
    let out = pipeline::providers::run(state, &cid, limit).await?;

    // Convert to JSON so we can (a) detect emptiness (b) strip a synthetic stub safely
    // without depending on concrete DTO types.
    let mut val: Value = serde_json::to_value(&out)
        .map_err(|e| SvcError::Internal(anyhow::anyhow!("serialize providers: {e}")))?;

    // Locate providers array (mut), if present.
    let providers_arr_opt = ["providers"]
        .iter()
        .try_fold(&mut val, |cur, key| cur.get_mut(*key).ok_or(()))
        .ok()
        .and_then(|v| v.as_array_mut());

    let no_real_providers = if let Some(arr) = providers_arr_opt {
        // If the only entry is a synthetic stub, clear it.
        if arr.len() == 1 {
            let is_stub = arr[0]
                .get("id")
                .and_then(|id| id.as_str())
                .map(|s| s == "local://stub")
                .unwrap_or(false);
        if is_stub {
                arr.clear();
            }
        }
        arr.is_empty()
    } else {
        // No providers field? Normalize to empty array to preserve shape.
        if let Value::Object(map) = &mut val {
            map.insert("providers".into(), json!([]));
        }
        true
    };

    if no_real_providers {
        return Ok((StatusCode::NOT_FOUND, Json(val)).into_response());
    }

    Ok(Json(val).into_response())
}
