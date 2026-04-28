//! RO:WHAT — Bounded ordered queue for one `(tenant,dimension)` sealed-slice stream.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/PERF. Prevents sequence gaps and unbounded backlog.
//! RO:INTERACTS — exporter::router, exporter::worker, accounting::SealedSlice.
//! RO:INVARIANTS — stream key matches slice ID; queue is contiguous from next_seq; bounded cap.
//! RO:METRICS — callers expose backlog depth and order overflow counters.
//! RO:CONFIG — exporter.ordered_buffer_cap.
//! RO:SECURITY — no secrets.
//! RO:TEST — unit: exporter_ordering_tests.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::{
    accounting::{Dimension, SealedSlice, TenantId},
    errors::{Error, Result},
};

/// Export stream key. Ordering is enforced per stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StreamKey {
    /// Tenant stream owner.
    pub tenant: TenantId,
    /// Metered dimension stream.
    pub dimension: Dimension,
}

impl StreamKey {
    /// Construct a stream key.
    pub fn new(tenant: TenantId, dimension: Dimension) -> Self {
        Self { tenant, dimension }
    }
}

impl From<&SealedSlice> for StreamKey {
    fn from(value: &SealedSlice) -> Self {
        Self {
            tenant: value.id.tenant,
            dimension: value.id.dimension,
        }
    }
}

/// Bounded queue that only accepts contiguous sequence numbers.
#[derive(Debug, Clone)]
pub struct ExportLane {
    key: StreamKey,
    cap: usize,
    next_seq: u64,
    queue: VecDeque<SealedSlice>,
    inflight_seq: Option<u64>,
}

impl ExportLane {
    /// Create a lane with the expected first sequence number.
    pub fn new(key: StreamKey, cap: usize, next_seq: u64) -> Self {
        Self {
            key,
            cap: cap.max(1),
            next_seq,
            queue: VecDeque::new(),
            inflight_seq: None,
        }
    }

    /// Push a slice if it belongs to this stream and preserves contiguous order.
    pub fn push(&mut self, slice: SealedSlice) -> Result<()> {
        if StreamKey::from(&slice) != self.key {
            return Err(Error::schema("slice stream does not match export lane"));
        }
        if self.queue.len() >= self.cap {
            return Err(Error::OrderOverflow);
        }

        let expected = self.next_seq.saturating_add(self.queue.len() as u64);
        if slice.id.seq < expected {
            return Err(Error::DuplicateExport);
        }
        if slice.id.seq != expected {
            return Err(Error::OrderOverflow);
        }

        self.queue.push_back(slice);
        Ok(())
    }

    /// Lease the next ordered slice without removing it.
    ///
    /// The leased slice stays in the lane until `ack(seq)` removes it or
    /// `nack(seq)` clears the in-flight marker so it can be retried.
    pub fn lease_next(&mut self) -> Option<SealedSlice> {
        if self.inflight_seq.is_some() {
            return None;
        }

        let slice = self.queue.front()?.clone();
        self.inflight_seq = Some(slice.id.seq);
        Some(slice)
    }

    /// Compatibility alias for callers that used the Batch 1 name.
    ///
    /// Batch 2 intentionally makes this a lease, not a destructive pop, so failed
    /// exports cannot lose a sealed slice.
    pub fn pop_next(&mut self) -> Option<SealedSlice> {
        self.lease_next()
    }

    /// Mark a sequence as ACKed, removing it from the ordered queue.
    pub fn ack(&mut self, seq: u64) -> Result<()> {
        if let Some(inflight) = self.inflight_seq {
            if inflight != seq {
                return Err(Error::OrderOverflow);
            }
        }

        let front = self.queue.front().ok_or(Error::OrderOverflow)?;
        if front.id.seq != seq || seq != self.next_seq {
            return Err(Error::OrderOverflow);
        }

        self.queue.pop_front();
        self.inflight_seq = None;
        self.next_seq = self.next_seq.saturating_add(1);
        Ok(())
    }

    /// Mark a sequence as failed so it may be retried.
    pub fn nack(&mut self, seq: u64) -> Result<()> {
        match self.inflight_seq {
            Some(inflight) if inflight == seq => {
                self.inflight_seq = None;
                Ok(())
            }
            Some(_) => Err(Error::OrderOverflow),
            None => Ok(()),
        }
    }

    /// Current queued slice count, including an in-flight front item.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Return true when the lane has no queued slices.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Return true when this lane has one leased slice awaiting ACK/NACK.
    pub fn has_inflight(&self) -> bool {
        self.inflight_seq.is_some()
    }

    /// Return the current in-flight sequence number, if any.
    pub fn inflight_seq(&self) -> Option<u64> {
        self.inflight_seq
    }

    /// Stream key.
    pub fn key(&self) -> StreamKey {
        self.key
    }

    /// Next expected sequence.
    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }
}
