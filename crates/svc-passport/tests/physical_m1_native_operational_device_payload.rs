//! RO:WHAT — Focused Physical M1 tests for versioned authenticated encryption of the Native Passport operational device payload.
//! RO:WHY — Proves the generated per-device signing seed can be protected by the operational VMK before any real Passport migration or filesystem mutation.
//! RO:INTERACTS — operational_device_payload, NativeSecretBytes, HKDF-SHA256, XChaCha20-Poly1305, and canonical device identity derivation.
//! RO:INVARIANTS — 32-byte VMK; 32-byte device seed; canonical persisted public-key binding must match the seed-derived identity; versioned/bounded envelope; wrong key, tamper, truncation, trailing data, malformed versions, and logical binding mismatch fail closed.
//! RO:METRICS — none.
//! RO:CONFIG — native-passport feature only.
//! RO:SECURITY — fixtures only; no live vault, PlatformSealer, Keychain, filesystem, root signature, username, wallet, ledger, or WebView mutation.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test physical_m1_native_operational_device_payload.

#![cfg(feature = "native-passport")]

use std::{fs, path::PathBuf};

use svc_passport::native::{
    decode_native_encrypted_operational_device_payload_v1,
    decrypt_native_operational_device_payload_v1, derive_native_device_public_identity_v1,
    encode_native_encrypted_operational_device_payload_v1,
    encrypt_native_operational_device_payload_v1, NativeOperationalDevicePayloadError,
    NativeOperationalDevicePayloadV1, NativeSecretBytes, DEVICE_ID_V1_SIGNING_SEED_BYTES,
    PHASE15Q_VAULT_MASTER_KEY_BYTES, PHASE6A_AEAD_NONCE_LEN, PHYSICAL_M1_DEVICE_PAYLOAD_AAD_DOMAIN,
    PHYSICAL_M1_DEVICE_PAYLOAD_ENVELOPE_MAGIC, PHYSICAL_M1_DEVICE_PAYLOAD_KEY_DOMAIN,
    PHYSICAL_M1_DEVICE_PAYLOAD_MAX_CIPHERTEXT_BYTES, PHYSICAL_M1_DEVICE_PAYLOAD_MAX_ENCODED_BYTES,
    PHYSICAL_M1_DEVICE_PAYLOAD_PLAINTEXT_MAGIC, PHYSICAL_M1_DEVICE_PAYLOAD_VERSION,
    PHYSICAL_M1_OPERATIONAL_DEVICE_PAYLOAD_LABEL, PHYSICAL_M1_OPERATIONAL_VAULT_DOMAIN,
};

const SEQUENTIAL_TEST_SEED_PUBLIC_KEY_HEX: &str =
    "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8";

fn vmk(byte: u8) -> NativeSecretBytes {
    NativeSecretBytes::new(vec![byte; PHASE15Q_VAULT_MASTER_KEY_BYTES])
        .expect("test operational VMK")
}

fn sequential_seed() -> NativeSecretBytes {
    NativeSecretBytes::new((0u8..32u8).collect()).expect("deterministic device seed")
}

fn payload() -> NativeOperationalDevicePayloadV1 {
    NativeOperationalDevicePayloadV1::new(sequential_seed())
        .expect("valid operational device payload")
}

fn nonce() -> [u8; PHASE6A_AEAD_NONCE_LEN] {
    [0x53; PHASE6A_AEAD_NONCE_LEN]
}

#[test]
fn physical_m1_operational_device_payload_contract_is_locked() {
    assert_eq!(
        PHYSICAL_M1_OPERATIONAL_DEVICE_PAYLOAD_LABEL,
        "PHYSICAL_M1_NATIVE_OPERATIONAL_DEVICE_PAYLOAD_V1",
    );

    assert_eq!(PHYSICAL_M1_DEVICE_PAYLOAD_VERSION, 1);

    assert_eq!(PHYSICAL_M1_DEVICE_PAYLOAD_PLAINTEXT_MAGIC, b"RONODS01",);

    assert_eq!(PHYSICAL_M1_DEVICE_PAYLOAD_ENVELOPE_MAGIC, b"RONODE01",);

    assert_eq!(
        PHYSICAL_M1_OPERATIONAL_VAULT_DOMAIN,
        "rustyonions.native-passport.operational-vault.v1",
    );

    assert_eq!(
        PHYSICAL_M1_DEVICE_PAYLOAD_KEY_DOMAIN,
        PHYSICAL_M1_OPERATIONAL_VAULT_DOMAIN,
    );

    assert_eq!(
        PHYSICAL_M1_DEVICE_PAYLOAD_AAD_DOMAIN,
        PHYSICAL_M1_OPERATIONAL_VAULT_DOMAIN,
    );

    assert_eq!(DEVICE_ID_V1_SIGNING_SEED_BYTES, 32);
    assert_eq!(PHASE15Q_VAULT_MASTER_KEY_BYTES, 32);
    assert_eq!(PHASE6A_AEAD_NONCE_LEN, 24);

    assert!(
        PHYSICAL_M1_DEVICE_PAYLOAD_MAX_CIPHERTEXT_BYTES
            < PHYSICAL_M1_DEVICE_PAYLOAD_MAX_ENCODED_BYTES,
    );
}

