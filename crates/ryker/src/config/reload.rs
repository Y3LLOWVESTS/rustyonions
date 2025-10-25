//! RO:WHAT — Config reload hook trait and counters.
//! RO:WHY  — Hosts may hot-apply fairness/deadline; capacity is cold-only.
//! RO:INTERACTS — runtime applies hooks; observe increments counters.
//! RO:INVARIANTS — reloads are atomic snapshot swaps; diffs are redacted.

use std::sync::atomic::AtomicU64;

#[derive(Default)]
pub struct ReloadCounters {
    pub total: AtomicU64,
    pub errors: AtomicU64,
}

pub trait RykerReloadHook: Send + Sync + 'static {
    /// Apply a new effective snapshot. Implementations must be fast and panic-free.
    fn apply(&self);
}
