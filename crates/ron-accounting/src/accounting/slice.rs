//! RO:WHAT — Immutable sealed accounting slice types and digest computation.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/RES. Snapshots become committed rewarder inputs.
//! RO:INTERACTS — recorder, exporter, WAL, utils::{encode,hashing}, svc-rewarder.
//! RO:INVARIANTS — rows sorted before seal; digest over zero-field preimage; no mutation after seal.
//! RO:METRICS — sealing callers increment accounting_slices_sealed_total.
//! RO:CONFIG — meta.amnesia records node posture; encoding cap is 1MiB.
//! RO:SECURITY — full b3 digest, no truncated IDs.
//! RO:TEST — unit: recording_tests; prop: encoding_prop.

use serde::{Deserialize, Serialize};

use crate::{
    accounting::{Dimension, LabelSet, TenantId, Window},
    errors::Result,
    utils::{encode::to_canonical_bytes, hashing::b3_hex},
};

/// Stable identity for an ordered sealed slice stream.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SliceId {
    /// Tenant stream owner.
    pub tenant: TenantId,
    /// Metering dimension for this stream.
    pub dimension: Dimension,
    /// Monotone sequence number per `(tenant, dimension)`.
    pub seq: u64,
}

/// Metadata committed into every sealed slice digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceMeta {
    /// Slice schema version.
    pub version: u16,
    /// Inclusive window start in Unix milliseconds.
    pub window_start_ms: u64,
    /// Exclusive window end in Unix milliseconds.
    pub window_end_ms: u64,
    /// Seal timestamp in Unix milliseconds.
    pub sealed_at_ms: u64,
    /// Previous slice digest for the stream, when known.
    pub prev_b3: Option<String>,
    /// True when the node was running in amnesia mode while sealing.
    pub amnesia: bool,
}

impl SliceMeta {
    /// Build metadata for a new sealed slice.
    pub fn new(window: Window, sealed_at_ms: u64, prev_b3: Option<String>, amnesia: bool) -> Self {
        Self {
            version: 1,
            window_start_ms: window.start_ms,
            window_end_ms: window.end_ms,
            sealed_at_ms,
            prev_b3,
            amnesia,
        }
    }
}

/// A normalized counter row captured into a sealed slice.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SliceRow {
    /// Normalized usage labels.
    pub labels: LabelSet,
    /// Metered dimension.
    pub dimension: Dimension,
    /// Non-negative counter value for this window.
    pub value: u64,
}

/// Immutable sealed usage slice with canonical digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedSlice {
    /// Ordered stream identity.
    pub id: SliceId,
    /// Immutable slice metadata.
    pub meta: SliceMeta,
    /// Sorted usage rows.
    pub rows: Vec<SliceRow>,
    /// Canonical `b3:<hex>` digest over the slice preimage.
    pub digest: String,
}

#[derive(Serialize)]
struct SlicePreimage<'a> {
    id: &'a SliceId,
    meta: &'a SliceMeta,
    rows: &'a [SliceRow],
}

impl SealedSlice {
    /// Construct a sealed slice and compute its canonical digest.
    pub fn new(id: SliceId, meta: SliceMeta, mut rows: Vec<SliceRow>) -> Result<Self> {
        rows.sort();
        let preimage = SlicePreimage {
            id: &id,
            meta: &meta,
            rows: &rows,
        };
        let bytes = to_canonical_bytes(&preimage)?;
        let digest = b3_hex(&bytes);
        Ok(Self {
            id,
            meta,
            rows,
            digest,
        })
    }

    /// Return the canonical digest string.
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Return true when this slice has no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}
