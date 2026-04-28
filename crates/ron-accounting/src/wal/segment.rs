//! RO:WHAT — WAL segment record helpers for length-delimited sealed slices.
//! RO:WHY — Pillar 12; Concerns: RES/ECON. Segment records need cheap corruption detection.
//! RO:INTERACTS — wal::replay, utils::hashing, SealedSlice encoding.
//! RO:INVARIANTS — checksum covers record bytes; replay never trusts corrupt records.
//! RO:METRICS — future replay increments checksum failure counters.
//! RO:CONFIG — fsync policy from WalConfig in later batch.
//! RO:SECURITY — no secrets; integrity only.
//! RO:TEST — wal_tests feature lane.

use serde::{Deserialize, Serialize};

use crate::utils::hashing::b3_hex;

/// A simple in-memory segment record used by Batch 1 tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentRecord {
    /// Encoded payload bytes.
    pub bytes: Vec<u8>,
    /// b3 checksum over bytes.
    pub checksum_b3: String,
}

impl SegmentRecord {
    /// Construct a checksummed record.
    pub fn new(bytes: Vec<u8>) -> Self {
        let checksum_b3 = b3_hex(&bytes);
        Self { bytes, checksum_b3 }
    }

    /// Return true when checksum still matches bytes.
    pub fn verify(&self) -> bool {
        b3_hex(&self.bytes) == self.checksum_b3
    }
}
