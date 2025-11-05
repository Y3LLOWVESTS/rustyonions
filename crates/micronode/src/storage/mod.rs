// crates/micronode/src/storage/mod.rs
//! RO:WHAT — Minimal Storage trait + in-memory impl (foundation).
//! RO:WHY  — Unblock KV routes without pulling new deps; swap to sled next.
//! RO:INVARIANTS — Sync trait (no async trait object needed); object-safe.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::errors::{Error, Result};

/// Sync storage interface for tiny KV (bucket/key → bytes).
pub trait Storage: Send + Sync + 'static {
    fn put(&self, bucket: &str, key: &str, val: &[u8]) -> Result<()>;
    fn get(&self, bucket: &str, key: &str) -> Result<Option<Vec<u8>>>;
    fn del(&self, bucket: &str, key: &str) -> Result<bool>;
}

pub type DynStorage = Arc<dyn Storage>;

/// In-memory baseline: buckets → (key → bytes)
#[derive(Default)]
pub struct MemStore {
    inner: RwLock<HashMap<String, HashMap<String, Vec<u8>>>>,
}

impl MemStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Storage for MemStore {
    fn put(&self, bucket: &str, key: &str, val: &[u8]) -> Result<()> {
        let mut guard = self.inner.write().map_err(|_| Error::Internal)?;
        let map = guard.entry(bucket.to_string()).or_default();
        map.insert(key.to_string(), val.to_vec());
        Ok(())
    }

    fn get(&self, bucket: &str, key: &str) -> Result<Option<Vec<u8>>> {
        let guard = self.inner.read().map_err(|_| Error::Internal)?;
        Ok(guard.get(bucket).and_then(|m| m.get(key)).cloned())
    }

    fn del(&self, bucket: &str, key: &str) -> Result<bool> {
        let mut guard = self.inner.write().map_err(|_| Error::Internal)?;
        if let Some(map) = guard.get_mut(bucket) {
            Ok(map.remove(key).is_some())
        } else {
            Ok(false)
        }
    }
}
