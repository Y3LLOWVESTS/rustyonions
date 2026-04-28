//! RO:WHAT — Compatibility shim for tooling that expects exporter/trait.rs.
//! RO:WHY — Pillar 12; Concerns: DX. Rust keyword handling is kept out of callers.
//! RO:INTERACTS — exporter::trait_ defines the actual public trait contract.
//! RO:INVARIANTS — no duplicate logic; re-export only.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — none.
//! RO:TEST — compile coverage through exporter::trait_mod.

pub use super::trait_::*;
