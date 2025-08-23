#![forbid(unsafe_code)]

use anyhow::Result;
use sled::Db;
use std::path::Path;

#[derive(Clone)]
pub struct Store {
    db: Db,
}

impl Store {
    /// Open sled at the given path.
    /// Accepts any path-like type to avoid &str / &Path mismatches.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path.as_ref())?;
        Ok(Self { db })
    }

    /// Put a blob by key.
    pub fn put(&self, key: &[u8], val: Vec<u8>) -> Result<()> {
        self.db.insert(key, val)?;
        self.db.flush()?; // ensure durability for tests/scripts
        Ok(())
    }

    /// Get a blob by key.
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.db.get(key)?.map(|iv| iv.to_vec()))
    }
}
