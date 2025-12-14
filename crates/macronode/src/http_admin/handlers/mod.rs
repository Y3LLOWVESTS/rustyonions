//! RO:WHAT — Admin HTTP handlers for Macronode.

pub mod healthz;
pub mod metrics;
pub mod reload;
pub mod shutdown;
pub mod status;
pub mod version;

// Dev-only debug helpers (feature-safe to leave compiled; guarded by auth).
pub mod debug_crash;
pub mod readyz;