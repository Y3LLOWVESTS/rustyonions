//! RO:WHAT — Tiny, std-only LRU cache (size-bounded).
//! RO:WHY  — Provide a simple building block for SDK-local caches without
//!           pulling in an external LRU crate.
//! RO:INTERACTS — Wrapped by `cache::mod` to add TTL behavior and metrics.
//! RO:INVARIANTS —
//!   - Capacity is always >= 1.
//!   - Insertion is bounded: on overflow, the least-recently-used entry
//!     is evicted.
//!   - `get` and `insert` are O(n) (small n: max_entries from config).
//! RO:METRICS — None here; outer cache may emit hit/miss counters.
//! RO:CONFIG — Capacity supplied by `CacheCfg.max_entries`.
//! RO:SECURITY — In-memory only; no persistence.
//! RO:TEST — Unit tests in this module.

use std::collections::VecDeque;

/// Very small LRU cache backed by a `VecDeque`.
///
/// This is intentionally simple and does not attempt to be maximally
/// efficient; for typical SDK cache sizes (≈1k entries) an O(n) scan
/// is sufficient and keeps the implementation easy to audit.
#[derive(Debug)]
pub struct Lru<K, V> {
    capacity: usize,
    entries: VecDeque<(K, V)>,
}

impl<K, V> Lru<K, V> {
    /// Construct a new LRU with the given capacity.
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(1);
        Self {
            capacity: cap,
            entries: VecDeque::with_capacity(cap),
        }
    }

    /// Current number of entries in the cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<K, V> Lru<K, V>
where
    K: Eq,
    V: Clone,
{
    /// Get a value by key, marking it as most-recently-used.
    ///
    /// Returns a cloned value. This keeps the implementation simple and
    /// avoids lifetime juggling while still being cheap for typical V.
    pub fn get(&mut self, key: &K) -> Option<V> {
        let mut hit_index = None;

        for (idx, (k, _)) in self.entries.iter().enumerate() {
            if k == key {
                hit_index = Some(idx);
                break;
            }
        }

        let idx = hit_index?;
        let (_, v) = &self.entries[idx];
        let value = v.clone();

        // Move the entry to the back if it wasn't already there.
        if idx + 1 != self.entries.len() {
            if let Some(entry) = self.entries.remove(idx) {
                self.entries.push_back(entry);
            }
        }

        Some(value)
    }

    /// Insert or replace a value.
    ///
    /// Returns the previous value for this key, if any.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        // Remove existing entry if present.
        let mut old = None;
        let mut existing_index = None;

        for (idx, (k, _)) in self.entries.iter().enumerate() {
            if k == &key {
                existing_index = Some(idx);
                break;
            }
        }

        if let Some(idx) = existing_index {
            if let Some((_, v)) = self.entries.remove(idx) {
                old = Some(v);
            }
        }

        self.entries.push_back((key, value));

        // Enforce capacity.
        if self.entries.len() > self.capacity {
            let _ = self.entries.pop_front();
        }

        old
    }

    /// Remove a key from the cache, returning its value if present.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let mut index = None;
        for (idx, (k, _)) in self.entries.iter().enumerate() {
            if k == key {
                index = Some(idx);
                break;
            }
        }

        index.and_then(|idx| self.entries.remove(idx).map(|(_, v)| v))
    }
}

#[cfg(test)]
mod tests {
    use super::Lru;

    #[test]
    fn obeys_capacity_and_lru_order() {
        let mut lru = Lru::new(2);

        lru.insert("a", 1);
        lru.insert("b", 2);

        // Access "a" to make it most recent.
        assert_eq!(lru.get(&"a"), Some(1));

        // Insert "c" — should evict "b".
        lru.insert("c", 3);

        assert_eq!(lru.get(&"a"), Some(1));
        assert_eq!(lru.get(&"b"), None);
        assert_eq!(lru.get(&"c"), Some(3));
    }

    #[test]
    fn remove_works() {
        let mut lru = Lru::new(2);
        lru.insert("a", 1);
        lru.insert("b", 2);

        assert_eq!(lru.remove(&"a"), Some(1));
        assert_eq!(lru.get(&"a"), None);
        assert_eq!(lru.len(), 1);
    }
}
