//! GET /resolve/:key
//! RO:WHY  Return 404 for well-formed misses (manifest:null AND providers:[]).
//! RO:INVARIANTS Keep the JSON body identical; only the HTTP status changes.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::Value;

use crate::{pipeline, AppState, error::SvcError};

pub async fn resolve(
    Path(key): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, SvcError> {
    let out = pipeline::resolve::run(state, &key, false).await?;

    // miss := manifest == null && providers == []
    let miss = match serde_json::to_value(&out) {
        Ok(Value::Object(map)) => {
            let manifest_is_null = map.get("manifest").is_some_and(|m| m.is_null());
            let providers_empty = map
                .get("providers")
                .and_then(|p| p.as_array())
                .is_none_or(|arr| arr.is_empty());
            manifest_is_null && providers_empty
        }
        _ => false,
    };

    if miss {
        return Ok((StatusCode::NOT_FOUND, Json(out)).into_response());
    }

    Ok(Json(out).into_response())
}
