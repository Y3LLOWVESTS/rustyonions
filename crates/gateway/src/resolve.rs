// crates/gateway/src/resolve.rs
#![forbid(unsafe_code)]

// Bus-only resolver: uses svc-index over UDS via IndexClient.
// The old sled-based code is removed. Signature kept for compatibility.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::index_client::IndexClient;

/// Resolve an address like "b3:<hex>.tld" to its bundle directory via svc-index.
///
/// NOTE: `index_db` is no longer used (legacy param kept to avoid breaking callers).
/// The socket path comes from RON_INDEX_SOCK or falls back to "/tmp/ron/svc-index.sock".
pub fn resolve_addr(_index_db: &Path, addr_str: &str) -> Result<PathBuf> {
    let client = IndexClient::from_env_or("/tmp/ron/svc-index.sock");
    client
        .resolve_dir(addr_str)
        .with_context(|| format!("resolve_addr({addr_str})"))
}
