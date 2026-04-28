//! RO:WHAT — Bounded router from sealed slices into per-stream export lanes.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/PERF. Preserves order while allowing fair draining.
//! RO:INTERACTS — ExportLane, SealedSlice, worker, config::ExporterConfig.
//! RO:INVARIANTS — per-stream ordering; bounded lanes; deterministic stream ordering.
//! RO:METRICS — callers expose backlog depth and shed/order overflow counts.
//! RO:CONFIG — exporter.ordered_buffer_cap.
//! RO:SECURITY — no secrets.
//! RO:TEST — unit: exporter_ordering_tests; loom router model later.

use std::collections::BTreeMap;

use crate::{
    accounting::SealedSlice,
    errors::Result,
    exporter::lane::{ExportLane, StreamKey},
};

/// Ordered exporter router keyed by `(tenant,dimension)`.
#[derive(Debug, Clone)]
pub struct ExporterRouter {
    lane_cap: usize,
    lanes: BTreeMap<StreamKey, ExportLane>,
}

impl ExporterRouter {
    /// Create a router with per-lane capacity.
    pub fn new(lane_cap: usize) -> Self {
        Self {
            lane_cap: lane_cap.max(1),
            lanes: BTreeMap::new(),
        }
    }

    /// Route a sealed slice into the correct ordered lane.
    pub fn route(&mut self, slice: SealedSlice) -> Result<()> {
        let key = StreamKey::from(&slice);
        let first_seq = slice.id.seq;
        self.lanes
            .entry(key)
            .or_insert_with(|| ExportLane::new(key, self.lane_cap, first_seq))
            .push(slice)
    }

    /// Lease one slice from the first non-inflight stream in deterministic key order.
    pub fn lease_next(&mut self) -> Option<SealedSlice> {
        for lane in self.lanes.values_mut() {
            if let Some(slice) = lane.lease_next() {
                return Some(slice);
            }
        }
        None
    }

    /// Compatibility alias for Batch 1 callers.
    ///
    /// This now leases instead of destructively popping so failed exports cannot
    /// lose a sealed slice.
    pub fn pop_next(&mut self) -> Option<SealedSlice> {
        self.lease_next()
    }

    /// Mark a stream sequence as ACKed and remove it from its lane.
    pub fn ack(&mut self, key: StreamKey, seq: u64) -> Result<()> {
        if let Some(lane) = self.lanes.get_mut(&key) {
            lane.ack(seq)?;
        }
        Ok(())
    }

    /// Mark a leased stream sequence as failed so it may be retried.
    pub fn nack(&mut self, key: StreamKey, seq: u64) -> Result<()> {
        if let Some(lane) = self.lanes.get_mut(&key) {
            lane.nack(seq)?;
        }
        Ok(())
    }

    /// Total queued slices across all lanes, including in-flight front items.
    pub fn backlog_len(&self) -> usize {
        self.lanes.values().map(ExportLane::len).sum()
    }

    /// Number of active lanes retained for sequence continuity.
    pub fn lane_count(&self) -> usize {
        self.lanes.len()
    }

    /// Number of lanes with one leased in-flight slice.
    pub fn inflight_len(&self) -> usize {
        self.lanes
            .values()
            .filter(|lane| lane.has_inflight())
            .count()
    }

    /// Return true when no slices are queued or in flight.
    pub fn is_empty(&self) -> bool {
        self.backlog_len() == 0 && self.inflight_len() == 0
    }
}
