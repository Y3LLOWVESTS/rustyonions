//! RO:WHAT — PQ-hybrid algorithm tags (enums only; no crypto).
//! RO:WHY  — Keep protocol PQ-agile without dragging crypto deps here.

pub mod pq_tags;

pub use pq_tags::{KemAlg, SignatureAlg};
