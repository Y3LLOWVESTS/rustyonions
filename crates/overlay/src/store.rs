use anyhow::Result;
use lru::LruCache;
use sled;
use std::num::NonZeroUsize;
use std::sync::Mutex;

#[derive(Debug)]
pub struct Store {
    db: sled::Db,
    cache: Mutex<LruCache<Vec<u8>, Vec<u8>>>, // key/value are bytes
}

impl Clone for Store {
    fn clone(&self) -> Self {
        // sled::Db is cheap-to-clone (handle to the same database).
        let cap = NonZeroUsize::new(1024).unwrap();
        Self {
            db: self.db.clone(),
            cache: Mutex::new(LruCache::new(cap)),
        }
    }
}

impl Store {
    pub fn open(path: &str) -> Result<Self> {
        let db = sled::open(path)?;
        let cap = NonZeroUsize::new(1024).unwrap();
        Ok(Self {
            db,
            cache: Mutex::new(LruCache::new(cap)),
        })
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        if let Some(v) = self.cache.lock().unwrap().get(key).cloned() {
            return Ok(Some(v));
        }
        let res = self.db.get(key)?.map(|ivec| ivec.to_vec());
        if let Some(ref v) = res {
            self.cache.lock().unwrap().put(key.to_vec(), v.clone());
        }
        Ok(res)
    }

    /// Store value under `key` exactly.
    pub fn put(&self, key: &[u8], val: Vec<u8>) -> Result<()> {
        self.cache.lock().unwrap().put(key.to_vec(), val.clone());
        self.db.insert(key, val)?;
        Ok(())
    }
}
