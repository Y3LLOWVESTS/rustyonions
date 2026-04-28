//! RO:WHAT — Deterministic chained accumulator for append-only entry records.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. The ledger needs a stable root function for replay equality and audit.
//! RO:INTERACTS — crate::types::EntryRecord, crate::engine::replay, crate::engine::ledger.
//! RO:INVARIANTS — same ordered records always produce the same root; hash input must exclude self-referential new_root bytes.
//! RO:METRICS — none directly.
//! RO:CONFIG — LedgerConfig::accumulator_kind chooses the algorithm seam, currently mapped to the same deterministic path.
//! RO:SECURITY — root is integrity-only; no secret material or signatures participate here.
//! RO:TEST — replay_recovery.rs and idempotency_prop.rs prove determinism.

use crate::{
    config::AccumulatorKind,
    error::LedgerError,
    types::{EntryRecord, Root},
};
use serde::Serialize;

/// Canonical hash input for a record.
///
/// This intentionally excludes `new_root`, because `new_root` is the value being
/// derived. Including it makes the accumulator self-referential and breaks replay:
/// live commit hashes a record before `new_root` is filled in, while replay sees a
/// stored record with `new_root` already populated.
#[derive(Debug, Serialize)]
struct CanonicalRecordForHash<'a> {
    seq: u64,
    entry: &'a crate::types::Entry,
    prev_root_hex: String,
}

/// Compute the next root for a record.
pub fn next_root(
    kind: AccumulatorKind,
    prev_root: Root,
    record: &EntryRecord,
) -> Result<Root, LedgerError> {
    let canonical = CanonicalRecordForHash {
        seq: record.seq.get(),
        entry: &record.entry,
        prev_root_hex: prev_root.to_hex(),
    };

    let encoded = match kind {
        AccumulatorKind::Merkle | AccumulatorKind::Verkle => serde_json::to_vec(&canonical)?,
    };

    let mut hasher = blake3::Hasher::new();
    hasher.update(prev_root.as_bytes());
    hasher.update(&encoded);
    Ok(Root::from_bytes(*hasher.finalize().as_bytes()))
}
