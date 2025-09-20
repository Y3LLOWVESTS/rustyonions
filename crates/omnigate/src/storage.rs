#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use std::path::{Component, Path, PathBuf};
use tokio::fs::File;
use tracing::debug;

/// App protocol id for Storage (tiles) GET.
pub const TILE_APP_PROTO_ID: u16 = 0x0301;

/// Filesystem-backed storage for tiles.
///
/// Bronze ring: safe-ish path join (no `..`), size cap for 413 mapping.
/// The actual streaming loop is done by the overlay; this module just resolves and opens.
pub struct FsStorage {
    root: PathBuf,
    pub max_file_bytes: u64,
}

impl FsStorage {
    /// Create a new filesystem storage rooted at `root`. `max_file_bytes` caps responses (→ 413).
    pub fn new(root: impl Into<PathBuf>, max_file_bytes: u64) -> Self {
        Self {
            root: root.into(),
            max_file_bytes,
        }
    }

    /// Resolve a *relative* path (no leading '/') safely under root.
    /// Rejects any path containing `..` or absolute components.
    fn resolve_under_root(&self, rel_path: &str) -> Result<PathBuf> {
        let p = Path::new(rel_path.trim_start_matches('/'));
        if p.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(anyhow!("invalid path"));
        }
        Ok(self.root.join(p))
    }

    /// Open a file and return (File, size). Enforces max size.
    pub async fn open(&self, rel_path: &str) -> Result<(File, u64)> {
        let path = self
            .resolve_under_root(rel_path)
            .with_context(|| format!("resolve {rel_path}"))?;

        let file = File::open(&path)
            .await
            .with_context(|| format!("open {}", path.display()))?;

        // Size (Bronze: read metadata, map errors)
        let meta = file
            .metadata()
            .await
            .with_context(|| format!("stat {}", path.display()))?;

        if !meta.is_file() {
            return Err(anyhow!("not a file"));
        }
        let size = meta.len();
        if size > self.max_file_bytes {
            return Err(anyhow!("too large: {} > {}", size, self.max_file_bytes));
        }

        debug!("open {} ({} bytes)", path.display(), size);
        Ok((file, size))
    }
}
