//! RO:WHAT — PQ posture helpers for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: SEC/GOV. Keeps hybrid/PQ downgrade policy explicit without wiring crypto yet.
//! RO:INTERACTS — config::PqConfig, readiness, future KMS attestation.
//! RO:INVARIANTS — unsupported PQ modes fail config validation; no silent downgrade when pq-only arrives.
//! RO:METRICS — future pq_negotiated labels.
//! RO:CONFIG — pq.mode.
//! RO:SECURITY — posture only; no key handling.
//! RO:TEST — config validation.

use crate::config::PqConfig;

/// True when hybrid PQ posture is requested.
#[must_use]
pub fn pq_hybrid_requested(cfg: &PqConfig) -> bool {
    cfg.mode == "hybrid"
}
