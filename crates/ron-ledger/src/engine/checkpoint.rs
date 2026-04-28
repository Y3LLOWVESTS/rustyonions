//! RO:WHAT — Checkpoint helper for producing durable `(seq, root, ts)` snapshots from the commit path.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. Checkpoints bound replay work and anchor deterministic recovery.
//! RO:INTERACTS — crate::types::CheckpointRecord, crate::engine::storage, crate::engine::ledger.
//! RO:INVARIANTS — checkpoint cadence is config-driven; checkpoint data is append-only and never rewrites history.
//! RO:METRICS — future wrappers can turn checkpoint events into counters/latency metrics through observers.
//! RO:CONFIG — LedgerConfig::checkpoint_interval.
//! RO:SECURITY — checkpoint records contain no secrets.
//! RO:TEST — replay_recovery.rs ensures checkpoint + WAL replay yields the same root.

use crate::types::{CheckpointRecord, Root, Seq};

/// Build a checkpoint record.
pub fn build_checkpoint(seq: Seq, root: Root, ts: u64) -> CheckpointRecord {
    CheckpointRecord { seq, root, ts }
}
