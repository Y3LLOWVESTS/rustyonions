//! RO:WHAT — Capability token header DTOs (claims/caveats only).
//! RO:WHY  — Typed claims for macaroon-style caps; verification lives in auth services.

pub mod caveats;
pub mod header;

pub use caveats::{Caveat, CaveatKind};
pub use header::CapTokenHdr;
