//! Admin endpoints (MVP): reindex/pin stubs + name→CID seeding.
//! Adds: X-Admin-Token guard, strict b3 validator, name: normalization.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{error::SvcError, AppState};

pub async fn reindex() -> impl IntoResponse {
    (StatusCode::ACCEPTED, "queued")
}

pub async fn pin() -> impl IntoResponse {
    (StatusCode::ACCEPTED, "queued")
}

#[derive(Deserialize)]
pub struct SeedBody {
    /// Name key. If missing "name:" prefix, it will be added.
    pub name: String,
    /// Content ID (BLAKE3) of the manifest to associate.
    pub cid: String,
}

#[inline]
fn is_b3(s: &str) -> bool {
    let s = s.strip_prefix("b3:").unwrap_or("");
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9'|b'a'..=b'f'))
}

/// PUT /admin/seed { "name": "hello", "cid": "b3:<64hex>" }
///
/// - requires header X-Admin-Token equal to env INDEX_ADMIN_TOKEN
/// - normalizes name to "name:<value>" if not already prefixed
/// - validates CID as b3:<64hex>
/// - stores mapping so /resolve/name:<value> works
pub async fn seed(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SeedBody>,
) -> Result<impl IntoResponse, SvcError> {
    // Token guard (simple pre-beta protection)
    let required = std::env::var("INDEX_ADMIN_TOKEN").unwrap_or_default();
    let provided = headers
        .get("X-Admin-Token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    if required.is_empty() || provided != required {
        return Err(SvcError::Unauthorized);
    }

    if !is_b3(&body.cid) {
        return Err(SvcError::BadRequest("invalid cid".into()));
    }

    let name = if body.name.starts_with("name:") {
        body.name
    } else {
        format!("name:{}", body.name)
    };

    state.store.put_manifest(&name, &body.cid);

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "ok": true, "name": name, "cid": body.cid })),
    ))
}
