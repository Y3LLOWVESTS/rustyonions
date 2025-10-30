//! RO:WHAT — JSON loader for `PolicyBundle` (strict).
//!
//! RO:INVARIANTS — `deny_unknown_fields` enforced by DTOs.

use crate::{errors::Error, model::PolicyBundle};

/// Parse a `PolicyBundle` from JSON bytes.
///
/// # Errors
///
/// Returns `Error::Parse` if the input is not valid JSON for `PolicyBundle`.
pub fn from_slice(bytes: &[u8]) -> Result<PolicyBundle, Error> {
    serde_json::from_slice::<PolicyBundle>(bytes).map_err(|e| Error::Parse(e.to_string()))
}
