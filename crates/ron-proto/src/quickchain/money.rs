//! RO:WHAT — QuickChain integer minor-unit money validation helpers.
//! RO:WHY — ECON/GOV: money must remain deterministic string data before any roots or settlement exist.
//! RO:INTERACTS — quickchain DTO validators, future svc-wallet/ron-ledger receipt DTOs.
//! RO:INVARIANTS — strings only; no floats; no signs; no separators; no arithmetic side effects.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects ambiguous money encodings that could fork canonical bytes.
//! RO:TEST — tests/quickchain_ids_and_money.rs.

use super::{QuickChainResult, QuickChainValidationError};

/// Maximum decimal digits accepted for a QuickChain minor-unit amount.
///
/// This leaves room for `u128::MAX` while rejecting intentionally huge strings
/// before they reach future canonicalization or service boundaries.
pub const MAX_QUICKCHAIN_MINOR_UNITS_DIGITS: usize = 39;

/// Validate a canonical non-negative integer minor-unit string.
///
/// Accepted examples: `"0"`, `"1"`, `"1000000"`.
/// Rejected examples: `""`, `"01"`, `"+1"`, `"-1"`, `"1.0"`,
/// `"1_000"`, and `"1 ROC"`.
pub fn validate_quickchain_minor_units(field: &'static str, value: &str) -> QuickChainResult<()> {
    if value.is_empty() {
        return Err(QuickChainValidationError::InvalidMoney {
            field,
            reason: "must not be empty",
        });
    }

    if value.len() > MAX_QUICKCHAIN_MINOR_UNITS_DIGITS {
        return Err(QuickChainValidationError::InvalidMoney {
            field,
            reason: "must not exceed u128 decimal width",
        });
    }

    if value.len() > 1 && value.starts_with('0') {
        return Err(QuickChainValidationError::InvalidMoney {
            field,
            reason: "must be canonical decimal without leading zeroes",
        });
    }

    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(QuickChainValidationError::InvalidMoney {
            field,
            reason: "must contain decimal digits only",
        });
    }

    value
        .parse::<u128>()
        .map_err(|_| QuickChainValidationError::InvalidMoney {
            field,
            reason: "must fit in u128 minor units",
        })?;

    Ok(())
}
