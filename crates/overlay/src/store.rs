#![forbid(unsafe_code)]
//! Persistence layer for the overlay store.

use crate::error::OverlayError;
use blake3::Hasher;
use sled::{Db, IVec};

/// A local content-addressed store backed by `sled`.
/// Data are currently stored monolithically under the blake3 hash key.
/// `chunk_size` is reserved for a future chunked layout.
pub struct Store {
    db: Db,
    bytes: sled::Tree,
    _meta: sled::Tree,
    /// Reserved for future chunking (len-prefix framing / chunked storage).
    pub(crate) chunk_size: usize,
}

impl Clone for Store {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            bytes: self.bytes.clone(),
            _meta: self._meta.clone(),
            chunk_size: self.chunk_size,
        }
    }
}

impl Store {
    /// Open a store at `path` with the given `chunk_size`.
    pub fn open(path: impl AsRef<std::path::Path>, chunk_size: usize) -> anyhow::Result<Self> {
        if chunk_size != 0 && chunk_size < 4096 {
            return Err(OverlayError::InvalidChunkSize.into());
        }
        let db = sled::open(path)?;
        let bytes = db.open_tree("bytes")?;
        let meta  = db.open_tree("meta")?;
        Ok(Self { db, bytes, _meta: meta, chunk_size })
    }

    /// Insert bytes, returning their blake3 **hex** content hash.
    pub fn put(&self, data: &[u8]) -> anyhow::Result<String> {
        // Touch field to avoid "unused" until chunking lands.
        let _ = self.chunk_size;

        let mut hasher = Hasher::new();
        hasher.update(data);
        let hash = hex::encode(hasher.finalize().as_bytes());

        self.bytes.insert(hash.as_bytes(), IVec::from(data))?;
        self.db.flush()?;
        Ok(hash)
    }

    /// Retrieve bytes by content hash. Returns `Ok(None)` if not found.
    pub fn get(&self, hash: &str) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.bytes.get(hash.as_bytes())?.map(|v| v.to_vec()))
    }
}
