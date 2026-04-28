//! RO:WHAT — Truthful readiness state for ron-accounting hosts and adapters.
//! RO:WHY — Pillar 12; Concerns: RES/PERF/GOV. Readiness must degrade under unsafe conditions.
//! RO:INTERACTS — config loader, exporter router, WAL, boundary ticker, future HTTP /readyz.
//! RO:INVARIANTS — all required gates true before ready; snapshot is lock-short and deterministic.
//! RO:METRICS — callers can mirror missing keys into readiness gauges.
//! RO:CONFIG — gates reflect config/exporter/WAL/ticker state.
//! RO:SECURITY — no secrets.
//! RO:TEST — unit/integration readiness tests in later batches.

use std::collections::BTreeMap;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Readiness gates for ron-accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum ReadyKey {
    /// Configuration has loaded and passed validation.
    ConfigLoaded,
    /// Internal queues and row caps are within bounds.
    QueuesBoundedOk,
    /// Exporter is available or intentionally disabled.
    ExporterOk,
    /// WAL is available or intentionally disabled by amnesia/config.
    WalOk,
    /// Boundary ticker/manual rollover path is alive.
    BoundaryTickerOk,
}

impl ReadyKey {
    /// Stable label value for metrics and readiness bodies.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfigLoaded => "config_loaded",
            Self::QueuesBoundedOk => "queues_bounded_ok",
            Self::ExporterOk => "exporter_ok",
            Self::WalOk => "wal_ok",
            Self::BoundaryTickerOk => "boundary_ticker_ok",
        }
    }
}

const ALL_KEYS: [ReadyKey; 5] = [
    ReadyKey::ConfigLoaded,
    ReadyKey::QueuesBoundedOk,
    ReadyKey::ExporterOk,
    ReadyKey::WalOk,
    ReadyKey::BoundaryTickerOk,
];

/// Shared readiness gate state.
#[derive(Debug, Default)]
pub struct Readiness {
    gates: RwLock<BTreeMap<ReadyKey, bool>>,
}

impl Readiness {
    /// Create a readiness state with all gates initially false.
    pub fn new() -> Self {
        let mut gates = BTreeMap::new();
        for key in ALL_KEYS {
            gates.insert(key, false);
        }
        Self {
            gates: RwLock::new(gates),
        }
    }

    /// Set a readiness key.
    pub fn set(&self, key: ReadyKey, ok: bool) {
        self.gates.write().insert(key, ok);
    }

    /// Mark every readiness key as ready; useful for embedded tests/examples.
    pub fn mark_all_ready(&self) {
        let mut gates = self.gates.write();
        for key in ALL_KEYS {
            gates.insert(key, true);
        }
    }

    /// Return `(ready, missing_keys)`.
    pub fn snapshot(&self) -> (bool, Vec<ReadyKey>) {
        let gates = self.gates.read();
        let missing: Vec<_> = ALL_KEYS
            .iter()
            .copied()
            .filter(|key| !gates.get(key).copied().unwrap_or(false))
            .collect();
        (missing.is_empty(), missing)
    }
}
