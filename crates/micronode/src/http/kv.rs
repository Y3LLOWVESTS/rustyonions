//! RO:WHAT — KV v1 HTTP handlers: PUT/GET/DELETE /kv/{bucket}/{key}.
//! RO:WHY  — Provide a minimal key/value API for Micronode, backed by Storage.
//! RO:INTERACTS — state::AppState (storage), storage::Storage, layers (body cap, decode guard).
//! RO:INVARIANTS — Binary-safe; no JSON; caps/enforced by layers; no auth yet.
//! RO:METRICS — HTTP metrics handled by global middleware; KV domain metrics later.
//! RO:CONFIG — No per-route config yet; concurrency/body caps are applied in app.rs.
//! RO:SECURITY — No auth/policy here; must be added via middleware in a later step.
//! RO:TEST — To be covered by integration tests (kv_roundtrip, guard_behavior).

use crate::state::AppState;
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
};

/// PUT /kv/{bucket}/{key}
///
/// Body is treated as opaque bytes with `Content-Type: application/octet-stream`.
/// Returns 201 on create, 204 on update, 500 on internal errors.
pub async fn put_kv(
    State(st): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
    body: Bytes,
) -> impl IntoResponse {
    match st.storage.put(&bucket, &key, &body) {
        Ok(true) => StatusCode::CREATED,
        Ok(false) => StatusCode::NO_CONTENT,
        Err(_e) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// GET /kv/{bucket}/{key}
///
/// Returns 200 with raw bytes and `Content-Type: application/octet-stream`
/// or 404 if the key is absent.
pub async fn get_kv(
    State(st): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
) -> impl IntoResponse {
    match st.storage.get(&bucket, &key) {
        Ok(Some(bytes)) => {
            (StatusCode::OK, [(header::CONTENT_TYPE, "application/octet-stream")], bytes)
                .into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_e) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// DELETE /kv/{bucket}/{key}
///
/// Returns 204 if a value was deleted, 404 if it did not exist.
pub async fn delete_kv(
    State(st): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
) -> impl IntoResponse {
    match st.storage.delete(&bucket, &key) {
        Ok(true) => StatusCode::NO_CONTENT,
        Ok(false) => StatusCode::NOT_FOUND,
        Err(_e) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
