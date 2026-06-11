//! RO:WHAT — QuickChain Phase 0 identifier shape validators.
//! RO:WHY — ECON/RES: operation, hold, and retry IDs must be unambiguous before replay/root work.
//! RO:INTERACTS — operation/hold/vector DTOs and future ledger receipt references.
//! RO:INVARIANTS — validation only; no uniqueness DB; no authority; no persistence; no ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — idempotency keys are bounded visible ASCII and must not carry secrets.
//! RO:TEST — tests/quickchain_ids_and_money.rs.

use super::{QuickChainResult, QuickChainValidationError};

pub const QUICKCHAIN_OPERATION_ID_PREFIX: &str = "op_";
pub const QUICKCHAIN_HOLD_ID_PREFIX: &str = "hold_";
pub const QUICKCHAIN_ID_HEX_LEN: usize = 32;
pub const MAX_QUICKCHAIN_IDEMPOTENCY_KEY_BYTES: usize = 128;

/// Validate `operation_id = "op_<32 lowercase hex>"`.
pub fn validate_operation_id_v1(field: &'static str, value: &str) -> QuickChainResult<()> {
    validate_prefixed_lower_hex_id(field, value, QUICKCHAIN_OPERATION_ID_PREFIX)
}

/// Validate `hold_id = "hold_<32 lowercase hex>"`.
pub fn validate_hold_id_v1(field: &'static str, value: &str) -> QuickChainResult<()> {
    validate_prefixed_lower_hex_id(field, value, QUICKCHAIN_HOLD_ID_PREFIX)
}

/// Validate a retry idempotency key.
///
/// The key is a retry/dedupe hint only. It is not authority and must not be
/// treated as a secret, signature, account id, or operation id.
pub fn validate_idempotency_key_v1(field: &'static str, value: &str) -> QuickChainResult<()> {
    if value.is_empty() {
        return Err(QuickChainValidationError::EmptyField { field });
    }

    if value.len() > MAX_QUICKCHAIN_IDEMPOTENCY_KEY_BYTES {
        return Err(QuickChainValidationError::FieldTooLong {
            field,
            max: MAX_QUICKCHAIN_IDEMPOTENCY_KEY_BYTES,
            actual: value.len(),
        });
    }

    if !value.bytes().all(|byte| matches!(byte, 0x21..=0x7e)) {
        return Err(QuickChainValidationError::InvalidToken { field });
    }

    Ok(())
}

fn validate_prefixed_lower_hex_id(
    field: &'static str,
    value: &str,
    prefix: &str,
) -> QuickChainResult<()> {
    if !value.starts_with(prefix) {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "must have the required QuickChain prefix",
        });
    }

    let hex = &value[prefix.len()..];
    if hex.len() != QUICKCHAIN_ID_HEX_LEN {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "must contain exactly 32 lowercase hex characters after prefix",
        });
    }

    if !hex
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return Err(QuickChainValidationError::InvalidToken { field });
    }

    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(QuickChainValidationError::InvalidToken { field });
    }

    Ok(())
}
