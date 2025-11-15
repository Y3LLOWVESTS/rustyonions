//! RO:WHAT — Index plane helpers (logical key → content address).
//! RO:WHY  — Provide a small, typed interface for resolving logical
//!           keys (names) to BLAKE3 content IDs.
//! RO:INTERACTS — Will call into `TransportHandle::call_oap` once
//!                wired; uses `SdkMetrics` for latency/retry metrics.
//! RO:INVARIANTS —
//!   - No mutation; this is a read-only plane at the SDK level.
//!   - Deadline is supplied per call by the host.

use std::time::{Duration, Instant};

use crate::errors::SdkError;
use crate::metrics::SdkMetrics;
use crate::transport::TransportHandle;
use crate::types::{AddrB3, Capability, IndexKey};

/// Resolve a logical `IndexKey` into a content address (`AddrB3`).
///
/// Beta note:
/// For now this is a **safe stub**:
///   - It enforces that `deadline > 0`.
///   - It records a metrics failure event.
///   - It returns a structured `SdkError::Unknown` instead of panicking.
///
/// Once the OAP/1 index surface is wired, this function will:
///   - build the appropriate OAP request,
///   - call `TransportHandle::call_oap(...)`,
///   - map wire DTOs into `AddrB3`,
///   - and surface precise `SdkError` variants (NotFound, CapabilityDenied, etc.).
pub async fn index_resolve(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    key: &IndexKey,
    deadline: Duration,
) -> Result<AddrB3, SdkError> {
    // We don't use these yet; keep the binding to avoid “unused” warnings
    // until the real transport wiring lands.
    let _ = (transport, cap, key);

    // Enforce a non-zero deadline at the SDK boundary so callers don't
    // accidentally issue "no deadline" calls.
    if deadline.as_millis() == 0 {
        return Err(SdkError::schema_violation(
            "index_resolve.deadline",
            "deadline must be > 0",
        ));
    }

    // Minimal observability stub: record that the call path was hit
    // and that it failed due to being unimplemented.
    let started = Instant::now();
    let endpoint = "/index/resolve";

    // Mark as failure for now — this keeps perf/metrics dashboards
    // honest while the plane is still a stub.
    metrics.inc_failure(endpoint, "unimplemented");
    metrics.observe_latency(endpoint, false, started.elapsed().as_millis() as u64);

    Err(SdkError::Unknown(
        "index_resolve not implemented yet".to_string(),
    ))
}
