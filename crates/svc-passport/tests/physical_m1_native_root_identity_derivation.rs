//! RO:WHAT — Focused Physical M1 tests for deterministic Native Passport root identity derivation.
//! RO:WHY — Promotes the frozen Phase 0C reference bytes into real native-passport runtime behavior without using dev-kms.
//! RO:INTERACTS — HKDF-SHA256, Ed25519, Passport-ID derivation, NativeSecretBytes, and Phase 0C/0D vectors.
//! RO:INVARIANTS — exact salt/info, signing-seed vector, root-public-key vector, Passport-ID vector, determinism, and malformed-length rejection.
//! RO:SECURITY — uses published test material only; no live Passport, PIN, platform sealer, username, wallet, or ledger mutation.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test physical_m1_native_root_identity_derivation.

#![cfg(feature = "native-passport")]

use hkdf::Hkdf;
use sha2::Sha256;

use svc_passport::{
    native::{
        derive_native_root_public_identity_v1, NativeRootIdentityDerivationError,
        NativeSecretBytes, PHYSICAL_M1_ROOT_IDENTITY_DERIVATION_LABEL,
    },
    native_plan::{
        ROOT_IDENTITY_V1_BIP39_SEED_BYTES, ROOT_IDENTITY_V1_HKDF_INFO, ROOT_IDENTITY_V1_HKDF_SALT,
        ROOT_IDENTITY_V1_SIGNING_SEED_BYTES,
    },
};

const BIP39_SEED_HEX: &str =
    "5eb00bbddcf069084889a8ab9155568165f5c453ccb85e70811aaed6f6da5fc19a5ac40b389cd370d086206dec8aa6c43daea6690f20ad3d8d48b2d2ce9e38e4";

const EXPECTED_ROOT_SIGNING_SEED_HEX: &str =
    "6494f88bac1634fb4db0aaaecc6841f4dc21e4e744e2cb58378fcda7cf3d2dca";

const EXPECTED_ROOT_PUBLIC_KEY_HEX: &str =
    "3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909";

const EXPECTED_PASSPORT_ID: &str =
    "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";

#[test]
fn independent_hkdf_reproduces_phase0c_signing_seed() {
    assert!(!cfg!(feature = "dev-kms"));

    let seed = decode_hex(BIP39_SEED_HEX);

    let hkdf = Hkdf::<Sha256>::new(Some(ROOT_IDENTITY_V1_HKDF_SALT.as_bytes()), &seed);

    let mut derived = [0u8; 32];

    hkdf.expand(ROOT_IDENTITY_V1_HKDF_INFO.as_bytes(), &mut derived)
        .expect("locked HKDF parameters");

    assert_eq!(lower_hex(&derived), EXPECTED_ROOT_SIGNING_SEED_HEX,);
}

#[test]
fn root_identity_reproduces_phase0c_and_phase0d_vectors() {
    assert_eq!(
        PHYSICAL_M1_ROOT_IDENTITY_DERIVATION_LABEL,
        "PHYSICAL_M1_NATIVE_ROOT_IDENTITY_DERIVATION_V1",
    );

    assert_eq!(ROOT_IDENTITY_V1_BIP39_SEED_BYTES, 64,);

    assert_eq!(ROOT_IDENTITY_V1_SIGNING_SEED_BYTES, 32,);

    let seed = NativeSecretBytes::new(decode_hex(BIP39_SEED_HEX)).expect("published seed fixture");

    let root = derive_native_root_public_identity_v1(&seed).expect("canonical root identity");

    assert_eq!(root.root_public_key.as_str(), EXPECTED_ROOT_PUBLIC_KEY_HEX,);

    assert_eq!(root.passport_id.as_str(), EXPECTED_PASSPORT_ID,);

    assert!(root.optional_handle.is_none());
}

#[test]
fn root_identity_is_deterministic() {
    let bytes = decode_hex(BIP39_SEED_HEX);

    let first = NativeSecretBytes::new(bytes.clone()).expect("first seed");

    let second = NativeSecretBytes::new(bytes).expect("second seed");

    assert_eq!(
        derive_native_root_public_identity_v1(&first,).expect("first"),
        derive_native_root_public_identity_v1(&second,).expect("second"),
    );
}

#[test]
fn root_identity_rejects_wrong_seed_length() {
    let short = NativeSecretBytes::new(vec![7u8; 32]).expect("short fixture");

    assert_eq!(
        derive_native_root_public_identity_v1(&short,),
        Err(NativeRootIdentityDerivationError::InvalidBip39SeedLength {
            actual: 32,
            expected: 64,
        },),
    );
}

fn decode_hex(input: &str) -> Vec<u8> {
    assert_eq!(input.len() % 2, 0);

    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]))
        .collect()
}

fn hex_nibble(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        b'A'..=b'F' => value - b'A' + 10,
        _ => panic!("invalid hex"),
    }
}

fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
