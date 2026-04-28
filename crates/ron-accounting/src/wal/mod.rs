//! RO:WHAT — Bounded WAL handle for sealed slices when persistence is enabled.
//! RO:WHY — Pillar 12; Concerns: ECON/RES. Durable staging is optional and disabled by amnesia.
//! RO:INTERACTS — config::WalConfig, accounting::SealedSlice, replay, segment, fs helpers.
//! RO:INVARIANTS — bounded entries/bytes; no WAL when amnesia normalized config disables it.
//! RO:METRICS — callers expose size_bytes and entries from WalStats.
//! RO:CONFIG — wal.enabled, wal.max_bytes, wal.max_entries, wal.dir.
//! RO:SECURITY — Batch 1 in-memory implementation stores no secrets; disk hardening lands later.
//! RO:TEST — unit: wal_tests with feature=wal.

pub mod fs;
pub mod replay;
pub mod segment;

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::{
    accounting::SealedSlice,
    errors::{Error, Result},
    utils::encode::to_canonical_bytes,
};

pub use crate::config::schema::WalConfig;

/// WAL statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalStats {
    /// Current staged bytes.
    pub size_bytes: u64,
    /// Current staged entries.
    pub entries: u64,
}

/// Bounded WAL handle.
#[derive(Debug, Clone)]
pub struct Wal {
    cfg: WalConfig,
    entries: Arc<Mutex<Vec<SealedSlice>>>,
    size_bytes: Arc<AtomicU64>,
}

impl Wal {
    /// Create a bounded WAL handle.
    pub fn new(cfg: WalConfig) -> Result<Self> {
        if cfg.max_entries == 0 || cfg.max_bytes == 0 {
            return Err(Error::PersistenceFull);
        }
        Ok(Self {
            cfg,
            entries: Arc::new(Mutex::new(Vec::new())),
            size_bytes: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Append a sealed slice to bounded staging.
    pub fn append(&self, slice: SealedSlice) -> Result<()> {
        let bytes = to_canonical_bytes(&slice)?;
        let size = bytes.len() as u64;
        let current = self.size_bytes.load(Ordering::Acquire);
        if current.saturating_add(size) > self.cfg.max_bytes {
            return Err(Error::PersistenceFull);
        }

        let mut entries = self.entries.lock();
        if entries.len() as u64 >= self.cfg.max_entries {
            return Err(Error::PersistenceFull);
        }
        entries.push(slice);
        self.size_bytes.fetch_add(size, Ordering::AcqRel);
        Ok(())
    }

    /// Drain all staged entries in insertion order.
    pub fn drain(&self) -> Vec<SealedSlice> {
        let mut entries = self.entries.lock();
        let drained = std::mem::take(&mut *entries);
        self.size_bytes.store(0, Ordering::Release);
        drained
    }

    /// Current WAL statistics.
    pub fn stats(&self) -> WalStats {
        WalStats {
            size_bytes: self.size_bytes.load(Ordering::Acquire),
            entries: self.entries.lock().len() as u64,
        }
    }
}
