//! Core sink traits for append-only audit chains.

use crate::errors::AppendError;
use crate::AuditRecord;

/// Snapshot of a chain head for a given stream.
#[derive(Debug, Clone, Default)]
pub struct ChainState {
    /// Last known `self_hash` at the head of the stream.
    pub head: String,
    /// Last known sequence number for the stream.
    pub seq: u64,
}

/// Read-only view of audit stream state.
pub trait AuditStream: Send + Sync {
    /// Return the current chain state for a given stream.
    fn state(&self, stream: &str) -> ChainState;

    /// Convenience helper: `state(stream).seq + 1`.
    fn next_seq(&self, stream: &str) -> u64 {
        self.state(stream).seq.saturating_add(1)
    }
}

/// Append-only sink for audit records.
///
/// Implementations are expected to:
/// - Enforce `prev/self_hash` consistency.
/// - Enforce monotonic `seq` within a stream.
/// - Provide persistence guarantees appropriate for the deployment.
pub trait AuditSink: Send + Sync {
    /// Append a single record to the sink.
    ///
    /// Implementations typically:
    /// - Validate bounds.
    /// - Validate hash and linkage.
    /// - Commit to WAL / storage.
    fn append(&self, rec: &AuditRecord) -> Result<String, AppendError>;

    /// Flush any buffered data to durable storage.
    fn flush(&self) -> Result<(), AppendError> {
        Ok(())
    }
}
