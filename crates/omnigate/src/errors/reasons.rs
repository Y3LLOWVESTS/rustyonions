// RO:WHAT  Canonical reason codes used by http_map to produce JSON envelopes.
// RO:INVARS Codes are stable ASCII-UPPER_SNAKE where applicable.

use axum::http::StatusCode;

#[derive(Debug, Copy, Clone)]
pub enum Reason {
    // Common
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    PayloadTooLarge,
    UnsupportedMediaType,
    TooManyRequests,
    Internal,

    // Policy
    PolicyDeny,
    PolicyError,

    // New: for 411 responses when payload methods omit Content-Length
    LengthRequired,
}

impl Reason {
    pub fn status(self) -> StatusCode {
        match self {
            Reason::BadRequest => StatusCode::BAD_REQUEST,
            Reason::Unauthorized => StatusCode::UNAUTHORIZED,
            Reason::Forbidden => StatusCode::FORBIDDEN,
            Reason::NotFound => StatusCode::NOT_FOUND,
            Reason::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            Reason::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Reason::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Reason::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            Reason::Internal => StatusCode::INTERNAL_SERVER_ERROR,

            Reason::PolicyDeny => StatusCode::FORBIDDEN,
            Reason::PolicyError => StatusCode::SERVICE_UNAVAILABLE,

            Reason::LengthRequired => StatusCode::LENGTH_REQUIRED, // 411
        }
    }

    pub fn code_str(self) -> &'static str {
        match self {
            Reason::BadRequest => "BAD_REQUEST",
            Reason::Unauthorized => "UNAUTHORIZED",
            Reason::Forbidden => "FORBIDDEN",
            Reason::NotFound => "NOT_FOUND",
            Reason::MethodNotAllowed => "METHOD_NOT_ALLOWED",
            Reason::PayloadTooLarge => "PAYLOAD_TOO_LARGE",
            Reason::UnsupportedMediaType => "UNSUPPORTED_MEDIA_TYPE",
            Reason::TooManyRequests => "TOO_MANY_REQUESTS",
            Reason::Internal => "INTERNAL",

            Reason::PolicyDeny => "POLICY_DENY",
            Reason::PolicyError => "POLICY_ERROR",

            Reason::LengthRequired => "LENGTH_REQUIRED",
        }
    }

    /// Lowercase/snake-case reason code used in Problem JSON.
    ///
    /// This is what middleware_contract tests assert on, e.g.:
    ///   * 413 → "payload_too_large"
    ///   * 415 → "unsupported_media_type"
    ///   * 411 → "length_required"
    pub fn reason_str(self) -> &'static str {
        match self {
            Reason::BadRequest => "bad_request",
            Reason::Unauthorized => "unauthorized",
            Reason::Forbidden => "forbidden",
            Reason::NotFound => "not_found",
            Reason::MethodNotAllowed => "method_not_allowed",
            Reason::PayloadTooLarge => "payload_too_large",
            Reason::UnsupportedMediaType => "unsupported_media_type",
            Reason::TooManyRequests => "too_many_requests",
            Reason::Internal => "internal",

            Reason::PolicyDeny => "policy_deny",
            Reason::PolicyError => "policy_error",

            Reason::LengthRequired => "length_required",
        }
    }

    pub fn retryable(self) -> bool {
        matches!(
            self,
            Reason::TooManyRequests | Reason::PolicyError | Reason::Internal
        )
    }
}
