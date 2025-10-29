//! RO:WHAT — Authentication surface for svc-storage (macaroon-style, keyed BLAKE3).

mod macaroon;
pub use macaroon::MacaroonClaims;
// Keep helpers private until we wire a mint script or tests:
// pub use macaroon::{mint_for, MacaroonError};
