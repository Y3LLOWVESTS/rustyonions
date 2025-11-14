//! Checkpoint / export helpers for audit chains.
//!
//! This is intentionally minimal for now; it can be extended later to
//! implement Merkle-style roots or chunked exports.

#[cfg(feature = "export")]
use crate::AuditRecord;

/// Simple checkpoint description for a contiguous span of records.
#[cfg(feature = "export")]
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// Inclusive start sequence number.
    pub from_seq: u64,
    /// Inclusive end sequence number.
    pub to_seq: u64,
    /// Hash of the last record in the span.
    pub head: String,
}

/// Compute a trivial checkpoint from a slice of records.
///
/// For now we just capture `[seq_min, seq_max]` and the `self_hash` of
/// the last record; this is enough to get tests going and can be
/// swapped for a more sophisticated construction later.
#[cfg(feature = "export")]
pub fn checkpoint_from_slice(records: &[AuditRecord]) -> Option<Checkpoint> {
    let last = records.last()?;
    let first_seq = records.first().map(|r| r.seq).unwrap_or(last.seq);
    Some(Checkpoint {
        from_seq: first_seq,
        to_seq: last.seq,
        head: last.self_hash.clone(),
    })
}
