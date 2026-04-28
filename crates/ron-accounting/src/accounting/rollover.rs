//! RO:WHAT — Boundary detection helper for fixed-window accounting rollover.
//! RO:WHY — Pillar 12; Concerns: ECON/RES. Prevents duplicate seals and rollover drift.
//! RO:INTERACTS — Window, Recorder, SliceId, SealedSlice, future background ticker.
//! RO:INVARIANTS — one rollover per crossed boundary; caller owns sequence allocation.
//! RO:METRICS — callers increment boundary tick and sealed slice counters.
//! RO:CONFIG — accounting.window_len_s.
//! RO:SECURITY — no secrets.
//! RO:TEST — unit: rollover_tests.

use serde::{Deserialize, Serialize};

use crate::{
    accounting::{Dimension, Recorder, SealedSlice, SliceId, TenantId, Window},
    errors::Result,
};

/// Rollover decision after observing a timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RolloverDecision {
    /// The timestamp remains inside the current window.
    Stay(Window),
    /// The timestamp moved into a later window; seal the previous window once.
    Rollover { previous: Window, next: Window },
}

/// Request parameters used when sealing a slice during rollover.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RolloverSealRequest {
    /// Tenant whose stream is being sealed.
    pub tenant: TenantId,
    /// Counter dimension being sealed.
    pub dimension: Dimension,
    /// Monotonic sequence number assigned by the caller/export stream.
    pub seq: u64,
    /// Previous slice digest for optional chained audit context.
    pub prev_b3: Option<String>,
    /// Whether the sealed artifact should be treated as amnesia-mode data.
    pub amnesia: bool,
}

/// Manual rollover handle for hosts that drive accounting from their own ticker.
#[derive(Debug, Clone, Copy)]
pub struct RolloverHandle {
    current: Window,
}

impl RolloverHandle {
    /// Create a handle for the window containing `timestamp_ms`.
    pub fn new(timestamp_ms: u64, window_len_s: u32) -> Result<Self> {
        Ok(Self {
            current: Window::for_timestamp_ms(timestamp_ms, window_len_s)?,
        })
    }

    /// Return the currently active window.
    pub fn current(&self) -> Window {
        self.current
    }

    /// Observe a timestamp and return the rollover decision.
    pub fn observe(&mut self, timestamp_ms: u64) -> Result<RolloverDecision> {
        if self.current.contains(timestamp_ms) {
            return Ok(RolloverDecision::Stay(self.current));
        }

        let previous = self.current;
        let next = Window::for_timestamp_ms(timestamp_ms, self.current.len_s)?;
        self.current = next;
        Ok(RolloverDecision::Rollover { previous, next })
    }

    /// Seal a stream when `timestamp_ms` crosses a boundary; otherwise return `None`.
    pub fn seal_if_due(
        &mut self,
        recorder: &Recorder,
        timestamp_ms: u64,
        request: RolloverSealRequest,
    ) -> Result<Option<SealedSlice>> {
        match self.observe(timestamp_ms)? {
            RolloverDecision::Stay(_) => Ok(None),
            RolloverDecision::Rollover { previous, .. } => {
                let id = SliceId {
                    tenant: request.tenant,
                    dimension: request.dimension,
                    seq: request.seq,
                };
                Ok(Some(recorder.seal_slice(
                    id,
                    previous,
                    request.prev_b3,
                    request.amnesia,
                )?))
            }
        }
    }
}
