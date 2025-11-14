//! Streaming helpers for audit sinks.
//!
//! For the initial seed we keep this extremely small; hosts can build
//! richer batching / mpsc-based streaming layers on top.

use crate::errors::AppendError;
use crate::sink::AuditSink;
use crate::AuditRecord;

/// A simple buffered sink wrapper that collects records and flushes them
/// in a single batch call.
///
/// The current implementation just forwards records one-by-one; it
/// exists mainly to give tests somewhere to hang future stream logic.
#[derive(Debug)]
pub struct BufferedSink<S> {
    inner: S,
}

impl<S> BufferedSink<S> {
    /// Wrap an existing sink.
    pub fn new(inner: S) -> Self {
        Self { inner }
    }

    /// Access the inner sink.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S> BufferedSink<S>
where
    S: AuditSink,
{
    /// Append all given records, stopping on the first error.
    pub fn append_all(&self, records: &[AuditRecord]) -> Result<(), AppendError> {
        for rec in records {
            self.inner.append(rec)?;
        }
        self.inner.flush()
    }
}
