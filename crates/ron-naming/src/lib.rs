#![forbid(unsafe_code)]
//! ron-naming2 — pure library for naming schemas/normalization/encodings.
//!
//! This scaffold intentionally contains **no Rust logic yet**; it establishes
//! a modular file layout to keep code small and maintainable.
//!
//! Public re-exports will be added here as the implementation lands.

pub mod types;
pub mod normalize;
pub mod address;
pub mod version;
pub mod wire;

#[cfg(feature = "verify")]
pub mod verify;
