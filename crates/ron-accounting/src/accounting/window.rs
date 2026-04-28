//! RO:WHAT — UTC-aligned fixed accounting window type.
//! RO:WHY — Pillar 12; Concerns: ECON/RES. Ensures all counters seal on deterministic boundaries.
//! RO:INTERACTS — recorder sealing, rollover, config validation, slice metadata.
//! RO:INVARIANTS — start inclusive; end exclusive; window length fixed 60s..3600s.
//! RO:METRICS — rollover callers count boundary ticks and sealed slices.
//! RO:CONFIG — accounting.window_len_s.
//! RO:SECURITY — no secrets.
//! RO:TEST — unit: rollover_tests.

use serde::{Deserialize, Serialize};

use crate::{
    errors::Result,
    utils::time::{aligned_window_end_ms, aligned_window_start_ms, validate_window_len_s},
};

/// A UTC-aligned fixed accounting window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Window {
    /// Inclusive start timestamp in Unix milliseconds.
    pub start_ms: u64,
    /// Exclusive end timestamp in Unix milliseconds.
    pub end_ms: u64,
    /// Window length in seconds.
    pub len_s: u32,
}

impl Window {
    /// Build the aligned window containing `timestamp_ms`.
    pub fn for_timestamp_ms(timestamp_ms: u64, len_s: u32) -> Result<Self> {
        validate_window_len_s(len_s)?;
        Ok(Self {
            start_ms: aligned_window_start_ms(timestamp_ms, len_s)?,
            end_ms: aligned_window_end_ms(timestamp_ms, len_s)?,
            len_s,
        })
    }

    /// Return the next adjacent fixed window.
    pub fn next(self) -> Self {
        let width_ms = u64::from(self.len_s) * 1_000;
        Self {
            start_ms: self.start_ms + width_ms,
            end_ms: self.end_ms + width_ms,
            len_s: self.len_s,
        }
    }

    /// Return true when `timestamp_ms` belongs to this window.
    pub fn contains(self, timestamp_ms: u64) -> bool {
        self.start_ms <= timestamp_ms && timestamp_ms < self.end_ms
    }
}
