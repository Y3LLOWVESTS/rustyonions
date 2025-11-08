//! RO:WHAT  Centralized bounds helpers.
//! RO:WHY   Enforce size/complexity limits consistently.

/// Rough upper bound for Base64URL (no padding) length for a given raw size.
pub fn max_b64url_chars_for(max_bytes: usize) -> usize {
    // ceiling((max_bytes * 4) / 3) without padding; use saturating math for safety.
    max_bytes.saturating_mul(4).div_ceil(3)
}
