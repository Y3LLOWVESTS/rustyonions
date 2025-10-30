//! RO:WHAT — Feature switches (placeholder for future toggles).
//!
//! RO:WHY  — Keep public surface stable while allowing internal perf/security opts.
//!
//! RO:INTERACTS — N/A today
//!
//! RO:INVARIANTS — default = strict parsing on

#[allow(dead_code)]
pub const STRICT: bool = cfg!(feature = "strict");
