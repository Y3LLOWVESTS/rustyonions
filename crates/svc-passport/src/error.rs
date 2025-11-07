//! RO:WHAT — Error taxonomy (stable, machine-parseable) and IntoResponse mapping.
//! RO:WHY  — Deterministic client behavior, SDK-friendly.
//! RO:INVARIANTS — No secrets in messages; stable codes.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("malformed")]
    Malformed,
    #[error("verify_failed")]
    VerifyFailed,
    #[error("expired")]
    Expired,
    #[error("nbf")]
    NotBefore,
    #[error("bad_aud")]
    BadAudience,
    #[error("unknown_kid")]
    UnknownKid,
    #[error("scope_denied")]
    ScopeDenied,
    #[error("internal")]
    Internal(anyhow::Error),
}

#[derive(Serialize)]
pub struct Problem<'a> {
    code: &'a str,
    message: &'a str,
    retryable: bool,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (code, msg, status, retryable) = match &self {
            Error::Malformed => (
                "malformed",
                "bad request body",
                StatusCode::BAD_REQUEST,
                false,
            ),
            Error::VerifyFailed => (
                "verify_failed",
                "signature did not verify",
                StatusCode::UNAUTHORIZED,
                false,
            ),
            Error::Expired => ("expired", "token expired", StatusCode::UNAUTHORIZED, false),
            Error::NotBefore => ("nbf", "token not valid yet", StatusCode::UNAUTHORIZED, true),
            Error::BadAudience => ("bad_aud", "audience denied", StatusCode::FORBIDDEN, false),
            Error::UnknownKid => (
                "unknown_kid",
                "unknown verifying key",
                StatusCode::UNAUTHORIZED,
                true,
            ),
            Error::ScopeDenied => (
                "scope_denied",
                "capability denied",
                StatusCode::FORBIDDEN,
                false,
            ),
            Error::Internal(_) => (
                "internal",
                "internal error",
                StatusCode::INTERNAL_SERVER_ERROR,
                true,
            ),
        };
        let body = Json(Problem {
            code,
            message: msg,
            retryable,
        });
        (status, body).into_response()
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::Internal(e)
    }
}
