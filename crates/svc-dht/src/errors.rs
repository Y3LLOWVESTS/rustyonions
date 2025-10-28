//! RO:WHAT — Error taxonomy for svc-dht with user hints
//! RO:WHY — Deterministic, typed errors; Concerns: DX/GOV/SEC
//! RO:INTERACTS — rpc/http, pipeline, provider store
//! RO:INVARIANTS — stable Display; avoid leaking internals
//! RO:TEST — unit tests for Display and status mapping

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DhtError {
    #[error("bootstrap quorum not reached")]
    NoBootstrap,
    #[error("asn diversity floor not met")]
    AsnCap,
    #[error("payload oversize")]
    OverSize,
    #[error("hop budget exceeded")]
    HopBudget,
    #[error("timeout")]
    Timeout,
    #[error("internal: {0}")]
    Internal(String),
}
