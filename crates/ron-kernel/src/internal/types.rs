//! RO:WHAT — Internal shared type aliases and small enums.
//! RO:WHY  — Reduces duplication and drift across modules; GOV/RES concern.
//! RO:INTERACTS — supervisor, metrics, bus, readiness, config.
//! RO:INVARIANTS — No heavy deps; stable aliases only; no cross-await locks introduced.
//! RO:METRICS/LOGS — N/A.
//! RO:CONFIG — N/A.
//! RO:SECURITY — N/A.
//! RO:TEST HOOKS — Type-only; covered transitively by module tests.

use std::time::Duration;

pub type ServiceName = &'static str;
pub type Version = u64;
pub type Millis = u64;
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Default bounded channel capacity for the in-process bus.
pub const DEFAULT_BUS_CAPACITY: usize = 1024;

/// Reason a supervised child stopped.
#[derive(Debug, Clone)]
pub enum CrashReason {
    Panic(String),
    Exit(i32),
    Oom,
    Error(String),
    Unknown,
}

/// Jitter bounds helper.
#[inline]
pub fn clamp_duration(v: Duration, min: Duration, max: Duration) -> Duration {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}
