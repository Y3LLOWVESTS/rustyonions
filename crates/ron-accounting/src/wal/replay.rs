//! RO:WHAT — WAL replay helpers for filtering corrupt records.
//! RO:WHY — Pillar 12; Concerns: RES/ECON. Restart recovery must be deterministic and safe.
//! RO:INTERACTS — wal::segment, wal::Wal, exporter router.
//! RO:INVARIANTS — corrupt records are skipped, never partially trusted.
//! RO:METRICS — future replay reports skipped/corrupt counters.
//! RO:CONFIG — WAL quotas and age caps in later batch.
//! RO:SECURITY — no secrets.
//! RO:TEST — wal_tests feature lane.

use crate::wal::segment::SegmentRecord;

/// Return only records whose checksum validates.
pub fn verified_records(records: &[SegmentRecord]) -> Vec<SegmentRecord> {
    records
        .iter()
        .filter(|record| record.verify())
        .cloned()
        .collect()
}
