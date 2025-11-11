//! Adapter: read-only pack access (e.g., PMTiles/pack files) — stub.

use std::path::PathBuf;

/// Pack source descriptor (placeholder).
#[derive(Debug, Clone)]
pub struct PackSource {
    /// Path to on-disk pack file (or future remote locator).
    pub path: PathBuf,
}

/// Minimal pack trait (read-only, stubbed).
pub trait PackStore: Send + Sync {
    /// Return `true` if the pack appears available.
    fn available(&self) -> bool;
}

/// No-op pack implementation used for wiring tests.
#[derive(Debug, Clone, Default)]
pub struct NullPack;

impl PackStore for NullPack {
    fn available(&self) -> bool {
        false
    }
}
