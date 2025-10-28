//! RO:WHAT — Optional verification helpers for test vectors / attestations.
//! RO:WHY  — Keep checks in-library for CI without introducing runtime owners.
//! RO:INVARIANTS — No network or disk I/O beyond caller-provided bytes.

use blake3::Hasher;

/// Compute a BLAKE3-256 hex for provided bytes (lowercase).
pub fn blake3_hex(bytes: &[u8]) -> String {
    let mut h = Hasher::new();
    h.update(bytes);
    h.finalize().to_hex().to_string()
}
