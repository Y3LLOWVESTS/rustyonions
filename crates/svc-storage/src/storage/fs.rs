//! Filesystem-backed storage for svc-storage.

use std::path::{Path, PathBuf};

use axum::body::Bytes;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::{errors::StorageError, types::HeadMeta};

/// Simple filesystem store rooted at `root/`.
pub struct FsStorage {
    root: PathBuf,
}

impl FsStorage {
    pub async fn new(root: PathBuf) -> anyhow::Result<Self> {
        if !root.exists() {
            fs::create_dir_all(&root).await?;
        }
        Ok(Self { root })
    }

    fn is_valid_b3_cid(cid: &str) -> bool {
        // "b3:" + 64 lowercase hex nybbles
        if let Some(rest) = cid.strip_prefix("b3:") {
            rest.len() == 64 && rest.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        } else {
            false
        }
    }

    fn path_for(&self, cid: &str) -> Result<PathBuf, StorageError> {
        if !Self::is_valid_b3_cid(cid) {
            return Err(StorageError::BadRequest("invalid cid".into()));
        }
        Ok(self.root.join(cid))
    }

    async fn write_all_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
        let tmp = path.with_extension("tmp");
        {
            let mut f = fs::File::create(&tmp).await?;
            f.write_all(data).await?;
            f.flush().await?;
        }
        // Replace temp with final
        // On all platforms tokio::fs::rename overwrites if allowed by OS.
        fs::rename(&tmp, path).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl crate::storage::Storage for FsStorage {
    async fn put_bytes(&self, bytes: Bytes) -> Result<HeadMeta, StorageError> {
        // Compute BLAKE3 CID
        let hash = blake3::hash(&bytes);
        let cid = format!("b3:{}", hash.to_hex());
        let len = bytes.len() as u64;

        let path = self.path_for(&cid)?;
        if !path.exists() {
            Self::write_all_atomic(&path, &bytes)
                .await
                .map_err(StorageError::Io)?;
        }

        Ok(HeadMeta { cid, len })
    }

    async fn has(&self, cid: &str) -> Result<bool, StorageError> {
        let path = self.path_for(cid)?;
        Ok(path.exists())
    }

    async fn head(&self, cid: &str) -> Result<HeadMeta, StorageError> {
        let path = self.path_for(cid)?;
        let meta = fs::metadata(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound
            } else {
                StorageError::Io(e)
            }
        })?;
        Ok(HeadMeta {
            cid: cid.to_string(),
            len: meta.len(),
        })
    }

    async fn get_full(&self, cid: &str) -> Result<Bytes, StorageError> {
        let path = self.path_for(cid)?;
        let mut f = fs::File::open(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound
            } else {
                StorageError::Io(e)
            }
        })?;

        let mut buf = Vec::new();
        f.read_to_end(&mut buf).await.map_err(StorageError::Io)?;
        Ok(Bytes::from(buf))
    }

    async fn get_range(
        &self,
        cid: &str,
        start: u64,
        end_inclusive: u64,
    ) -> Result<(Bytes, u64), StorageError> {
        let path = self.path_for(cid)?;
        let meta = fs::metadata(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound
            } else {
                StorageError::Io(e)
            }
        })?;
        let total = meta.len();
        if start > end_inclusive || end_inclusive >= total {
            return Err(StorageError::BadRequest("invalid range".into()));
        }

        let mut f = fs::File::open(&path).await.map_err(StorageError::Io)?;
        let span = (end_inclusive - start + 1) as usize;
        let mut buf = vec![0u8; span];

        f.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(StorageError::Io)?;
        f.read_exact(&mut buf).await.map_err(StorageError::Io)?;

        Ok((Bytes::from(buf), total))
    }
}
