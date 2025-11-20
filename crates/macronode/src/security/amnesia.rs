//! RO:WHAT — Amnesia posture helpers for Macronode.
//! RO:WHY  — Provide a single source of truth for how "amnesia mode" is
//!           interpreted so config / CLI / services stay consistent.
//! RO:INVARIANTS —
//!   - `Persistent` is the default for Macronode (unlike Micronode).
//!   - `Amnesic` is a best-effort "RAM-first, no durable residue" posture.

#![allow(dead_code)]

/// High-level amnesia posture for this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmnesiaMode {
    /// Normal mode: services are allowed to persist state to disk.
    Persistent,
    /// Best-effort amnesia: avoid durable state, prefer RAM-only caches.
    Amnesic,
}

impl AmnesiaMode {
    /// Returns true if the node should avoid writing persistent state.
    #[must_use]
    pub const fn is_amnesic(self) -> bool {
        matches!(self, AmnesiaMode::Amnesic)
    }
}

/// Classify the mode from a simple boolean flag (e.g. config/CLI/env).
///
/// This keeps the rest of the code from re-encoding the boolean semantics
/// in multiple places.
#[must_use]
pub fn classify_amnesia(enabled: bool) -> AmnesiaMode {
    if enabled {
        AmnesiaMode::Amnesic
    } else {
        AmnesiaMode::Persistent
    }
}
