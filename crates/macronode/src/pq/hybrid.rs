//! RO:WHAT — Stub types for hybrid PQ keying configuration.
//! RO:WHY  — Keep the *shape* of PQ integration stable while we defer
//!           the actual cryptography and transport/KMS wiring to other
//!           crates (ron-transport, ron-kms, svc-gateway, svc-overlay).
//!
//! RO:STATUS —
//!   - Foundation slice; no crypto, no external dependencies.
//!   - Safe to extend once PQ libraries and envelopes are chosen.
//!
//! RO:INVARIANTS —
//!   - This module is purely descriptive and carries no secrets.
//!   - Default configuration is conservative (no PQ suite selected).

/// Logical identifier for a PQ-capable hybrid algorithm suite.
///
/// Foundation cut keeps this stringly-typed; once we lock in a concrete
/// set of KEM/sign combos (e.g. "x25519+mlkem768"), we can substitute a
/// richer enum or newtype.
pub type SuiteId = &'static str;

/// Minimal configuration describing which PQ suite (if any) this node
/// prefers for **hybrid** operation.
///
/// This is intentionally small and self-contained. Downstream crates
/// will decide how to interpret and enforce it at the transport and
/// KMS layers.
#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// Selected hybrid suite identifier (e.g. "x25519+mlkem768").
    ///
    /// `None` means "no PQ preference" and is the current default.
    pub suite: Option<SuiteId>,
}

impl Default for HybridConfig {
    fn default() -> Self {
        // Foundation: do not force PQ on; macronode will remain classical
        // until operators enable PQ at the edges and downstream planes
        // are wired to support it.
        Self { suite: None }
    }
}
