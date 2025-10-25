//! RO:WHAT — Crash-only supervision with jittered backoff.
//! RO:WHY  — Resilience; restarts counted by host metrics via observe hooks.
//! RO:INTERACTS — backoff calc; host spawns async tasks; no global runtime.
//! RO:INVARIANTS — decorrelated jitter; bounded backoff; cancel-safe.
#![allow(clippy::module_inception)]

mod backoff;
mod supervisor;

pub use backoff::decorrelated_jitter;
pub use supervisor::Supervisor;
