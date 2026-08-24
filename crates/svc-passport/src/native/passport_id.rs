//! RO:WHAT — Derives the canonical Native Passport V1 public Passport ID from a validated Ed25519 root public key.
//! RO:WHY — Physical M1 needs a real Passport subject before any username claim can leave CrabLink.
//! RO:INTERACTS — Native Passport DTO validation, Phase 0D Passport-ID constants, BLAKE3, and future root-key finalization.
//! RO:INVARIANTS — exact Phase 0D domain/kind/algorithm encoding; public-key input only; deterministic output; no username, wallet, vault, PIN, or device material participates.
//! RO:SECURITY — no key generation, signing, secret access, recovery material, vault mutation, capability issuance, wallet mutation, or ledger mutation.
//! RO:TEST — tests/physical_m1_native_passport_id_derivation.rs.

use crate::native_plan::{PASSPORT_ID_V1_HASH_DOMAIN, PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX};

use super::{Ed25519PublicKeyHex, NativePassportDtoError, PassportIdV1};

/// Physical M1 label for the first real Native Passport identity primitive.
pub const PHYSICAL_M1_PASSPORT_ID_DERIVATION_LABEL: &str =
    "PHYSICAL_M1_NATIVE_PASSPORT_ID_DERIVATION_V1";

/// Derive a canonical Native Passport V1 main Passport ID from a validated
/// Ed25519 root public key.
///
/// Frozen Phase 0D hash input:
/// `domain|main|ed25519|root_public_key_hex`.
pub fn derive_native_passport_id_v1(
    root_public_key: &Ed25519PublicKeyHex,
) -> Result<PassportIdV1, NativePassportDtoError> {
    let hash_input = format!(
        "{}|main|ed25519|{}",
        PASSPORT_ID_V1_HASH_DOMAIN,
        root_public_key.as_str(),
    );

    let digest = blake3::hash(hash_input.as_bytes()).to_hex().to_string();

    PassportIdV1::parse(format!(
        "{}{}",
        PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX, digest,
    ))
}
