//! RO:WHAT — Observability surfaces for Macronode.
//! RO:WHY  — Keep main/bootstrap clean; centralize logging/metrics wiring.
//! RO:INVARIANTS —
//!   - Logging is initialized exactly once per process.
//!   - Metrics module is present (even if initially a stub) so `/metrics` works.

pub mod logging;
pub mod metrics;
