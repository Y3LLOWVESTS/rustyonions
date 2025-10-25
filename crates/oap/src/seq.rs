//! RO:WHAT — Simple, allocator-free sequence/correlation id generator.
//! RO:WHY  — Provide monotonic ids for `corr_id` without globals/unsafe.
//! RO:INTERACTS — Used by clients/servers to stamp `Header::corr_id`.
//! RO:INVARIANTS — Monotonic per-instance; seed adds time entropy to avoid collisions.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Monotonic sequence generator (per-instance).
#[derive(Debug)]
pub struct Seq {
    counter: AtomicU64,
}

impl Default for Seq {
    fn default() -> Self {
        // Seed with current nanos (truncated) to diversify instances.
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| (d.as_nanos() as u64) ^ 0xA5A5_5A5A_D3C3_3C3D)
            .unwrap_or(0xD00D_F00D_C0FF_EE00);
        Self { counter: AtomicU64::new(seed) }
    }
}

impl Seq {
    pub fn new() -> Self { Self::default() }

    /// Fetch-next correlation id.
    #[inline]
    pub fn next(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
    }
}
