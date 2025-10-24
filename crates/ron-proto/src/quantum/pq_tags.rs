//! RO:WHAT — Enumerations for PQ-capable algorithms referenced by DTOs.
//! RO:WHY  — Wire-stable tokens for hybrid/transition periods (no crypto here).

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SignatureAlg {
    Ed25519,
    Dilithium3,
    HybridEd25519Dilithium3,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum KemAlg {
    X25519,
    Kyber768,
    HybridX25519Kyber768,
}
