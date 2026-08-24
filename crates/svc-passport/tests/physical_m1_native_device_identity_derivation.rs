//! RO:WHAT — Focused Physical M1 tests for canonical Native Passport V1 device public identity derivation.
//! RO:WHY — Promotes the frozen Phase 0F Device-ID vector into real native-passport behavior before device generation, custody, root authorization, or server proof runtime.
//! RO:INTERACTS — device_identity, Phase 0F Device-ID constants, Ed25519, BLAKE3, NativeSecretBytes, and canonical Native Passport DTOs.
//! RO:INVARIANTS — exact Device-ID transcript/domain/prefix; deterministic public derivation; malformed seed length rejects; public result contains no secret material.
//! RO:METRICS — none.
//! RO:CONFIG — native-passport feature only.
//! RO:SECURITY — deterministic test seed only; no live Passport vault, Keychain, PIN, root signing, RNG, persistence, routes, username, wallet, or ledger mutation.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test physical_m1_native_device_identity_derivation.

#![cfg(feature = "native-passport")]

use std::{fs, path::PathBuf};

use svc_passport::{
    native::{
        derive_native_device_id_v1, derive_native_device_public_identity_v1, Ed25519PublicKeyHex,
        NativeDeviceIdentityDerivationError, NativeSecretBytes, DEVICE_ID_V1_SIGNING_SEED_BYTES,
        PHYSICAL_M1_DEVICE_IDENTITY_DERIVATION_LABEL,
    },
    native_plan::{
        is_device_id_v1_ed25519_b3, DEVICE_ID_V1_ED25519_B3_PREFIX, DEVICE_ID_V1_HASH_DOMAIN,
    },
};

const LOCKED_DEVICE_PUBLIC_KEY_HEX: &str =
    "2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";

const LOCKED_DEVICE_ID: &str =
    "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";

const SEQUENTIAL_TEST_SEED_PUBLIC_KEY_HEX: &str =
    "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8";

#[test]
fn physical_m1_device_id_reproduces_frozen_phase0f_vector() {
    assert_eq!(
        PHYSICAL_M1_DEVICE_IDENTITY_DERIVATION_LABEL,
        "PHYSICAL_M1_NATIVE_DEVICE_IDENTITY_DERIVATION_V1",
    );

    assert_eq!(
        DEVICE_ID_V1_HASH_DOMAIN,
        "rustyonions.native-passport.device-id.v1",
    );

    assert_eq!(DEVICE_ID_V1_ED25519_B3_PREFIX, "device:v1:ed25519:b3:",);

    let public_key = Ed25519PublicKeyHex::parse(LOCKED_DEVICE_PUBLIC_KEY_HEX)
        .expect("locked Phase 0F public key");

    let device_id = derive_native_device_id_v1(&public_key).expect("canonical Device ID");

    assert_eq!(device_id.as_str(), LOCKED_DEVICE_ID);

    assert!(is_device_id_v1_ed25519_b3(device_id.as_str()));
}

#[test]
fn physical_m1_device_id_transcript_matches_independent_phase0f_hash() {
    let public_key =
        Ed25519PublicKeyHex::parse(LOCKED_DEVICE_PUBLIC_KEY_HEX).expect("locked public key");

    let independent_hash_input = format!(
        "{}|ed25519|{}",
        DEVICE_ID_V1_HASH_DOMAIN, LOCKED_DEVICE_PUBLIC_KEY_HEX,
    );

    let digest = blake3::hash(independent_hash_input.as_bytes())
        .to_hex()
        .to_string();

    let expected = format!("{}{}", DEVICE_ID_V1_ED25519_B3_PREFIX, digest,);

    assert_eq!(expected, LOCKED_DEVICE_ID);

    assert_eq!(
        derive_native_device_id_v1(&public_key)
            .expect("derived Device ID")
            .as_str(),
        expected,
    );
}

#[test]
fn physical_m1_seed_derives_expected_ed25519_public_identity() {
    assert_eq!(DEVICE_ID_V1_SIGNING_SEED_BYTES, 32);

    let seed_bytes: Vec<u8> = (0u8..32u8).collect();

    let seed = NativeSecretBytes::new(seed_bytes).expect("deterministic test seed");

    let identity = derive_native_device_public_identity_v1(&seed).expect("public device identity");

    assert_eq!(
        identity.device_public_key.as_str(),
        SEQUENTIAL_TEST_SEED_PUBLIC_KEY_HEX,
    );

    assert!(is_device_id_v1_ed25519_b3(identity.device_id.as_str()),);

    let debug = format!("{identity:?}");

    assert!(debug.contains(SEQUENTIAL_TEST_SEED_PUBLIC_KEY_HEX));

    assert!(!debug.contains("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",));
}

#[test]
fn physical_m1_device_public_identity_is_deterministic_for_same_seed() {
    let bytes: Vec<u8> = (0u8..32u8).collect();

    let first = NativeSecretBytes::new(bytes.clone()).expect("first seed");

    let second = NativeSecretBytes::new(bytes).expect("second seed");

    assert_eq!(
        derive_native_device_public_identity_v1(&first).expect("first identity"),
        derive_native_device_public_identity_v1(&second).expect("second identity"),
    );
}

#[test]
fn physical_m1_device_identity_rejects_wrong_seed_length() {
    let short = NativeSecretBytes::new(vec![7u8; 31]).expect("short fixture");

    assert_eq!(
        derive_native_device_public_identity_v1(&short),
        Err(
            NativeDeviceIdentityDerivationError::InvalidDeviceSigningSeedLength {
                actual: 31,
                expected: 32,
            },
        ),
    );
}

#[test]
fn physical_m1_device_identity_source_remains_pure_without_runtime_authority() {
    let source_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/native/device_identity.rs");

    let source = fs::read_to_string(source_path).expect("device identity source");

    for forbidden in [
        "serde::",
        "Serialize",
        "std::fs::",
        "tokio::fs",
        "Router::",
        ".route(",
        "OsRng",
        "getrandom::",
        "platform_seal(",
        "platform_unseal(",
        "vault_decrypt(",
        "vault_encrypt(",
        "issue_capability(",
        "wallet.spend(",
        "ledger.write(",
        "mint_roc(",
        "burn_roc(",
    ] {
        assert!(
            !source.contains(forbidden),
            "device identity primitive must not gain runtime marker {forbidden}",
        );
    }
}
