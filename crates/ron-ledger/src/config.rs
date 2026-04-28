//! RO:WHAT — Typed ledger configuration for batching, limits, checkpoint cadence, engine profile, and PQ seam flags.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES/GOV. Keep runtime knobs explicit and validated without dragging in service config loaders.
//! RO:INTERACTS — crate::engine, crate::api validation, tests, examples.
//! RO:INVARIANTS — batch cap > 0; queue/checkpoint knobs non-zero; amnesia and persistent modes stay explicit.
//! RO:METRICS — none directly; service wrappers can surface config-derived gauges if desired.
//! RO:CONFIG — this file is the config contract.
//! RO:SECURITY — limits prevent pathological payloads; pq mode is only a seam, not custody.
//! RO:TEST — unit assertions via config validation and integration tests that hit size/conflict paths.

use crate::error::{LedgerError, RejectReason};

/// Storage profile for the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineMode {
    /// In-memory, no durable artifacts.
    Amnesia,
    /// File-backed / durable storage.
    Persistent,
}

impl Default for EngineMode {
    fn default() -> Self {
        Self::Amnesia
    }
}

/// Accumulator choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccumulatorKind {
    /// BLAKE3 chained accumulator.
    Merkle,
    /// Future seam for a different accumulator.
    Verkle,
}

impl Default for AccumulatorKind {
    fn default() -> Self {
        Self::Merkle
    }
}

/// PQ posture seam for future service wrappers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PqMode {
    /// No PQ requirement in this library.
    Off,
    /// Future hybrid verification posture.
    Hybrid,
}

impl Default for PqMode {
    fn default() -> Self {
        Self::Off
    }
}

/// Hard request and queue limits.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Limits {
    /// Maximum entries per batch.
    pub batch_max_entries: usize,
    /// Maximum batch body bytes when encoded as JSON.
    pub max_body_bytes: usize,
    /// Maximum queue capacity a wrapper may choose to expose.
    pub queue_capacity: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            batch_max_entries: 1024,
            max_body_bytes: 1 << 20,
            queue_capacity: 65_536,
        }
    }
}

/// Ledger config consumed directly by the engine.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LedgerConfig {
    /// Limits for requests and wrappers.
    pub limits: Limits,
    /// Checkpoint every N committed entries.
    pub checkpoint_interval: u64,
    /// Engine profile.
    pub engine_mode: EngineMode,
    /// Accumulator choice.
    pub accumulator_kind: AccumulatorKind,
    /// PQ seam for future wrappers.
    pub pq_mode: PqMode,
}

impl Default for LedgerConfig {
    fn default() -> Self {
        Self {
            limits: Limits::default(),
            checkpoint_interval: 10_000,
            engine_mode: EngineMode::Amnesia,
            accumulator_kind: AccumulatorKind::Merkle,
            pq_mode: PqMode::Off,
        }
    }
}

impl LedgerConfig {
    /// Validate configuration invariants.
    pub fn validate(&self) -> Result<(), LedgerError> {
        if self.limits.batch_max_entries == 0 {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "batch_max_entries must be > 0",
            ));
        }
        if self.limits.max_body_bytes < 1024 {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "max_body_bytes must be >= 1024",
            ));
        }
        if self.limits.queue_capacity < self.limits.batch_max_entries {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "queue_capacity must be >= batch_max_entries",
            ));
        }
        if self.checkpoint_interval == 0 {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "checkpoint_interval must be > 0",
            ));
        }
        Ok(())
    }
}
