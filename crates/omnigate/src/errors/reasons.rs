//! RO:WHAT — Canonical error reason codes for JSON envelopes.
//! RO:WHY  — Stable keys for clients, decoupled from HTTP status texts.

use axum::http::StatusCode;

/// Keep names kebab/underscore compatible — we expose snake_case in JSON.
#[derive(Clone, Copy, Debug)]
pub enum Reason {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    TooManyRequests,
    PayloadTooLarge,
    UnsupportedMediaType, // 415
    Internal,
    Unavailable,
}

impl Reason {
    /// Programmatic, stable key sent in the JSON envelope.
    pub const fn key(self) -> &'static str {
        match self {
            Reason::BadRequest => "bad_request",
            Reason::Unauthorized => "unauthorized",
            Reason::Forbidden => "forbidden",
            Reason::NotFound => "not_found",
            Reason::Conflict => "conflict",
            Reason::TooManyRequests => "too_many_requests",
            Reason::PayloadTooLarge => "payload_too_large",
            Reason::UnsupportedMediaType => "unsupported_media_type",
            Reason::Internal => "internal",
            Reason::Unavailable => "unavailable",
        }
    }

    /// Canonical HTTP mapping.
    pub const fn status(self) -> StatusCode {
        match self {
            Reason::BadRequest => StatusCode::BAD_REQUEST,
            Reason::Unauthorized => StatusCode::UNAUTHORIZED,
            Reason::Forbidden => StatusCode::FORBIDDEN,
            Reason::NotFound => StatusCode::NOT_FOUND,
            Reason::Conflict => StatusCode::CONFLICT,
            Reason::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            Reason::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Reason::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Reason::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Reason::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}
