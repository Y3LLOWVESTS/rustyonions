//! RO:WHAT — Typed error taxonomy (`ProtoError`, `Kind`) with stable reasons.
//! RO:WHY  — Deterministic errors for metrics and control flow; immutable reason strings.
//! RO:TEST — Unit tests assert reason strings remain stable across versions.

mod kind;
mod reason;

pub use kind::Kind;
pub use reason::stable_reason;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProtoError {
    pub kind: Kind,
    pub message: String,
}

impl ProtoError {
    pub fn metric_reason(&self) -> &'static str {
        stable_reason(self.kind)
    }
}
