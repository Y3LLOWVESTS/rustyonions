//! RO:WHAT — (stub) Macaroon-like caveats; keep deterministic JSON for signing.
//! RO:NOTE — Silver+: add first-class caveat evaluation and attenuation.

#![allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Caveat {
    pub kind: String,
    pub value: String,
}
