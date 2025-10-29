// RO:WHAT  — Storage trait + simple in-memory impl for smoke/local dev.
// RO:WHY   — Keep the trait object-safe (handlers hold Arc<dyn Storage>).
// RO:INVARIANTS — CID is content-addressed; NotFound on missing keys; range bounds clamped by caller.

use std::{collections::HashMap, sync::Arc};

use axum::body::Bytes;
use parking_lot::RwLock;

use crate::errors::StorageError;

#[derive(Debug, Clone)]
pub struct HeadMeta {
    pub len: u64,
    pub etag: String,
}

pub type Result<T, E = StorageError> = std::result::Result<T, E>;

#[async_trait::async_trait]
pub trait Storage: Send + Sync + 'static {
    async fn put(&self, cid: &str, data: Bytes) -> Result<()>;

    #[allow(dead_code)]
    async fn exists(&self, cid: &str) -> Result<bool>;

    async fn head(&self, cid: &str) -> Result<HeadMeta>;

    #[allow(dead_code)]
    async fn get_full(&self, cid: &str) -> Result<Bytes>;

    /// Returns (bytes, total_len). Caller provides inclusive range.
    async fn get_range(&self, cid: &str, start: u64, end_inclusive: u64) -> Result<(Bytes, u64)>;
}

/// A simple in-memory storage for smoke tests and local development.
pub struct MemoryStorage {
    inner: RwLock<HashMap<String, Bytes>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Storage for MemoryStorage {
    async fn put(&self, cid: &str, data: Bytes) -> Result<()> {
        let mut map = self.inner.write();
        map.insert(cid.to_string(), data);
        Ok(())
    }

    #[allow(dead_code)]
    async fn exists(&self, cid: &str) -> Result<bool> {
        let map = self.inner.read();
        Ok(map.contains_key(cid))
    }

    async fn head(&self, cid: &str) -> Result<HeadMeta> {
        let map = self.inner.read();
        let v = map.get(cid).ok_or(StorageError::NotFound)?;
        let len = v.len() as u64;
        // Strong ETag (content hash).
        let etag = format!("\"{}\"", blake3::hash(v).to_hex());
        Ok(HeadMeta { len, etag })
    }

    async fn get_full(&self, cid: &str) -> Result<Bytes> {
        let map = self.inner.read();
        let v = map.get(cid).ok_or(StorageError::NotFound)?;
        Ok(v.clone())
    }

    async fn get_range(&self, cid: &str, start: u64, end_inclusive: u64) -> Result<(Bytes, u64)> {
        let map = self.inner.read();
        let v = map.get(cid).ok_or(StorageError::NotFound)?;
        let total_len = v.len() as u64;

        // Clamp defensively; inclusive end.
        let s = (start as usize).min(v.len());
        let e = (end_inclusive as usize).min(v.len().saturating_sub(1));
        let s = s.min(e);

        // Zero-copy slice.
        let out = v.slice(s..=e);
        Ok((out, total_len))
    }
}

// Convenience so other modules can hold Arc<dyn Storage>.
pub type DynStorage = Arc<dyn Storage + Send + Sync + 'static>;
