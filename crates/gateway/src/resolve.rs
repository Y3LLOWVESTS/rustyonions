#![forbid(unsafe_code)]

use anyhow::{bail, Context, Result};
use sled::Db;
use std::path::{Path, PathBuf};

use naming::Address;

fn open_index(path: &Path) -> Result<Db> {
    Ok(sled::open(path)?)
}

/// Resolve "b3:<hex>.tld" or "<hex>.tld" to a bundle directory path.
pub fn resolve_addr(index_db: &Path, addr_str: &str) -> Result<PathBuf> {
    let addr = Address::parse(addr_str).context("parse address")?;
    let key_with_prefix = addr.to_string_with_prefix(true); // "b3:<hex>.tld"
    let db = open_index(index_db)?;
    if let Some(v) = db.get(key_with_prefix.as_bytes())? {
        let p = String::from_utf8_lossy(&v).to_string();
        let pb = PathBuf::from(p);
        if pb.is_dir() {
            return Ok(pb);
        } else {
            bail!("indexed path is not a dir: {}", pb.display());
        }
    }

    // Fallback: try bare "<hex>.tld" key (if older entries were stored without "b3:")
    let bare = addr.to_string_with_prefix(false);
    if let Some(v) = db.get(bare.as_bytes())? {
        let p = String::from_utf8_lossy(&v).to_string();
        let pb = PathBuf::from(p);
        if pb.is_dir() {
            return Ok(pb);
        } else {
            bail!("indexed path is not a dir: {}", pb.display());
        }
    }

    bail!("address not found in index: {addr_str}");
}