#[test]
fn physical_m1_device_payload_round_trips_and_preserves_device_identity() {
    let operational_vmk = vmk(0x21);
    let original = payload();
    let nonce = nonce();

    let encrypted =
        encrypt_native_operational_device_payload_v1(&operational_vmk, &nonce, &original)
            .expect("encrypt operational device payload");

    let encoded = encode_native_encrypted_operational_device_payload_v1(&encrypted)
        .expect("encode encrypted operational device payload");

    assert!(encoded.len() <= PHYSICAL_M1_DEVICE_PAYLOAD_MAX_ENCODED_BYTES,);

    let decoded = decode_native_encrypted_operational_device_payload_v1(&encoded)
        .expect("decode encrypted operational device payload");

    let restored = decrypt_native_operational_device_payload_v1(&operational_vmk, &decoded)
        .expect("decrypt operational device payload");

    assert_eq!(
        original.device_signing_seed().as_slice(),
        restored.device_signing_seed().as_slice(),
    );

    assert_eq!(original.device_public_key(), restored.device_public_key(),);

    assert_eq!(
        restored.device_public_key().as_str(),
        SEQUENTIAL_TEST_SEED_PUBLIC_KEY_HEX,
    );

    let identity = derive_native_device_public_identity_v1(restored.device_signing_seed())
        .expect("restored public device identity");

    assert_eq!(identity.device_public_key, *restored.device_public_key(),);

    assert_eq!(
        identity.device_public_key.as_str(),
        SEQUENTIAL_TEST_SEED_PUBLIC_KEY_HEX,
    );
}

#[test]
fn physical_m1_persistable_ciphertext_does_not_contain_plain_device_seed() {
    let operational_vmk = vmk(0x31);
    let original = payload();

    let encrypted =
        encrypt_native_operational_device_payload_v1(&operational_vmk, &nonce(), &original)
            .expect("encrypt device payload");

    let encoded = encode_native_encrypted_operational_device_payload_v1(&encrypted)
        .expect("encode device payload");

    let seed = original.device_signing_seed().as_slice();

    assert!(
        !encoded.windows(seed.len()).any(|window| window == seed),
        "persistable envelope must not contain plaintext device seed",
    );

    let debug = format!("{encrypted:?}");

    assert!(debug.contains("REDACTED_AUTHENTICATED_CIPHERTEXT"),);

    assert!(!debug.contains("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",));

    let payload_debug = format!("{original:?}");

    assert!(payload_debug.contains("REDACTED_NATIVE_DEVICE_SIGNING_SEED",),);
}

#[test]
fn physical_m1_wrong_operational_vmk_fails_authentication() {
    let correct_vmk = vmk(0x41);
    let wrong_vmk = vmk(0x42);

    let encrypted =
        encrypt_native_operational_device_payload_v1(&correct_vmk, &nonce(), &payload())
            .expect("encrypt device payload");

    assert_eq!(
        decrypt_native_operational_device_payload_v1(&wrong_vmk, &encrypted,),
        Err(NativeOperationalDevicePayloadError::AuthenticationFailed,),
    );
}

#[test]
fn physical_m1_ciphertext_tamper_fails_authentication() {
    let operational_vmk = vmk(0x51);

    let encrypted =
        encrypt_native_operational_device_payload_v1(&operational_vmk, &nonce(), &payload())
            .expect("encrypt device payload");

    let mut encoded = encode_native_encrypted_operational_device_payload_v1(&encrypted)
        .expect("encode device payload");

    let last = encoded.last_mut().expect("ciphertext byte exists");

    *last ^= 0x01;

    let tampered = decode_native_encrypted_operational_device_payload_v1(&encoded)
        .expect("structurally valid tampered envelope");

    assert_eq!(
        decrypt_native_operational_device_payload_v1(&operational_vmk, &tampered,),
        Err(NativeOperationalDevicePayloadError::AuthenticationFailed,),
    );
}

