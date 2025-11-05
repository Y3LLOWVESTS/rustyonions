//! RO:WHAT — Error types for Micronode (foundation).
//! RO:WHY  — Stable envelopes and anyhow interop.
//! RO:INVARIANTS — Avoid leaking secrets; messages deterministic for SDKs.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("config: {0}")]
    Config(String),
    #[error("internal error")]
    Internal,
}

pub type Result<T> = std::result::Result<T, Error>;
