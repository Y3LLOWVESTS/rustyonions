//! RO:WHAT — Strict validators and parse errors for `ContentId`.
//! RO:WHY  — Keep hashing out of ron-proto; only parse/validate.
//! RO:INTERACTS — Used by ContentId serde and FromStr.
//! RO:INVARIANTS — 64 lowercase hex after "b3:" prefix.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseContentIdError {
    #[error("missing 'b3:' prefix")]
    MissingPrefix,
    #[error("hex length must be 64 characters")]
    BadLen,
    #[error("hex must be lowercase [0-9a-f]")]
    BadHex,
}

pub fn validate_b3_str(s: &str) -> Result<(), ParseContentIdError> {
    if !s.starts_with(super::CONTENT_ID_PREFIX) {
        return Err(ParseContentIdError::MissingPrefix);
    }
    let hex = &s[super::CONTENT_ID_PREFIX.len()..];
    if hex.len() != super::CONTENT_ID_HEX_LEN {
        return Err(ParseContentIdError::BadLen);
    }
    if !is_lower_hex64(hex) {
        return Err(ParseContentIdError::BadHex);
    }
    Ok(())
}

pub fn is_lower_hex64(hex: &str) -> bool {
    hex.bytes().all(|b| matches!(b, b'0'..=b'9'|b'a'..=b'f'))
}
