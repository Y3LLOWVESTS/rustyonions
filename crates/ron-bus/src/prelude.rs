//! RO:WHAT — Convenience prelude for common imports.
//! RO:WHY  — Reduce repetitive `use ron_bus::{...}` in host code/examples.
//! RO:INTERACTS — Re-exports public types only (no macros, no globals).
//! RO:INVARIANTS — Keep small and explicit.

pub use crate::{Bus, BusConfig, BusError, Event};
