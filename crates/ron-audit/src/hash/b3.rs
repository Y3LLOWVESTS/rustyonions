//! BLAKE3 hashing for canonical audit records.

use blake3::Hasher;

use crate::canon::{canonicalize_without_self_hash, CanonError};
use crate::dto::DedupeKey;
use crate::AuditRecord;

/// Compute the canonical BLAKE3 hash of a record, excluding `self_hash`,
/// and return it in the `"b3:<hex>"` string form used on-chain.
pub fn b3_no_self(rec: &AuditRecord) -> Result<String, CanonError> {
    let bytes = canonicalize_without_self_hash(rec)?;
    let mut hasher = Hasher::new();
    hasher.update(&bytes);
    let hash = hasher.finalize();
    Ok(format!("b3:{}", hash.to_hex()))
}

/// Compute the canonical BLAKE3 hash of a record, excluding `self_hash`,
/// and return the raw 32-byte output as a dedupe key.
pub fn dedupe_key(rec: &AuditRecord) -> Result<DedupeKey, CanonError> {
    let bytes = canonicalize_without_self_hash(rec)?;
    let mut hasher = Hasher::new();
    hasher.update(&bytes);
    let hash = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    Ok(out)
}
