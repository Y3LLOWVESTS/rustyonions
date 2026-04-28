//! RO:WHAT — Security module facade for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: SEC/GOV. Separates capabilities, TLS, and PQ posture.
//! RO:INTERACTS — HTTP handlers, config, future KMS/wallet clients.
//! RO:INVARIANTS — capabilities only; no ambient trust; no external-chain logic.
//! RO:METRICS — auth rejects counted by handlers.
//! RO:CONFIG — tls and pq configs.
//! RO:SECURITY — this module owns security seams.
//! RO:TEST — HTTP and config tests.

pub mod caps;
pub mod pq;
pub mod tls;
