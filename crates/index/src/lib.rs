use anyhow::Result;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::{path::{Path, PathBuf}, str};
use naming::Address;

/// Default DB path inside repo (you can change when wiring into node)
pub const DEFAULT_DB_PATH: &str = ".data/index";

const TREE_ADDR: &str = "addr";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddrEntry {
    /// Absolute path to the bundle directory (where Manifest.toml lives)
    pub bundle_dir: PathBuf,
    /// Optional human tags later if needed
    pub created_unix: i64,
}

pub struct Index {
    db: Db,
}

impl Index {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    fn addr_tree(&self) -> sled::Tree {
        self.db.open_tree(TREE_ADDR).expect("tree open")
    }

    pub fn put_address(&self, addr: &Address, bundle_dir: impl AsRef<Path>) -> Result<()> {
        let entry = AddrEntry {
            bundle_dir: dunce::canonicalize(bundle_dir)?,
            created_unix: chrono::Utc::now().timestamp(),
        };
        let bytes = bincode::serialize(&entry)?;
        self.addr_tree().insert(addr.to_string(), bytes)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn get_address(&self, addr: &Address) -> Result<Option<AddrEntry>> {
        let opt = self.addr_tree().get(addr.to_string())?;
        if let Some(iv) = opt {
            Ok(Some(bincode::deserialize::<AddrEntry>(&iv)?))
        } else {
            Ok(None)
        }
    }
}
