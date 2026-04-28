//! RO:WHAT — Library DTOs for ingest/roots request-response flows and typed reject items.
//! RO:WHY  — Pillar 12; Concerns: ECON/DX/GOV. Keep future service wrappers aligned with a stable internal schema.
//! RO:INTERACTS — crate::types, crate::error, crate::engine.
//! RO:INVARIANTS — DTOs are serde-friendly; reject reasons stay typed; no transport/runtime coupling leaks in.
//! RO:METRICS — none directly.
//! RO:CONFIG — size caps are enforced by the engine using LedgerConfig.
//! RO:SECURITY — capability refs and KIDs are identifiers only; no secret-bearing transport wrappers here.
//! RO:TEST — interop_vectors.rs and reject_taxonomy.rs validate round-trips and stable reason mapping.

use crate::{
    error::RejectReason,
    types::{Entry, Root, Seq},
};

/// Batch ingest request.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IngestRequest {
    /// Entries to commit in order.
    pub batch: Vec<Entry>,
    /// Optional batch-level idempotency identifier.
    pub idem_id: Option<String>,
}

/// Indexed reject item for partial/failed ingestion.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RejectItem {
    /// Entry index in the request batch.
    pub idx: usize,
    /// Stable reject reason.
    pub reason: RejectReason,
}

/// Batch ingest response.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IngestResponse {
    /// True when the batch was accepted and committed.
    pub accepted: bool,
    /// First sequence assigned when accepted.
    pub seq_start: Option<Seq>,
    /// Last sequence assigned when accepted.
    pub seq_end: Option<Seq>,
    /// Head root after processing.
    pub new_root: Root,
    /// Reject items when not accepted.
    pub reasons: Vec<RejectItem>,
}

/// Root publication item.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootItem {
    /// Sequence of the root.
    pub seq: Seq,
    /// Root bytes.
    pub root: Root,
    /// Unix millis at publication time.
    pub ts: u64,
}

/// Paginated roots response.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootsResponse {
    /// Ordered root items.
    pub roots: Vec<RootItem>,
    /// Next sequence cursor.
    pub next: Seq,
}
