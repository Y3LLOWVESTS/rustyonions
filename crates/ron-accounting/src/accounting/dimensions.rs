//! RO:WHAT — Defines stable accounting dimensions for bytes, requests, and CPU units.
//! RO:WHY — Pillar 12; Concerns: ECON/DX. Shared dimension names prevent wire drift.
//! RO:INTERACTS — recorder keys, slice IDs, exporter stream keys, rewarder policies.
//! RO:INVARIANTS — integer-only units; enum is non_exhaustive for future dimensions.
//! RO:METRICS — dimension label values derive from as_str().
//! RO:CONFIG — no config.
//! RO:SECURITY — no secrets.
//! RO:TEST — unit: recording_tests; public API snapshots.

use serde::{Deserialize, Serialize};

/// Canonical bytes dimension label.
pub const BYTES: &str = "bytes";

/// Canonical requests dimension label.
pub const REQUESTS: &str = "requests";

/// Canonical CPU unit dimension label.
pub const CPU_UNITS: &str = "cpu_units";

/// Metered usage dimensions supported by Batch 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    /// Byte counters, usually bytes stored or served.
    Bytes,
    /// Request counters, usually admitted operations.
    Requests,
    /// Abstract CPU or work units.
    Cpu,
}

impl Dimension {
    /// Stable string label for metrics and canonical slice encoding.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bytes => BYTES,
            Self::Requests => REQUESTS,
            Self::Cpu => CPU_UNITS,
        }
    }
}
