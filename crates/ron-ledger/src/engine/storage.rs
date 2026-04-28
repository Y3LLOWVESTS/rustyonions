//! RO:WHAT — Storage trait plus in-memory and file-backed append-only implementations for records and checkpoints.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. The engine depends on a tiny storage seam instead of a specific DB.
//! RO:INTERACTS — crate::engine::ledger, crate::engine::replay, crate::types::EntryRecord / CheckpointRecord.
//! RO:INVARIANTS — append-only writes; durable backend never mutates history in place; amnesia backend leaves no disk artifacts.
//! RO:METRICS — none directly; wrappers can instrument IO externally if needed.
//! RO:CONFIG — EngineMode decides whether callers choose MemoryStorage or FileStorage.
//! RO:SECURITY — file backend stores only identifiers and ledger records; no secrets or raw capability material.
//! RO:TEST — replay_recovery.rs exercises FileStorage; other tests use MemoryStorage.

use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use parking_lot::Mutex;

use crate::{
    error::{LedgerError, RejectReason},
    types::{CheckpointRecord, EntryRecord},
};

/// Storage seam required by the engine.
pub trait Storage: Send + Sync + 'static {
    /// Append a new entry record.
    fn append_record(&self, record: &EntryRecord) -> Result<(), LedgerError>;
    /// Append a new checkpoint.
    fn append_checkpoint(&self, checkpoint: &CheckpointRecord) -> Result<(), LedgerError>;
    /// Load all entry records in append order.
    fn load_records(&self) -> Result<Vec<EntryRecord>, LedgerError>;
    /// Load all checkpoints in append order.
    fn load_checkpoints(&self) -> Result<Vec<CheckpointRecord>, LedgerError>;
}

/// In-memory storage for amnesia mode and tests.
#[derive(Debug, Default)]
pub struct MemoryStorage {
    records: Mutex<Vec<EntryRecord>>,
    checkpoints: Mutex<Vec<CheckpointRecord>>,
}

impl Storage for MemoryStorage {
    fn append_record(&self, record: &EntryRecord) -> Result<(), LedgerError> {
        self.records.lock().push(record.clone());
        Ok(())
    }

    fn append_checkpoint(&self, checkpoint: &CheckpointRecord) -> Result<(), LedgerError> {
        self.checkpoints.lock().push(checkpoint.clone());
        Ok(())
    }

    fn load_records(&self) -> Result<Vec<EntryRecord>, LedgerError> {
        Ok(self.records.lock().clone())
    }

    fn load_checkpoints(&self) -> Result<Vec<CheckpointRecord>, LedgerError> {
        Ok(self.checkpoints.lock().clone())
    }
}

/// Simple file-backed append-only storage.
#[derive(Debug, Clone)]
pub struct FileStorage {
    dir: PathBuf,
    wal_path: PathBuf,
    checkpoint_path: PathBuf,
}

impl FileStorage {
    /// Create or open a file-backed storage directory.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, LedgerError> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;
        let wal_path = dir.join("wal.jsonl");
        let checkpoint_path = dir.join("checkpoints.jsonl");
        if !wal_path.exists() {
            File::create(&wal_path)?;
        }
        if !checkpoint_path.exists() {
            File::create(&checkpoint_path)?;
        }
        Ok(Self {
            dir,
            wal_path,
            checkpoint_path,
        })
    }

    /// Directory path used by this storage.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn append_json_line<T: serde::Serialize>(
        &self,
        path: &Path,
        value: &T,
    ) -> Result<(), LedgerError> {
        let bytes = serde_json::to_vec(value)?;
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.flush()?;
        Ok(())
    }

    fn read_json_lines<T: for<'de> serde::Deserialize<'de>>(
        &self,
        path: &Path,
    ) -> Result<Vec<T>, LedgerError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut out = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            out.push(serde_json::from_str(&line)?);
        }
        Ok(out)
    }
}

impl Storage for FileStorage {
    fn append_record(&self, record: &EntryRecord) -> Result<(), LedgerError> {
        self.append_json_line(&self.wal_path, record)
    }

    fn append_checkpoint(&self, checkpoint: &CheckpointRecord) -> Result<(), LedgerError> {
        self.append_json_line(&self.checkpoint_path, checkpoint)
    }

    fn load_records(&self) -> Result<Vec<EntryRecord>, LedgerError> {
        self.read_json_lines(&self.wal_path)
    }

    fn load_checkpoints(&self) -> Result<Vec<CheckpointRecord>, LedgerError> {
        self.read_json_lines(&self.checkpoint_path)
    }
}

impl FileStorage {
    /// Build a storage instance from a path and reject empty directories explicitly.
    pub fn validate_dir(dir: &Path) -> Result<(), LedgerError> {
        if dir.as_os_str().is_empty() {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "storage dir cannot be empty",
            ));
        }
        Ok(())
    }
}
