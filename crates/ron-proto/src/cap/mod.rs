//! RO:WHAT — Capability token header DTOs (claims/caveats only).
//! RO:WHY  — Typed claims for macaroon-style caps; verification lives in auth services.

pub mod header;
pub mod caveats;

pub use header::CapTokenHdr;
pub use caveats::{Caveat, CaveatKind};