#[test]
fn physical_m1_payload_codec_rejects_truncation_and_trailing_bytes() {
    let encrypted = encrypt_native_operational_device_payload_v1(&vmk(0x61), &nonce(), &payload())
        .expect("encrypt device payload");

    let encoded = encode_native_encrypted_operational_device_payload_v1(&encrypted)
        .expect("encode device payload");

    let mut truncated = encoded.clone();
    truncated.pop();

    assert_eq!(
        decode_native_encrypted_operational_device_payload_v1(&truncated,),
        Err(NativeOperationalDevicePayloadError::InvalidEncodedPayload,),
    );

    let mut trailing = encoded;
    trailing.push(0);

    assert_eq!(
        decode_native_encrypted_operational_device_payload_v1(&trailing,),
        Err(NativeOperationalDevicePayloadError::InvalidEncodedPayload,),
    );
}

#[test]
fn physical_m1_payload_codec_rejects_wrong_magic_and_version() {
    let encrypted = encrypt_native_operational_device_payload_v1(&vmk(0x71), &nonce(), &payload())
        .expect("encrypt device payload");

    let encoded = encode_native_encrypted_operational_device_payload_v1(&encrypted)
        .expect("encode device payload");

    let mut wrong_magic = encoded.clone();
    wrong_magic[0] ^= 0x01;

    assert_eq!(
        decode_native_encrypted_operational_device_payload_v1(&wrong_magic,),
        Err(NativeOperationalDevicePayloadError::InvalidEncodedPayload,),
    );

    let mut wrong_version = encoded;

    let version_offset = PHYSICAL_M1_DEVICE_PAYLOAD_ENVELOPE_MAGIC.len();

    wrong_version[version_offset] = 0;
    wrong_version[version_offset + 1] = 2;

    assert_eq!(
        decode_native_encrypted_operational_device_payload_v1(&wrong_version,),
        Err(NativeOperationalDevicePayloadError::UnsupportedVersion { actual: 2 },),
    );
}

#[test]
fn physical_m1_payload_rejects_invalid_secret_lengths() {
    let short_vmk = NativeSecretBytes::new(vec![0x81; 31]).expect("short VMK fixture");

    assert_eq!(
        encrypt_native_operational_device_payload_v1(&short_vmk, &nonce(), &payload(),),
        Err(
            NativeOperationalDevicePayloadError::InvalidOperationalVmkLength {
                actual: 31,
                expected: 32,
            },
        ),
    );

    let short_seed = NativeSecretBytes::new(vec![0x91; 31]).expect("short seed fixture");

    assert_eq!(
        NativeOperationalDevicePayloadV1::new(short_seed),
        Err(
            NativeOperationalDevicePayloadError::InvalidDeviceSigningSeedLength {
                actual: 31,
                expected: 32,
            },
        ),
    );
}

#[test]
fn physical_m1_operational_device_payload_source_has_no_persistence_or_authority() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/native/operational_device_payload.rs"),
    )
    .expect("operational device payload source");

    for required in [
        "Hkdf::<Sha256>",
        "XChaCha20Poly1305",
        "Zeroizing",
        "NativeSecretBytes",
        "PHYSICAL_M1_DEVICE_PAYLOAD_KEY_DOMAIN",
        "PHYSICAL_M1_DEVICE_PAYLOAD_AAD_DOMAIN",
        "REDACTED_NATIVE_DEVICE_SIGNING_SEED",
        "REDACTED_AUTHENTICATED_CIPHERTEXT",
        "derive_native_device_public_identity_v1",
        "DevicePublicBindingMismatch",
        "device_public_key",
    ] {
        assert!(
            source.contains(required),
            "payload source missing {required}",
        );
    }

    for forbidden in [
        "std::fs::",
        "tokio::fs",
        "File::create",
        "OpenOptions::",
        "NativePlatformSealer",
        "seal_native_secret(",
        "unseal_native_secret(",
        "write_native_encrypted_vault_atomic(",
        "getrandom::",
        "#[tauri::command]",
        "tauri::",
        "serde::Serialize",
        "serde::Deserialize",
        "SigningKey",
        ".sign(",
        "issue_capability(",
        "wallet.spend(",
        "ledger.write(",
        "mint_roc(",
        "burn_roc(",
        "println!",
        "eprintln!",
        "tracing::",
    ] {
        assert!(
            !source.contains(forbidden),
            "payload primitive gained forbidden runtime marker {forbidden}",
        );
    }
}
