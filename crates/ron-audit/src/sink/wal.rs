//! Placeholder WAL-backed sink implementation.
//!
//! This module is feature-gated behind `wal` and currently provides a
//! minimal, non-durable implementation that behaves like `RamSink`.
//!
//! A real WAL-backed sink can replace this without breaking the public
//! trait surface.

#[cfg(feature = "wal")]
use std::sync::Arc;

#[cfg(feature = "wal")]
use crate::sink::{AuditSink, AuditStream, ChainState};
#[cfg(feature = "wal")]
use crate::{errors::AppendError, AuditRecord};

/// Minimal WAL sink placeholder.
///
/// Internally this just forwards to an in-memory `RamSink`. The type is
/// present so that higher layers can experiment with the `wal` feature
/// flag without breaking compilation.
#[cfg(feature = "wal")]
#[derive(Debug, Default)]
pub struct WalSink {
    inner: Arc<crate::sink::ram::RamSink>,
}

#[cfg(feature = "wal")]
impl WalSink {
    /// Construct a new placeholder WAL sink.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(crate::sink::ram::RamSink::new()),
        }
    }
}

#[cfg(feature = "wal")]
impl AuditStream for WalSink {
    fn state(&self, stream: &str) -> ChainState {
        self.inner.state(stream)
    }
}

#[cfg(feature = "wal")]
impl AuditSink for WalSink {
    fn append(&self, rec: &AuditRecord) -> Result<String, AppendError> {
        self.inner.append(rec)
    }

    fn flush(&self) -> Result<(), AppendError> {
        self.inner.flush()
    }
}
