//! Map StorageError -> HTTP responses.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

use crate::errors::StorageError;

pub fn into_response(err: StorageError) -> impl IntoResponse {
    let (status, code) = match err {
        StorageError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
        StorageError::BadAddress => (StatusCode::BAD_REQUEST, "bad_address"),
        StorageError::RangeNotSatisfiable => {
            (StatusCode::RANGE_NOT_SATISFIABLE, "range_not_satisfiable")
        }
        StorageError::CapacityExceeded => (StatusCode::PAYLOAD_TOO_LARGE, "capacity_exceeded"),
        StorageError::IntegrityFailed => (StatusCode::BAD_REQUEST, "integrity_failed"),
        StorageError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "io_error"),
    };

    (
        status,
        Json(json!({
            "title": code,
            "status": status.as_u16(),
            "detail": err.to_string()
        })),
    )
}
