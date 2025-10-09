//! ron-audit2 — structure-only scaffold (no logic here).
//! Public surface to be defined per finalized docs.

pub mod errors;
pub mod dto;
pub mod canon;
pub mod hash;
pub mod verify;
pub mod bounds;
pub mod sink;
pub mod stream;
pub mod privacy;
pub mod metrics;

// A small prelude is handy for hosts (left empty for now).
pub mod prelude {}
