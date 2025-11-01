//! RO:WHAT — Error taxonomy + HTTP mapping.
//! RO:WHY  — Deterministic errors (RON invariant); Concerns: DX/GOV.
//! RO:INTERACTS — http_map.rs, reasons.rs.

pub mod http_map;
pub mod reasons;

pub use http_map::{GateError, Problem};
pub use reasons::Reason;
