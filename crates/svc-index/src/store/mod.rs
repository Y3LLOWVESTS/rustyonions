//! Store abstraction; sled-backed (feature) or in-memory.

mod sled_store; // declare the sibling module within `store/`

#[derive(Clone)]
pub enum Store {
    #[cfg(feature = "sled-store")]
    Sled(self::sled_store::SledStore),
    Memory(self::sled_store::MemStore),
}

impl Store {
    pub fn new(enable_sled: bool) -> anyhow::Result<Self> {
        if cfg!(feature = "sled-store") && enable_sled {
            Ok(Self::Sled(self::sled_store::SledStore::open()?))
        } else {
            Ok(Self::Memory(self::sled_store::MemStore::default()))
        }
    }

    pub fn get_manifest(&self, key: &str) -> Option<String> {
        match self {
            #[cfg(feature = "sled-store")]
            Store::Sled(s) => s.get_manifest(key),
            Store::Memory(m) => m.get_manifest(key),
        }
    }

    pub fn put_manifest(&self, key: &str, cid: &str) {
        match self {
            #[cfg(feature = "sled-store")]
            Store::Sled(s) => s.put_manifest(key, cid),
            Store::Memory(m) => m.put_manifest(key, cid),
        }
    }
}
