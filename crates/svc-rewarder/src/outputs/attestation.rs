//! RO:WHAT — Manifest attestation DTOs for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: GOV/SEC. Keeps signing material as a seam while manifest schema is stable.
//! RO:INTERACTS — outputs::manifest and future ron-kms signing adapters.
//! RO:INVARIANTS — no private keys in DTOs; signatures are optional in batch 1.
//! RO:METRICS — signing failures will be counted by callers in later batches.
//! RO:CONFIG — pq.mode and future signing config control fields.
//! RO:SECURITY — signatures are opaque strings; never log secret key material.
//! RO:TEST — manifest serialization tests through integration path.

use serde::{Deserialize, Serialize};

/// Optional attestation attached to a sealed manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attestation {
    /// Ed25519 signature string, if enabled.
    pub sig_ed25519: Option<String>,
    /// PQ signature string, if enabled.
    pub sig_pq: Option<String>,
    /// Signing timestamp in unix milliseconds.
    pub signed_at_millis: u64,
}
