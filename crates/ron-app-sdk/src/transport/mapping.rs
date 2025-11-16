//! RO:WHAT — HTTP / reqwest → SdkError mapping helpers.
//! RO:WHY  — Keep `TransportHandle` focused on deadlines/backoff and
//!           centralize wire-to-taxonomy mapping here.
//! RO:INTERACTS — Used only from `transport::handle`; consults
//!                `crate::errors::SdkError` and reqwest error/status APIs.
//! RO:INVARIANTS — never panic; unknown statuses/errors become
//!                 `SdkError::Unknown` with human-readable message;
//!                 timeouts map to `Transport(std::io::ErrorKind::TimedOut)`.
//! RO:METRICS — none directly; higher layers may aggregate by error type.
//! RO:CONFIG — none (pure helpers).
//! RO:SECURITY — messages are safe for logs; no secrets included.
//! RO:TEST — local unit tests for representative mappings.

use std::{io::ErrorKind, time::Duration};

use crate::errors::SdkError;

/// Map an HTTP status code into the stable `SdkError` taxonomy.
///
/// This follows the mapping table in `ALL_DOCS.md` (error taxonomy
/// section) so that callers can rely on consistent semantics across
/// transports and services.
pub(crate) fn map_http_status(
    status: reqwest::StatusCode,
    retry_after: Option<Duration>,
) -> SdkError {
    use reqwest::StatusCode;

    match status {
        StatusCode::NOT_FOUND => SdkError::NotFound,
        StatusCode::CONFLICT => SdkError::Conflict,
        StatusCode::UNAUTHORIZED => SdkError::CapabilityExpired,
        StatusCode::FORBIDDEN => SdkError::CapabilityDenied,
        StatusCode::TOO_MANY_REQUESTS => SdkError::RateLimited { retry_after },
        s if s.is_server_error() => SdkError::Server(s.as_u16()),
        s => SdkError::Unknown(format!("unexpected http status: {s}")),
    }
}

/// Map reqwest errors into the stable `SdkError` taxonomy.
///
/// We deliberately distinguish transport-level timeouts and generic
/// I/O failures from "overall deadline exceeded", which is handled
/// explicitly in the retry wrapper.
pub(crate) fn map_reqwest_error(err: reqwest::Error) -> SdkError {
    if err.is_timeout() {
        return SdkError::Transport(ErrorKind::TimedOut);
    }

    // Heuristic TLS detection based on error text; the underlying
    // `reqwest`/`rustls` error types are not stable across versions.
    let msg = err.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("tls") || lower.contains("certificate") || lower.contains("ssl") {
        return SdkError::Tls;
    }

    if err.is_connect() || err.is_request() || err.is_body() {
        return SdkError::Transport(ErrorKind::Other);
    }

    SdkError::Unknown(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    #[test]
    fn http_status_mapping_basic() {
        assert!(matches!(
            map_http_status(StatusCode::NOT_FOUND, None),
            SdkError::NotFound
        ));
        assert!(matches!(
            map_http_status(StatusCode::CONFLICT, None),
            SdkError::Conflict
        ));
        assert!(matches!(
            map_http_status(StatusCode::TOO_MANY_REQUESTS, Some(Duration::from_secs(1))),
            SdkError::RateLimited { .. }
        ));
        assert!(matches!(
            map_http_status(StatusCode::INTERNAL_SERVER_ERROR, None),
            SdkError::Server(500)
        ));
    }
}
