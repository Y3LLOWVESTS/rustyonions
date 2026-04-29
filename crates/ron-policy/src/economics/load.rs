//! RO:WHAT — TOML parsing entry points for ROC economics policy.
//!
//! RO:WHY — Pillar 12; Concerns: ECON/DX/GOV. Consumers need strict parse+validate helpers.
//!
//! RO:INTERACTS — `economics::types`, `economics::validate`, and `crate::errors`.
//!
//! RO:INVARIANTS — no file/network I/O; caller supplies bytes; DTOs deny unknown fields.
//!
//! RO:METRICS — none directly.
//!
//! RO:CONFIG — parses `configs/roc-economics.toml` style TOML.
//!
//! RO:SECURITY — human-safe errors only.
//!
//! RO:TEST — `economics_policy.rs`.

use crate::economics::types::EconomicsPolicy;
use crate::economics::validate;
use crate::errors::Error;

/// Parse and validate a ROC economics policy from TOML bytes.
///
/// # Errors
///
/// Returns `Error::Parse` for malformed UTF-8/TOML and `Error::Validation` for invariant
/// failures such as invalid splits or unsupported actions.
pub fn from_slice(bytes: &[u8]) -> Result<EconomicsPolicy, Error> {
    let raw = std::str::from_utf8(bytes).map_err(|err| Error::Parse(err.to_string()))?;
    from_str(raw)
}

/// Parse and validate a ROC economics policy from a TOML string.
///
/// # Errors
///
/// Returns `Error::Parse` for malformed TOML and `Error::Validation` for invariant failures.
pub fn from_str(raw: &str) -> Result<EconomicsPolicy, Error> {
    let policy =
        toml::from_str::<EconomicsPolicy>(raw).map_err(|err| Error::Parse(err.to_string()))?;
    validate::validate(&policy)?;
    Ok(policy)
}
