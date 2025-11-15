//! RO:WHAT — Index plane helpers (logical key → content address).
//! RO:WHY  — Provide a small, typed interface for resolving logical
//!           keys (names) to BLAKE3 content IDs.
//! RO:INTERACTS — Will call into `TransportHandle::call_oap` once
//!                wired; uses `SdkMetrics` for latency/retry metrics.
//! RO:INVARIANTS —
//!   - No mutation; this is a read-only plane at the SDK level.
//!   - Deadline is supplied per call by the host.

use std::time::Duration;

use crate::errors::SdkError;
use crate::metrics::SdkMetrics;
use crate::transport::TransportHandle;
use crate::types::{AddrB3, Capability, IndexKey};

/// Resolve a logical `IndexKey` into a content address (`AddrB3`).
pub async fn index_resolve(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    key: &IndexKey,
    deadline: Duration,
) -> Result<AddrB3, SdkError> {
    let _ = (transport, metrics, cap, key, deadline);
    todo!("index_resolve not implemented yet");
}
