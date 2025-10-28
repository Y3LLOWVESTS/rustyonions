//! RO:WHAT — Cache facade (RAM default; sled optional)
//! RO:WHY — Micronode amnesia by default; Concerns: PERF/SEC
pub mod memory; // TODO phase 2
#[cfg(feature = "sled-cache")]
pub mod sled_cache; // TODO phase 2
