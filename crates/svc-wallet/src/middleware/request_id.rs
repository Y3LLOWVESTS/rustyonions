//! RO:WHAT — Correlation/request id helper for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: DX/RES/GOV. Operators need bounded ids across logs/errors.
//! RO:INTERACTS — util::headers, routes error envelopes.
//! RO:INVARIANTS — visible ASCII only; max 128 bytes; never reads Authorization.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects control characters to reduce log injection risk.
//! RO:TEST — accepts_bounded_visible_id.

use crate::errors::{WalletError, WalletResult};

/// Validate and normalize a request/correlation id.
pub fn normalize_request_id(value: &str) -> WalletResult<String> {
    if value.is_empty() || value.len() > 128 {
        return Err(WalletError::bad_request(
            "request id must be 1..=128 visible bytes",
        ));
    }
    if !value.chars().all(|ch| ch.is_ascii_graphic()) {
        return Err(WalletError::bad_request(
            "request id contains unsupported characters",
        ));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_bounded_visible_id() {
        assert_eq!(normalize_request_id("abc-123").unwrap(), "abc-123");
        assert!(normalize_request_id("").is_err());
        assert!(normalize_request_id("bad id").is_err());
    }
}
