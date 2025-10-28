//! RO:WHAT — Wire (encoding) helpers for JSON/CBOR round-trips.
//! RO:WHY  — Interop hygiene; DTOs are pure; services pick the transport.
//! RO:INVARIANTS — #[serde(deny_unknown_fields)] on message shapes.

pub mod json;
pub mod cbor;
