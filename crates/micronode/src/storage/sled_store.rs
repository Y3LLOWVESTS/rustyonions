// crates/micronode/src/storage/sled_store.rs
//! RO:WHAT — Sled-backed Storage adapter (bucket = tree; key = binary value).
//! RO:MODE — Behind `sled-store` feature; not used by default.
//! RO:ERRS — Map sled errors to `Error::Internal` (beta scope).

#[cfg(feature = "sled-store")]
mod sled_adapter {
    use super::super::{Storage};
    use crate::errors::{Error, Result};
    use std::sync::Arc;

    pub struct SledStore {
        db: sled::Db,
    }

    impl SledStore {
        pub fn open(path: &str) -> Result<Arc<Self>> {
            let db = sled::open(path).map_err(|_| Error::Internal)?;
            Ok(Arc::new(Self { db }))
        }

        #[inline]
        fn tree(&self, bucket: &str) -> Result<sled::Tree> {
            self.db.open_tree(bucket).map_err(|_| Error::Internal)
        }
    }

    impl Storage for SledStore {
        fn put(&self, bucket: &str, key: &str, val: &[u8]) -> Result<()> {
            let t = self.tree(bucket)?;
            t.insert(key.as_bytes(), val).map_err(|_| Error::Internal)?;
            Ok(())
        }

        fn get(&self, bucket: &str, key: &str) -> Result<Option<Vec<u8>>> {
            let t = self.tree(bucket)?;
            let v = t.get(key.as_bytes()).map_err(|_| Error::Internal)?;
            Ok(v.map(|ivec| ivec.to_vec()))
        }

        fn del(&self, bucket: &str, key: &str) -> Result<bool> {
            let t = self.tree(bucket)?;
            let removed = t.remove(key.as_bytes()).map_err(|_| Error::Internal)?;
            Ok(removed.is_some())
        }
    }
}

// Public re-export only when feature is enabled.
#[cfg(feature = "sled-store")]
pub use sled_adapter::SledStore;
