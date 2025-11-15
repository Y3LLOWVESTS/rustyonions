//! RO:WHAT — Lightweight SDK context metadata.
//! RO:WHY  — Give callers a tiny “who am I talking to?” view without
//!           ever branching semantics based on profile (see I-1).
//! RO:INTERACTS — Constructed in `RonAppSdk::new`, may be enriched
//!                later when we add a real handshake.
//! RO:INVARIANTS —
//!   - No behavior decisions are made based on `NodeProfile`.
//!   - `amnesia` is a *hint* surfaced to hosts, not a control plane.

/// Logical deployment profile of the remote node.
///
/// This is intentionally small and matches the high-level Micronode /
/// Macronode split in the RON-CORE blueprints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeProfile {
    /// Micronode profile (amnesia-friendly, edge-first).
    Micronode,
    /// Macronode profile (persistent overlay / services).
    Macronode,
}

/// Small, immutable view of SDK context.
///
/// Today this is constructed from configuration + (eventually) a small
/// handshake. It is **never** used to branch semantics — that’s the
/// whole point of invariant I-1 (profile parity).
#[derive(Debug, Clone, Copy)]
pub struct SdkContext {
    /// Reported/assumed node profile (Micronode/Macronode).
    pub profile: NodeProfile,
    /// Whether the remote node is currently in “amnesia mode”.
    ///
    /// For Micronodes this often maps to “RAM-only” posture; for
    /// Macronodes this may indicate a temporary override.
    pub amnesia: bool,
}

impl SdkContext {
    /// Construct a new SDK context.
    pub fn new(profile: NodeProfile, amnesia: bool) -> Self {
        Self { profile, amnesia }
    }

    /// Get the node profile (Micronode/Macronode).
    pub fn profile(&self) -> NodeProfile {
        self.profile
    }

    /// Whether the node is in amnesia posture.
    pub fn amnesia(&self) -> bool {
        self.amnesia
    }
}
