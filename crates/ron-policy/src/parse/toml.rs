//! RO:WHAT — TOML loader for `PolicyBundle` (strict).

use crate::{errors::Error, model::PolicyBundle};

/// Parse a `PolicyBundle` from TOML bytes.
///
/// # Errors
///
/// Returns `Error::Parse` if the input is not valid UTF-8 or not valid TOML for `PolicyBundle`.
pub fn from_slice(bytes: &[u8]) -> Result<PolicyBundle, Error> {
    let s = std::str::from_utf8(bytes).map_err(|e| Error::Parse(e.to_string()))?;
    toml::from_str::<PolicyBundle>(s).map_err(|e| Error::Parse(e.to_string()))
}
