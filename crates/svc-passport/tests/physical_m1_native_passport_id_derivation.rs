//! RO:WHAT — Focused Physical M1 tests for canonical Native Passport V1 Passport-ID derivation.
//! RO:WHY — Locks the first real public identity primitive to the already-frozen Phase 0D vector.
//! RO:INTERACTS — svc-passport native Passport-ID derivation, DTO validators, and Phase 0D vector constants.
//! RO:INVARIANTS — exact domain/kind/algorithm/public-key transcript and deterministic BLAKE3 result.
//! RO:SECURITY — test uses public fixture material only; no private keys, recovery phrase, PIN, vault mutation, signing, wallet, or ledger authority.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test physical_m1_native_passport_id_derivation.

#![cfg(feature = "native-passport")]

use svc_passport::{
    native::{
        derive_native_passport_id_v1, Ed25519PublicKeyHex, PHYSICAL_M1_PASSPORT_ID_DERIVATION_LABEL,
    },
    native_plan::{
        is_passport_id_v1_main_ed25519_b3, PASSPORT_ID_V1_HASH_DOMAIN,
        PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX,
    },
};

const ROOT_PUBLIC_KEY_HEX: &str =
    "3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909";

const EXPECTED_HASH_INPUT: &str =
    "rustyonions.native-passport.passport-id.v1|main|ed25519|3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909";

const EXPECTED_PASSPORT_ID: &str =
    "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";

#[test]
fn physical_m1_derivation_reproduces_frozen_phase0d_vector() {
    let root_public_key =
        Ed25519PublicKeyHex::parse(ROOT_PUBLIC_KEY_HEX).expect("locked Phase 0C root public key");

    let derived = derive_native_passport_id_v1(&root_public_key).expect("canonical Passport ID");

    assert_eq!(
        PHYSICAL_M1_PASSPORT_ID_DERIVATION_LABEL,
        "PHYSICAL_M1_NATIVE_PASSPORT_ID_DERIVATION_V1",
    );

    assert_eq!(
        PASSPORT_ID_V1_HASH_DOMAIN,
        "rustyonions.native-passport.passport-id.v1",
    );

    assert_eq!(
        PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX,
        "passport:v1:main:ed25519:b3:",
    );

    assert_eq!(derived.as_str(), EXPECTED_PASSPORT_ID,);

    assert!(is_passport_id_v1_main_ed25519_b3(derived.as_str(),),);
}

#[test]
fn physical_m1_independent_hash_transcript_matches_phase0d() {
    let independent_hash_input = format!(
        "{}|main|ed25519|{}",
        PASSPORT_ID_V1_HASH_DOMAIN, ROOT_PUBLIC_KEY_HEX,
    );

    assert_eq!(independent_hash_input, EXPECTED_HASH_INPUT,);

    let digest = blake3::hash(independent_hash_input.as_bytes())
        .to_hex()
        .to_string();

    assert_eq!(
        format!("{}{}", PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX, digest,),
        EXPECTED_PASSPORT_ID,
    );
}

#[test]
fn physical_m1_passport_id_derivation_is_deterministic() {
    let root_public_key =
        Ed25519PublicKeyHex::parse(ROOT_PUBLIC_KEY_HEX).expect("locked public key");

    let first = derive_native_passport_id_v1(&root_public_key).expect("first derivation");

    let second = derive_native_passport_id_v1(&root_public_key).expect("second derivation");

    assert_eq!(first, second,);
}
