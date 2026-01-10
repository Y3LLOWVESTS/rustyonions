//! RO:WHAT — Admin HTTP handlers for Macronode.
//! RO:WHY  — Keep handler modules discoverable and explicitly exported.
//! RO:INVARIANTS —
//!   - Router references must resolve here.
//!   - “Dev-only” helpers are auth-guarded; safe to keep compiled.

#![forbid(unsafe_code)]

pub mod healthz;
pub mod metrics;
pub mod readyz;
pub mod version;

pub mod reload;
pub mod shutdown;
pub mod status;

// Storage inventory (read-only, curated).
pub mod storage;

// System summary (CPU/RAM + optional network rate).
pub mod system_summary;

// Benchmarks (node-executed, bounded).
pub mod bench;

// Dev-only debug helpers (feature-safe to leave compiled; guarded by auth).
pub mod debug_crash;

pub mod system_net_accounting;
