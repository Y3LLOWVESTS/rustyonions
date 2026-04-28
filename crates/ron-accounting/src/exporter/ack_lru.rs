//! RO:WHAT — Small bounded replay cache for exported `(slice_id,digest)` ACK keys.
//! RO:WHY — Pillar 12; Concerns: ECON/RES. Makes retries duplicate-safe without unbounded memory.
//! RO:INTERACTS — exporter worker/router, WAL replay, SealedSlice IDs.
//! RO:INVARIANTS — bounded capacity; insert returns false for duplicates; oldest evicted first.
//! RO:METRICS — callers may expose len/capacity as backlog gauges.
//! RO:CONFIG — capacity chosen by host/exporter policy.
//! RO:SECURITY — stores digests and stream IDs, no secrets.
//! RO:TEST — unit: exporter_ordering_tests.

use std::collections::{HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::accounting::{SealedSlice, SliceId};

/// Idempotency key for a sealed slice ACK.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AckKey {
    /// Slice stream identity.
    pub id: SliceId,
    /// Slice digest.
    pub digest: String,
}

impl AckKey {
    /// Construct a new ACK key.
    pub fn new(id: SliceId, digest: impl Into<String>) -> Self {
        Self {
            id,
            digest: digest.into(),
        }
    }

    /// Build an ACK key from a sealed slice.
    pub fn from_slice(slice: &SealedSlice) -> Self {
        Self {
            id: slice.id.clone(),
            digest: slice.digest.clone(),
        }
    }
}

/// Bounded insertion-ordered ACK cache.
#[derive(Debug, Clone)]
pub struct AckLru {
    cap: usize,
    order: VecDeque<AckKey>,
    set: HashSet<AckKey>,
}

impl AckLru {
    /// Create an ACK cache with a fixed capacity.
    pub fn new(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            order: VecDeque::new(),
            set: HashSet::new(),
        }
    }

    /// Insert a key; returns false when it was already present.
    pub fn insert(&mut self, key: AckKey) -> bool {
        if self.set.contains(&key) {
            return false;
        }

        if self.order.len() >= self.cap {
            if let Some(oldest) = self.order.pop_front() {
                self.set.remove(&oldest);
            }
        }

        self.order.push_back(key.clone());
        self.set.insert(key);
        true
    }

    /// Insert a sealed slice by its `(id,digest)` ACK key.
    pub fn insert_slice(&mut self, slice: &SealedSlice) -> bool {
        self.insert(AckKey::from_slice(slice))
    }

    /// Return true when the key is already present.
    pub fn contains(&self, key: &AckKey) -> bool {
        self.set.contains(key)
    }

    /// Return true when a slice's `(id,digest)` ACK key is already present.
    pub fn contains_slice(&self, slice: &SealedSlice) -> bool {
        self.contains(&AckKey::from_slice(slice))
    }

    /// Remove all ACK keys.
    pub fn clear(&mut self) {
        self.order.clear();
        self.set.clear();
    }

    /// Current cache length.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Return true when the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Maximum retained ACK keys.
    pub fn capacity(&self) -> usize {
        self.cap
    }
}
