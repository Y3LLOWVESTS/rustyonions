//! RO:WHAT — TOML loader for `PolicyBundle` (strict).
//!
//! # Errors
//!
//! Returns `Error::Parse` if TOML is malformed or not UTF-8.

use crate::{errors::Error, model::PolicyBundle};

pub fn from_slice(bytes: &[u8]) -> Result<PolicyBundle, Error> {
    let s = std::str::from_utf8(bytes).map_err(|e| Error::Parse(e.to_string()))?;
    toml::from_str::<PolicyBundle>(s).map_err(|e| Error::Parse(e.to_string()))
}
