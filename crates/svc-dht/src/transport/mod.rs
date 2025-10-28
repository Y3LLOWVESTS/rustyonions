//! RO:WHAT — Thin wrapper around ron-transport clients
//! RO:WHY — Keep svc-dht transport-agnostic; Concerns: SEC/RES
pub mod clients; // TODO phase 2
#[cfg(feature = "arti")]
pub mod tor; // TODO phase 2
