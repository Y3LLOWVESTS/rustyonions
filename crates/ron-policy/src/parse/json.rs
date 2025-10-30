//! RO:WHAT — JSON loader for `PolicyBundle` (strict).
//!
//! RO:INVARIANTS — `deny_unknown_fields` enforced by DTOs.
//!
//! # Errors
//!
//! Returns `Error::Parse` if JSON is malformed.

use crate::{errors::Error, model::PolicyBundle};

pub fn from_slice(bytes: &[u8]) -> Result<PolicyBundle, Error> {
    serde_json::from_slice::<PolicyBundle>(bytes).map_err(|e| Error::Parse(e.to_string()))
}
