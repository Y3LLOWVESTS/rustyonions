//! RO:WHAT — Read-only vs Mutate operation gating.
//! RO:WHY  — Allow policy/rate-limit paths to key off operation class.
//! RO:INVARIANTS — Pure classification; no IO; easy to unit test.

/// Operation class used by admissions/policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpClass {
    ReadOnly,
    Mutate,
}

/// Guard object carried in request extensions to mark op class.
#[derive(Debug, Clone, Copy)]
pub struct OpGuard {
    class: OpClass,
}

impl OpGuard {
    pub fn new(class: OpClass) -> Self {
        Self { class }
    }
    pub fn class(&self) -> OpClass {
        self.class
    }
}
