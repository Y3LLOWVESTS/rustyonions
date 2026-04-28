//! RO:WHAT — Small bounded in-memory cache for sealed inputs and manifests.
//! RO:WHY — Pillar 12; Concerns: PERF/RES. Batch 1 needs deterministic, amnesia-safe local caching.
//! RO:INTERACTS — http::RewarderState and future accounting/policy adapters.
//! RO:INVARIANTS — bounded by entry count; RAM-only; no locks held across await.
//! RO:METRICS — cache metrics are future work; callers can count hits/misses later.
//! RO:CONFIG — size can later be wired to inputs_cache_ttl/work_queue_max.
//! RO:SECURITY — no disk persistence in amnesia mode.
//! RO:TEST — exercised by HTTP idempotency path.

use std::collections::VecDeque;

use parking_lot::Mutex;

/// Tiny FIFO cache with a hard entry bound.
#[derive(Debug)]
pub struct FifoCache<K, V> {
    max_entries: usize,
    entries: Mutex<VecDeque<(K, V)>>,
}

impl<K, V> FifoCache<K, V>
where
    K: Clone + Eq,
    V: Clone,
{
    /// Create a cache with at least one entry of capacity.
    #[must_use]
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries: max_entries.max(1),
            entries: Mutex::new(VecDeque::new()),
        }
    }

    /// Get a cached value by key.
    pub fn get(&self, key: &K) -> Option<V> {
        self.entries
            .lock()
            .iter()
            .find_map(|(k, v)| (k == key).then(|| v.clone()))
    }

    /// Insert or replace a value, evicting FIFO when needed.
    pub fn insert(&self, key: K, value: V) {
        let mut entries = self.entries.lock();
        if let Some(pos) = entries.iter().position(|(k, _)| *k == key) {
            entries.remove(pos);
        }
        entries.push_back((key, value));
        while entries.len() > self.max_entries {
            entries.pop_front();
        }
    }
}
