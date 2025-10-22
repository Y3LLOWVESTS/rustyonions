//! RO:WHAT — Centralized constants for kernel tuning and invariants.
//! RO:WHY  — Keep perf/backpressure and retry limits consistent with blueprints (avoid drift).
//! RO:INTERACTS — bus capacity (bus::Bus), supervisor backoff (supervisor), readiness (metrics).
//! RO:INVARIANTS — bounded queues; backoff caps; no unbounded growth anywhere.

/// Default broadcast capacity per sender (bounded).
#[allow(dead_code)]
pub const DEFAULT_BUS_CAPACITY: usize = 4096;

/// Supervisor backoff: initial delay in milliseconds.
pub const SUP_BACKOFF_MS_START: u64 = 100;

/// Supervisor backoff cap in milliseconds (jittered up to this).
pub const SUP_BACKOFF_MS_CAP: u64 = 30_000;
