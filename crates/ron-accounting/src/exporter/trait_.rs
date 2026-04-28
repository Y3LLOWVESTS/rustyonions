//! RO:WHAT — Exporter trait and ACK states for idempotent sealed-slice delivery.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/DX. Core code depends on one tiny integration trait.
//! RO:INTERACTS — worker, router, WAL replay, future svc-wallet/ron-ledger adapter.
//! RO:INVARIANTS — put() is idempotent; Duplicate is a successful terminal ACK state.
//! RO:METRICS — worker records latency/failure around trait calls.
//! RO:CONFIG — deadlines/backoff live in worker/router config.
//! RO:SECURITY — capability checks belong to concrete exporters, not this trait.
//! RO:TEST — unit: exporter_ordering_tests; examples/export_to_mock.rs.

use std::{future::Future, pin::Pin};

use serde::{Deserialize, Serialize};

use crate::{accounting::SealedSlice, errors::Result};

/// Boxed export future that avoids forcing an async-trait dependency on consumers.
pub type BoxExportFuture<'a> = Pin<Box<dyn Future<Output = Result<Ack>> + Send + 'a>>;

/// Idempotent downstream acknowledgement state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum Ack {
    /// Slice was accepted normally.
    Ok,
    /// Slice was already accepted; this is retry-safe success.
    Duplicate,
}

/// Minimal exporter trait for sealed accounting slices.
pub trait Exporter: Send + Sync {
    /// Put a sealed slice. Implementations must treat duplicate `(slice_id,digest)` as idempotent.
    fn put<'a>(&'a self, slice: &'a SealedSlice) -> BoxExportFuture<'a>;
}
