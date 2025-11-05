// crates/micronode/src/http/kv.rs
//! RO:WHAT — /kv API (PUT/GET/DELETE /kv/{bucket}/{key})
//! RO:INVARIANTS — Apply ingress guards in app.rs (decode→cap→concurrency).
//! RO:OBS — Content-Type is octet-stream for GET; PUT echoes no body.

use crate::state::AppState;
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};

// PUT /kv/{bucket}/{key}
pub async fn put_kv(
    State(st): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
    body: Bytes,
) -> impl IntoResponse {
    match st.storage.put(&bucket, &key, &body) {
        Ok(()) => StatusCode::CREATED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// GET /kv/{bucket}/{key}
pub async fn get_kv(
    State(st): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
) -> Response {
    match st.storage.get(&bucket, &key) {
        Ok(Some(bytes)) => {
            let mut resp = (StatusCode::OK, bytes).into_response();
            resp.headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
            resp
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

// DELETE /kv/{bucket}/{key}
pub async fn del_kv(
    State(st): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
) -> impl IntoResponse {
    match st.storage.del(&bucket, &key) {
        Ok(true) => StatusCode::NO_CONTENT,
        Ok(false) => StatusCode::NOT_FOUND,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
