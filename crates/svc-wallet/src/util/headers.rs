//! RO:WHAT — Header-name constants and small extraction helpers for wallet routes.
//! RO:WHY  — Pillar 12; Concerns: SEC/DX. Standardizes correlation and idempotency header behavior.
//! RO:INTERACTS — routes, middleware::request_id, dto::requests.
//! RO:INVARIANTS — idempotency key ≤64 bytes; X-Request-Id is bounded; Authorization is never logged here.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — does not expose or format bearer token values.
//! RO:TEST — bounded_header_value.

use crate::errors::{WalletError, WalletResult};

/// Standard idempotency header accepted by POST routes.
pub const IDEMPOTENCY_KEY: &str = "Idempotency-Key";
/// Alternate idempotency header reserved by WEB3.md.
pub const X_IDEMPOTENCY_KEY: &str = "X-Idempotency-Key";
/// Correlation/request id header.
pub const X_REQUEST_ID: &str = "X-Request-Id";
/// Correlation id header used by the API docs.
pub const X_CORR_ID: &str = "X-Corr-ID";

/// Validate a visible ASCII header value and return it as owned string.
pub fn bounded_header_value(value: &str, max_len: usize, name: &str) -> WalletResult<String> {
    if value.is_empty() || value.len() > max_len {
        return Err(WalletError::bad_request(format!(
            "{name} must be 1..={max_len} bytes"
        )));
    }
    if !value.chars().all(|c| c.is_ascii_graphic()) {
        return Err(WalletError::bad_request(format!(
            "{name} contains unsupported characters"
        )));
    }
    Ok(value.to_string())
}
