//! ron-policy: shared policy/quotas/limits library for RustyOnions.
//!
//! Minimal, compile-first stub that always allows, with a typed decision object.
//! Replace the internals with real quotas & config when ready.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub reason: &'static str,
}

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    quotas_enabled: bool,
}

impl PolicyEngine {
    /// Build a default policy engine.
    /// In the future, load from config (env/files/remote) and wire to metrics.
    pub fn new_default() -> Self {
        Self { quotas_enabled: false }
    }

    /// Check whether `principal` may perform `action`.
    /// Replace this with real logic: rate limits, roles, per-node/tenant caps, etc.
    pub fn check(&self, _principal: &str, _action: &str) -> PolicyDecision {
        if self.quotas_enabled {
            // Placeholder path for when quotas are enabled later.
            PolicyDecision { allowed: true, reason: "quotas-enabled: allow (stub)" }
        } else {
            PolicyDecision { allowed: true, reason: "allow-all (stub)" }
        }
    }
}
