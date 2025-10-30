//! Service error type ↔ HTTP mapping.

use crate::types::ErrorResponse;
use axum::{http::StatusCode, response::IntoResponse, Json};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SvcError {
    #[error("not_found")]
    NotFound,
    #[error("bad_request: {0}")]
    BadRequest(String),
    #[error("over_capacity")]
    OverCapacity,
    #[error("upstream_unready")]
    UpstreamUnready,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("internal")]
    Internal(anyhow::Error),
}

impl IntoResponse for SvcError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, message): (StatusCode, &'static str, String) = match self {
            SvcError::NotFound => (StatusCode::NOT_FOUND, "not_found", "Not found".to_string()),
            SvcError::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m),
            SvcError::OverCapacity => (
                StatusCode::TOO_MANY_REQUESTS,
                "over_capacity",
                "Over capacity".to_string(),
            ),
            SvcError::UpstreamUnready => (
                StatusCode::SERVICE_UNAVAILABLE,
                "upstream_unready",
                "Upstream not ready".to_string(),
            ),
            SvcError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Unauthorized".to_string(),
            ),
            SvcError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "Forbidden".to_string()),
            SvcError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Internal error".to_string(),
            ),
        };
        (
            status,
            Json(ErrorResponse {
                code: code.into(),
                message,
            }),
        )
            .into_response()
    }
}
