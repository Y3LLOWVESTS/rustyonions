//! Simple in-memory `AuditSink` implementation.
//!
//! This is primarily for testing and small deployments; it does not provide
//! durability beyond process lifetime.

use std::collections::HashMap;
use std::sync::RwLock;

use crate::errors::AppendError;
use crate::sink::{AuditSink, AuditStream, ChainState};
use crate::{dto::ChainHeadDto, AuditRecord};

/// In-memory append-only sink, keyed by stream.
#[derive(Debug, Default)]
pub struct RamSink {
    inner: RwLock<HashMap<String, Vec<AuditRecord>>>,
}

impl RamSink {
    /// Create an empty in-memory sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a copy of all records for a stream.
    pub fn records_for(&self, stream: &str) -> Vec<AuditRecord> {
        let guard = self
            .inner
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        guard.get(stream).cloned().unwrap_or_default()
    }

    /// Export a snapshot of all known chain heads.
    ///
    /// This is an in-memory convenience helper intended for:
    /// - admin/diagnostic APIs, and
    /// - tests that need to assert on checkpoint semantics.
    ///
    /// Each entry corresponds to a single logical stream.
    pub fn heads(&self) -> Vec<ChainHeadDto> {
        let guard = self
            .inner
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        guard
            .iter()
            .filter_map(|(stream, records)| {
                records.last().map(|last| ChainHeadDto {
                    stream: stream.clone(),
                    seq: last.seq,
                    head: last.self_hash.clone(),
                })
            })
            .collect()
    }
}

impl AuditStream for RamSink {
    fn state(&self, stream: &str) -> ChainState {
        let guard = self
            .inner
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(records) = guard.get(stream) {
            if let Some(last) = records.last() {
                return ChainState {
                    head: last.self_hash.clone(),
                    seq: last.seq,
                };
            }
        }

        ChainState::default()
    }
}

impl AuditSink for RamSink {
    fn append(&self, rec: &AuditRecord) -> Result<String, AppendError> {
        let mut guard = self
            .inner
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let stream = rec.stream.clone();
        let records = guard.entry(stream.clone()).or_default();

        // Enforce simple append-only linkage rule: prev == last.self_hash.
        if let Some(last) = records.last() {
            if rec.prev != last.self_hash {
                return Err(AppendError::Tamper);
            }
        }

        records.push(rec.clone());
        Ok(rec.self_hash.clone())
    }
}
