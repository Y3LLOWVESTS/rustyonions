//! RO:WHAT — Storage abstraction for Micronode (KV engine + in-memory implementation).
//! RO:WHY  — Give Micronode a boring key/value API that can later plug sled/RocksDB/etc.
//! RO:INTERACTS — Used by state::AppState and HTTP KV handlers (http::kv).
//! RO:INVARIANTS — No locks across `.await`; operations are short, bounded, and sync.
//! RO:METRICS — KV ops/bytes metrics can be layered on top later (domain counters).
//! RO:CONFIG — Engine selection will come from Config (engine="mem" | "sled") in a later step.
//! RO:SECURITY — No auth here; capability/policy checks live at HTTP layer.
//! RO:TEST — Unit tests for MemStore + HTTP integration tests for KV routes (future).

use crate::errors::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for Micronode KV engines.
///
/// This is intentionally small and synchronous; HTTP handlers stay async and
/// call into this trait without holding locks across `.await`.
pub trait Storage: Send + Sync {
    /// Insert or overwrite a value.
    ///
    /// Returns `Ok(true)` if the key was newly created, `Ok(false)` if it
    /// replaced an existing value.
    fn put(&self, bucket: &str, key: &str, value: &[u8]) -> Result<bool>;

    /// Fetch a value, if present.
    fn get(&self, bucket: &str, key: &str) -> Result<Option<Vec<u8>>>;

    /// Delete a value.
    ///
    /// Returns `Ok(true)` if a value existed and was removed, `Ok(false)` if
    /// the key was absent.
    fn delete(&self, bucket: &str, key: &str) -> Result<bool>;
}

/// Shared trait object type for storage engines.
pub type DynStorage = Arc<dyn Storage + Send + Sync>;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct BucketKey {
    bucket: String,
    key: String,
}

impl BucketKey {
    fn new(bucket: &str, key: &str) -> Self {
        Self { bucket: bucket.to_owned(), key: key.to_owned() }
    }
}

/// Simple in-memory store backed by a `RwLock<HashMap<BucketKey, Vec<u8>>>`.
///
/// Intended for:
///   - dev/prototyping
///   - amnesia-first micronode profiles
///   - tests
#[derive(Debug, Default)]
pub struct MemStore {
    inner: RwLock<HashMap<BucketKey, Vec<u8>>>,
}

impl MemStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Storage for MemStore {
    fn put(&self, bucket: &str, key: &str, value: &[u8]) -> Result<bool> {
        let k = BucketKey::new(bucket, key);
        let mut guard = self.inner.write();
        let existed = guard.insert(k, value.to_vec()).is_some();
        Ok(!existed)
    }

    fn get(&self, bucket: &str, key: &str) -> Result<Option<Vec<u8>>> {
        let k = BucketKey::new(bucket, key);
        let guard = self.inner.read();
        Ok(guard.get(&k).cloned())
    }

    fn delete(&self, bucket: &str, key: &str) -> Result<bool> {
        let k = BucketKey::new(bucket, key);
        let mut guard = self.inner.write();
        Ok(guard.remove(&k).is_some())
    }
}

// In the next step we can add:
//
// #[cfg(feature = "sled-store")]
// pub mod sled_store;
//
// and implement `Storage` for a sled-backed engine.
