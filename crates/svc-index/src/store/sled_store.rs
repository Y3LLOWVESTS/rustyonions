//! Sled-backed (or in-memory) key→manifest store (MVP).
//! Changes: honor RON_INDEX_DB env; flush after writes for durability.

#[cfg(feature = "sled-store")]
#[derive(Clone)]
pub struct SledStore {
    /// Tree that holds key → manifest CID
    man: sled::Tree,
    _db: sled::Db,
}

#[cfg(feature = "sled-store")]
impl SledStore {
    pub fn open() -> anyhow::Result<Self> {
        let path = std::env::var("RON_INDEX_DB").unwrap_or_else(|_| "svc-index.db".into());
        let db = sled::open(path)?;
        let man = db.open_tree("manifest")?;
        Ok(Self { man, _db: db })
    }
    pub fn get_manifest(&self, key: &str) -> Option<String> {
        self.man
            .get(key.as_bytes())
            .ok()
            .flatten()
            .and_then(|ivec| String::from_utf8(ivec.to_vec()).ok())
    }
    pub fn put_manifest(&self, key: &str, cid: &str) {
        let _ = self.man.insert(key.as_bytes(), cid.as_bytes());
        let _ = self.man.flush(); // ensure durability for beta MVP
    }
}

#[derive(Clone, Default)]
pub struct MemStore {
    map: std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<String, String>>>,
}

impl MemStore {
    pub fn get_manifest(&self, key: &str) -> Option<String> {
        self.map.read().get(key).cloned()
    }
    pub fn put_manifest(&self, key: &str, cid: &str) {
        self.map.write().insert(key.to_string(), cid.to_string());
    }
}
