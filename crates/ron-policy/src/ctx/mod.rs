//! RO:WHAT — Evaluation context types (normalized request facts) and clock trait.
//!
//! RO:WHY  — Deterministic, testable evaluation independent of actual services.
//!
//! RO:INTERACTS — `engine::eval` (consumes `Context`), `ctx::{normalize,clock}`

pub mod clock;
pub mod normalize;

use std::collections::BTreeSet;

/// Minimal context the engine needs to decide.
#[derive(Debug, Clone)]
pub struct Context {
    pub tenant: String,
    pub method: String,
    pub region: String,
    pub body_bytes: u64,
    pub tags: BTreeSet<String>,
    pub now_ms: u64,
}

impl Context {
    #[must_use]
    pub fn builder() -> normalize::ContextBuilder {
        normalize::ContextBuilder::default()
    }
}
