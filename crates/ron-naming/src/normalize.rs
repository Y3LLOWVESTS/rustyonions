//! RO:WHAT — Unicode/IDNA normalization to canonical ASCII FQDNs.
//! RO:WHY  — Interop & safety: enforce UTS-46/IDNA processing and local hygiene.
//! RO:INTERACTS — types::Fqdn
//! RO:INVARIANTS — Lowercase; NFC; IDNA ASCII (Punycode) with trailing dot stripped; collapse consecutive dots.
//! RO:TEST — tests/normalize_idempotence.rs; examples/normalize_roundtrip.rs

use idna::domain_to_ascii;
use once_cell::sync::Lazy;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;

use crate::types::Fqdn;

/// Normalized ASCII FQDN (newtype wrapper for type-safety).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedFqdn(pub Fqdn);

static DOTS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\.+").expect("regex"));

/// Normalize an input domain (Unicode or ASCII) into canonical ASCII FQDN.
///
/// Steps:
/// 1. Trim whitespace; strip any leading/trailing dots.
/// 2. Unicode NFC normalize.
/// 3. Collapse consecutive dots to a single dot.
/// 4. Apply UTS-46 / IDNA to ASCII (punycode).
/// 5. Lowercase; validate ASCII FQDN hygiene.
pub fn normalize_fqdn_ascii(input: &str) -> Result<NormalizedFqdn, NormalizeError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(NormalizeError::Empty);
    }
    let nfc = trimmed.nfc().collect::<String>();
    let no_edges = nfc.trim_matches('.');
    let collapsed = DOTS.replace_all(no_edges, ".").into_owned();
    let ascii = domain_to_ascii(&collapsed).map_err(|_| NormalizeError::Idna)?;
    let lower = ascii.to_ascii_lowercase();
    let fqdn = Fqdn(lower);
    if !fqdn.is_valid() {
        return Err(NormalizeError::InvalidAscii);
    }
    Ok(NormalizedFqdn(fqdn))
}

/// Normalization errors.
#[derive(thiserror::Error, Debug)]
pub enum NormalizeError {
    /// Empty input.
    #[error("empty input")]
    Empty,
    /// IDNA/UTS-46 mapping failed.
    #[error("invalid domain (IDNA)")]
    Idna,
    /// Resulting ASCII FQDN failed hygiene checks.
    #[error("invalid ascii fqdn")]
    InvalidAscii,
}
