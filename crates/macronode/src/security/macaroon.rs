//! RO:WHAT — Opaque capability-token wrapper for Macronode.
//! RO:WHY  — Macronode itself does not parse or verify macaroons/JWTs;
//!           it just treats them as opaque bearer tokens that downstream
//!           services (KMS/auth) can validate.
//! RO:INVARIANTS —
//!   - This module never logs token contents.
//!   - Parsing is intentionally minimal: higher layers decide semantics.

#![allow(dead_code)]

/// Opaque capability or macaroon-style token.
///
/// In this crate we treat the token as an opaque string. Verification and
/// interpretation belong to dedicated auth/KMS services.
#[derive(Debug, Clone)]
pub struct CapabilityToken {
    raw: String,
}

impl CapabilityToken {
    /// Construct a token from a raw bearer string (without the "Bearer " prefix).
    #[must_use]
    pub fn new<S: Into<String>>(raw: S) -> Self {
        Self { raw: raw.into() }
    }

    /// View the underlying token bytes.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

/// Parse a `Authorization` header value into a bearer token, if present.
///
/// This is intentionally tiny and does not validate the token format.
#[must_use]
pub fn parse_bearer_header(header: &str) -> Option<CapabilityToken> {
    // Common forms:
    //   "Bearer abc123"
    //   "bearer abc123"
    let trimmed = header.trim();
    let prefix_lower = "bearer ";

    if trimmed.len() <= prefix_lower.len() {
        return None;
    }

    if trimmed.to_ascii_lowercase().starts_with(prefix_lower) {
        let token = &trimmed[prefix_lower.len()..];
        if token.is_empty() {
            None
        } else {
            Some(CapabilityToken::new(token))
        }
    } else {
        None
    }
}
