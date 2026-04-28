//! RO:WHAT — Sharded in-memory recorder for bounded hot-path usage counters.
//! RO:WHY — Pillar 12; Concerns: ECON/PERF/RES. Records metering without becoming ledger truth.
//! RO:INTERACTS — labels, dimensions, window, slice, config::AccountingConfig.
//! RO:INVARIANTS — non-negative counters; saturating adds; bounded rows; no .await on hot path.
//! RO:METRICS — callers increment accounting_recorded_total and accounting_rows_current.
//! RO:CONFIG — shards and capacity_rows.
//! RO:SECURITY — labels already normalized; no secrets retained intentionally.
//! RO:TEST — unit: recording_tests; benches: record, seal.

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{
    accounting::{
        Dimension, LabelSet, SealedSlice, SliceId, SliceMeta, SliceRow, TenantId, Window,
    },
    errors::{Error, Result},
    utils::time::now_unix_ms,
};

/// Runtime recorder configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecorderConfig {
    /// Number of internal shards; must be a power of two.
    pub shards: usize,
    /// Maximum distinct counter rows retained in memory.
    pub capacity_rows: usize,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            shards: 64,
            capacity_rows: 200_000,
        }
    }
}

/// Hash key for a single in-window counter.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CounterKey {
    /// Normalized labels.
    pub labels: LabelSet,
    /// Metered dimension.
    pub dimension: Dimension,
}

impl CounterKey {
    /// Construct a counter key.
    pub fn new(labels: LabelSet, dimension: Dimension) -> Self {
        Self { labels, dimension }
    }
}

/// Snapshot row produced by the recorder.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CounterRow {
    /// Counter key.
    pub key: CounterKey,
    /// Current non-negative value.
    pub value: u64,
}

/// Sharded bounded recorder for transient usage counters.
#[derive(Debug, Clone)]
pub struct Recorder {
    shards: Arc<Vec<RwLock<HashMap<CounterKey, u64>>>>,
    row_count: Arc<AtomicUsize>,
    cfg: RecorderConfig,
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new(RecorderConfig::default()).expect("default recorder config is valid")
    }
}

impl Recorder {
    /// Construct a recorder with validated configuration.
    pub fn new(cfg: RecorderConfig) -> Result<Self> {
        validate_recorder_config(&cfg)?;
        let mut shards = Vec::with_capacity(cfg.shards);
        for _ in 0..cfg.shards {
            shards.push(RwLock::new(HashMap::new()));
        }
        Ok(Self {
            shards: Arc::new(shards),
            row_count: Arc::new(AtomicUsize::new(0)),
            cfg,
        })
    }

    /// Record a non-negative increment for a normalized label/dimension pair.
    pub fn record(&self, labels: LabelSet, dimension: Dimension, inc: u64) -> Result<()> {
        if inc == 0 {
            return Ok(());
        }
        let key = CounterKey::new(labels, dimension);
        let shard_idx = self.shard_index(&key);
        let mut shard = self.shards[shard_idx].write();

        match shard.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                let next = entry.get().saturating_add(inc);
                entry.insert(next);
                Ok(())
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let previous = self.row_count.fetch_add(1, Ordering::AcqRel);
                if previous >= self.cfg.capacity_rows {
                    self.row_count.fetch_sub(1, Ordering::AcqRel);
                    return Err(Error::Busy);
                }
                entry.insert(inc);
                Ok(())
            }
        }
    }

    /// Return a deterministic snapshot of all current rows without clearing them.
    pub fn snapshot(&self) -> Vec<CounterRow> {
        let mut rows = Vec::new();
        for shard in self.shards.iter() {
            let guard = shard.read();
            rows.extend(guard.iter().map(|(key, value)| CounterRow {
                key: key.clone(),
                value: *value,
            }));
        }
        rows.sort();
        rows
    }

    /// Seal and drain rows for a specific `(tenant, dimension)` stream.
    pub fn seal_slice(
        &self,
        id: SliceId,
        window: Window,
        prev_b3: Option<String>,
        amnesia: bool,
    ) -> Result<SealedSlice> {
        let rows = self.drain_stream(id.tenant, id.dimension);
        let sealed_at_ms = now_unix_ms()?;
        let meta = SliceMeta::new(window, sealed_at_ms, prev_b3, amnesia);
        SealedSlice::new(id, meta, rows)
    }

    /// Return the current approximate distinct row count.
    pub fn row_count(&self) -> usize {
        self.row_count.load(Ordering::Acquire)
    }

    /// Clear all counters.
    pub fn clear(&self) {
        for shard in self.shards.iter() {
            shard.write().clear();
        }
        self.row_count.store(0, Ordering::Release);
    }

    fn drain_stream(&self, tenant: TenantId, dimension: Dimension) -> Vec<SliceRow> {
        let mut rows = Vec::new();
        let mut removed = 0_usize;

        for shard in self.shards.iter() {
            let mut guard = shard.write();
            let keys_to_remove: Vec<_> = guard
                .keys()
                .filter(|key| key.labels.tenant == tenant && key.dimension == dimension)
                .cloned()
                .collect();

            for key in keys_to_remove {
                if let Some(value) = guard.remove(&key) {
                    removed += 1;
                    rows.push(SliceRow {
                        labels: key.labels,
                        dimension: key.dimension,
                        value,
                    });
                }
            }
        }

        if removed > 0 {
            self.row_count.fetch_sub(removed, Ordering::AcqRel);
        }
        rows.sort();
        rows
    }

    fn shard_index(&self, key: &CounterKey) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) & (self.cfg.shards - 1)
    }
}

fn validate_recorder_config(cfg: &RecorderConfig) -> Result<()> {
    if cfg.shards == 0 || !cfg.shards.is_power_of_two() || cfg.shards > 4096 {
        return Err(Error::schema(
            "recorder shards must be a power of two in [1, 4096]",
        ));
    }
    if cfg.capacity_rows < 1024 {
        return Err(Error::schema(
            "recorder capacity_rows must be at least 1024",
        ));
    }
    Ok(())
}
