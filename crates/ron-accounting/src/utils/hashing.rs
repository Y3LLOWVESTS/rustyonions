//! RO:WHAT — BLAKE3 digest helpers for b3-prefixed slice commitments and chaining.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC. Sealed snapshots need stable integrity IDs.
//! RO:INTERACTS — accounting::slice, WAL replay, interop vectors, svc-rewarder inputs.
//! RO:INVARIANTS — full BLAKE3-256 digest; canonical textual form is b3:<64 hex>.
//! RO:METRICS — callers report digest/schema failures.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — verifies full digest strings; never accepts truncated identifiers.
//! RO:TEST — unit: recording_tests; prop: encoding_prop.

use crate::errors::{Error, Result};

/// Canonical digest prefix used across RustyOnions.
pub const B3_PREFIX: &str = "b3:";

/// Compute a raw BLAKE3-256 digest.
pub fn b3_digest(bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(bytes).as_bytes()
}

/// Compute a canonical `b3:<hex>` digest string.
pub fn b3_hex(bytes: &[u8]) -> String {
    format!("{}{}", B3_PREFIX, blake3::hash(bytes).to_hex())
}

/// Compute a simple chained digest over an optional previous digest and a payload.
pub fn chained_b3_hex(previous_b3: Option<&str>, payload: &[u8]) -> Result<String> {
    let mut hasher = blake3::Hasher::new();
    if let Some(prev) = previous_b3 {
        hasher.update(&parse_b3(prev)?);
    }
    hasher.update(payload);
    Ok(format!("{}{}", B3_PREFIX, hasher.finalize().to_hex()))
}

/// Parse a canonical `b3:<64 hex>` string into raw bytes.
pub fn parse_b3(value: &str) -> Result<[u8; 32]> {
    let hex = value
        .strip_prefix(B3_PREFIX)
        .ok_or_else(|| Error::schema("digest must start with b3:"))?;
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::schema("digest must be 64 hex characters"));
    }

    let decoded = hex::decode(hex).map_err(|err| Error::schema(err.to_string()))?;
    let mut out = [0_u8; 32];
    out.copy_from_slice(&decoded);
    Ok(out)
}
