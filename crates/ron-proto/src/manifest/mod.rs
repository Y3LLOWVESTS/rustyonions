//! RO:WHAT — Versioned manifest DTOs for content graphs.
//! RO:WHY  — Names → manifests → providers; pure data for index/storage.
//! RO:INVARIANTS — Explicit versioning; deterministic ordering (BTreeMap when maps appear).

pub mod common;
pub mod v1;

pub use common::*;
pub use v1::*;
