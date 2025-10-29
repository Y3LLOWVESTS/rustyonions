//! RO:WHAT — In-memory CAS (Micronode amnesia).

use super::{HeadMeta, Storage};
use crate::{errors::StorageError, prelude::*};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct MemoryStorage {
    inner: Mutex<HashMap<String, Bytes>>,
}

#[async_trait::async_trait]
impl Storage for MemoryStorage {
    async fn put_bytes(&self, bytes: Bytes) -> Result<HeadMeta, StorageError> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&bytes);
        let cid = format!("b3:{}", hasher.finalize().to_hex());
        let mut g = self.inner.lock();
        g.entry(cid.clone()).or_insert(bytes.clone());
        Ok(HeadMeta {
            cid: cid.clone(),
            size: bytes.len() as u64,
            etag: format!("\"{}\"", cid),
        })
    }

    async fn has(&self, cid: &str) -> Result<bool, StorageError> {
        Ok(self.inner.lock().contains_key(cid))
    }

    async fn head(&self, cid: &str) -> Result<HeadMeta, StorageError> {
        let g = self.inner.lock();
        let b = g.get(cid).ok_or(StorageError::NotFound)?;
        Ok(HeadMeta {
            cid: cid.to_string(),
            size: b.len() as u64,
            etag: format!("\"{}\"", cid),
        })
    }

    async fn get_full(&self, cid: &str) -> Result<Bytes, StorageError> {
        let g = self.inner.lock();
        let b = g.get(cid).ok_or(StorageError::NotFound)?;
        Ok(b.clone())
    }

    async fn get_range(
        &self,
        cid: &str,
        start: u64,
        end_inclusive: u64,
    ) -> Result<(Bytes, u64), StorageError> {
        let g = self.inner.lock();
        let b = g.get(cid).ok_or(StorageError::NotFound)?;
        let len = b.len() as u64;
        if start >= len || end_inclusive >= len || start > end_inclusive {
            return Err(StorageError::RangeNotSatisfiable);
        }
        let s = start as usize;
        let e = end_inclusive as usize + 1;
        Ok((b.slice(s..e), len))
    }
}
