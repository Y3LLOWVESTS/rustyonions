//! RO:WHAT — Small validation helpers for account IDs, assets, idempotency keys, and memos.
//! RO:WHY  — Pillar 12; Concerns: SEC/DX/ECON. Reject malformed identifiers before policy or ledger calls.
//! RO:INTERACTS — dto::requests, auth::caps, policy::enforce.
//! RO:INVARIANTS — bounded strings; stable character sets; no floats; no secret-bearing fields.
//! RO:METRICS — callers map errors into wallet_rejects_total.
//! RO:CONFIG — asset validation reads WalletConfig.
//! RO:SECURITY — reduces log/header injection risk by rejecting control characters.
//! RO:TEST — validates_allowed_account_chars; rejects_bad_idempotency_key.

use crate::{
    config::WalletConfig,
    errors::{WalletError, WalletResult},
};

/// Validate account identifier grammar shared with ron-ledger.
pub fn validate_account_id(value: &str) -> WalletResult<()> {
    if value.is_empty() || value.len() > 256 {
        return Err(WalletError::bad_request("account id must be 1..=256 bytes"));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '/' | '-' | '_'))
    {
        return Err(WalletError::bad_request(
            "account id contains unsupported characters",
        ));
    }
    Ok(())
}

/// Validate asset against current v1 config.
pub fn validate_asset(value: &str, cfg: &WalletConfig) -> WalletResult<()> {
    if value != cfg.asset {
        return Err(WalletError::bad_request(
            "unsupported asset for this wallet",
        ));
    }
    Ok(())
}

/// Validate idempotency key from header or body.
pub fn validate_idempotency_key(value: &str) -> WalletResult<()> {
    if value.is_empty() || value.len() > 64 {
        return Err(WalletError::bad_request(
            "Idempotency-Key must be 1..=64 bytes",
        ));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.'))
    {
        return Err(WalletError::bad_request(
            "Idempotency-Key contains unsupported characters",
        ));
    }
    Ok(())
}

/// Validate optional memo.
pub fn validate_memo(value: Option<&str>) -> WalletResult<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.len() > 256 {
        return Err(WalletError::bad_request("memo must be <=256 bytes"));
    }
    if value.chars().any(char::is_control) {
        return Err(WalletError::bad_request(
            "memo must not contain control chars",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_allowed_account_chars() {
        validate_account_id("t:demo/u:alice/w").unwrap();
    }

    #[test]
    fn rejects_bad_idempotency_key() {
        assert!(validate_idempotency_key("bad space").is_err());
    }
}
