//! RO:WHAT — Ephemeral TTL cache facade for SDK callers.
//! RO:WHY  — Provide a small, in-memory cache to hide repeated GETs or
//!           metadata lookups without ever touching disk (I-11 no-persistence).
//! RO:INTERACTS — Wraps `cache::lru::Lru`; driven by `CacheCfg` from config;
//!                may be used by storage/index planes later.
//! RO:INVARIANTS —
//!   - Obeys `CacheCfg.max_entries` (bounded size).
//!   - Per-entry TTL enforced on read; expired entries are evicted.
//!   - Purely in-memory, process-local; no serialization.
//! RO:METRICS — Places to hook cache hit/miss counters via `SdkMetrics`.
//! RO:CONFIG — Reads `CacheCfg { enabled, max_entries, ttl }`.
//! RO:SECURITY — Does not store secrets long-term; caller chooses what to
//!               cache. Amnesia mode may disable cache at a higher level.
//! RO:TEST — Unit tests in this module.

mod lru;

pub use lru::Lru;

use std::time::{Duration, Instant};

use crate::config::CacheCfg;

/// Internal wrapper storing value + insertion timestamp.
#[derive(Debug, Clone)]
struct Entry<V> {
    value: V,
    inserted_at: Instant,
}

impl<V> Entry<V> {
    fn new(value: V) -> Self {
        Self {
            value,
            inserted_at: Instant::now(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.inserted_at.elapsed() > ttl
    }
}

/// Simple TTL + size-bounded cache built on top of `Lru`.
///
/// Generic over key and value types. This is intended for relatively
/// small caches (hundreds to a few thousand entries).
#[derive(Debug)]
pub struct TtlCache<K, V> {
    cfg: CacheCfg,
    inner: Lru<K, Entry<V>>,
}

impl<K, V> TtlCache<K, V>
where
    K: Eq,
    V: Clone,
{
    /// Create a new cache from configuration.
    ///
    /// If `cfg.enabled` is false, callers should typically avoid creating
    /// the cache at all and treat this as a no-op.
    pub fn new(cfg: CacheCfg) -> Self {
        let cap = cfg.max_entries.max(1);
        Self {
            cfg,
            inner: Lru::new(cap),
        }
    }

    /// Access the underlying config.
    pub fn config(&self) -> &CacheCfg {
        &self.cfg
    }

    /// Look up a value by key, enforcing TTL.
    ///
    /// Returns `None` if the key is not present or the entry has expired.
    pub fn get(&mut self, key: &K) -> Option<V> {
        if !self.cfg.enabled {
            return None;
        }

        let ttl = self.cfg.ttl;
        if ttl.is_zero() {
            return None;
        }

        if let Some(entry) = self.inner.get(key) {
            if entry.is_expired(ttl) {
                // Evict expired entry.
                let _ = self.inner.remove(key);
                None
            } else {
                Some(entry.value)
            }
        } else {
            None
        }
    }

    /// Insert or replace a value for the given key.
    pub fn insert(&mut self, key: K, value: V) {
        if !self.cfg.enabled {
            return;
        }

        let entry = Entry::new(value);
        let _ = self.inner.insert(key, entry);
    }

    /// Remove an entry (if present).
    pub fn remove(&mut self, key: &K) {
        let _ = self.inner.remove(key);
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        // Reinitialize the LRU with the configured capacity.
        let cap = self.cfg.max_entries.max(1);
        self.inner = Lru::new(cap);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_enabled() -> CacheCfg {
        CacheCfg {
            enabled: true,
            max_entries: 2,
            ttl: Duration::from_millis(10),
        }
    }

    #[test]
    fn respects_enabled_flag() {
        let cfg = CacheCfg {
            enabled: false,
            max_entries: 2,
            ttl: Duration::from_secs(60),
        };

        let mut cache = TtlCache::new(cfg);
        cache.insert("a", 1);
        assert_eq!(cache.get(&"a"), None);
    }

    #[test]
    fn evicts_on_ttl() {
        let cfg = cfg_enabled();
        let mut cache = TtlCache::new(cfg);

        cache.insert("a", 1);
        assert_eq!(cache.get(&"a"), Some(1));

        std::thread::sleep(Duration::from_millis(15));
        assert_eq!(cache.get(&"a"), None);
    }

    #[test]
    fn respects_capacity_lru_behavior() {
        let cfg = CacheCfg {
            enabled: true,
            max_entries: 2,
            ttl: Duration::from_secs(60),
        };

        let mut cache = TtlCache::new(cfg);

        cache.insert("a", 1);
        cache.insert("b", 2);

        // Touch "a" to make it recently used.
        assert_eq!(cache.get(&"a"), Some(1));

        // Insert "c" — should evict "b".
        cache.insert("c", 3);

        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"c"), Some(3));
    }
}
