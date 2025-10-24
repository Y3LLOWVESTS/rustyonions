//! RO:WHAT — SignedDescriptorV1 header (detached signature carried elsewhere).
//! RO:WHY  — Replay-safe governance inputs; PQ-agile via quantum tags.
//! RO:INVARIANTS — deny_unknown_fields; deterministic field order.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultiSigNofM {
    pub n: u8,
    pub m: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SignedDescriptorV1 {
    pub descriptor_cid: crate::id::ContentId,
    pub alg: crate::quantum::SignatureAlg,
    pub quorum: MultiSigNofM,
    pub issued_at: u64,
    pub expires_at: u64,
    /// CID of rationale or supplemental evidence
    pub rationale_cid: Option<crate::id::ContentId>,
}
