//! RO:WHAT — Governance DTOs (signable descriptors; no signatures here).
//! RO:WHY  — Typed inputs for policy/registry; PQ-agile via quantum:: tags.

pub mod signed_descriptor;

pub use signed_descriptor::{SignedDescriptorV1, MultiSigNofM};
