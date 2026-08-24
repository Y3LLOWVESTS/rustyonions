//! RO:WHAT — Physical M1 tests for the V2 platform-bound vault and pure verified V1-to-V2 migration preparation.
//! RO:WHY — Proves encrypted device-key custody can be added without mutating V1 semantics or touching the physical Passport vault.
//! RO:INTERACTS — Phase 15R V1 envelope, V2 envelope/version dispatcher, operational-device payload crypto, and NativeEncryptedVaultV1 bounds.
//! RO:INVARIANTS — V1 bytes remain canonically reproducible; sealed factors and wrapped VMKs survive migration unchanged; V1 and V2 dispatch explicitly; malformed V2 fails closed.
//! RO:METRICS — none.
//! RO:CONFIG — native-passport feature only.
//! RO:SECURITY — fixture secrets only; no filesystem write, PlatformSealer, Keychain, Tauri, root signing, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test physical_m1_platform_bound_vault_v2_migration.

#![cfg(feature = "native-passport")]

use std::{fs, path::PathBuf};

use svc_passport::native::{
    decode_native_platform_bound_vault, decode_native_platform_bound_vault_v2,
    decode_native_platform_bound_vault_versioned, decrypt_native_operational_device_payload_v1,
    encode_native_platform_bound_vault, encode_native_platform_bound_vault_v2,
    encode_native_vault_authenticated_header, encrypt_native_operational_device_payload_v1,
    prepare_native_platform_bound_vault_v1_to_v2_migration, NativeOperationalDevicePayloadError,
    NativeOperationalDevicePayloadV1, NativePinWrappedCompartmentVmkV1,
    NativePinWrappedVaultKeysV1, NativePlatformBoundVaultError, NativePlatformBoundVaultV1,
    NativePlatformBoundVaultV2Error, NativePlatformBoundVaultVersioned, NativePlatformFamily,
    NativeSealedMaterialV1, NativeSecretBytes, NativeSecureCompartment,
    PHASE15Q_VAULT_MASTER_KEY_BYTES, PHASE15Q_WRAPPED_VMK_BYTES, PHASE6A_AEAD_NONCE_LEN,
    PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_LABEL, PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_MAGIC,
    PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_VERSION,
};

const ROOT_SALT: [u8; 16] = [0x11; 16];
const OPERATIONAL_SALT: [u8; 16] = [0x22; 16];

const ROOT_NONCE: [u8; 24] = [0x33; 24];
const OPERATIONAL_NONCE: [u8; 24] = [0x44; 24];

const DEVICE_PAYLOAD_NONCE: [u8; PHASE6A_AEAD_NONCE_LEN] = [0x77; PHASE6A_AEAD_NONCE_LEN];

fn wrapped_compartment(
    compartment: NativeSecureCompartment,
    salt: [u8; 16],
    nonce: [u8; 24],
    ciphertext_byte: u8,
) -> NativePinWrappedCompartmentVmkV1 {
    NativePinWrappedCompartmentVmkV1::new(
        compartment,
        salt,
        nonce,
        encode_native_vault_authenticated_header(compartment, &salt, &nonce),
        vec![ciphertext_byte; PHASE15Q_WRAPPED_VMK_BYTES],
    )
    .expect("structurally valid wrapped compartment")
}

fn wrapped_keys() -> NativePinWrappedVaultKeysV1 {
    NativePinWrappedVaultKeysV1::new(
        wrapped_compartment(
            NativeSecureCompartment::RecoveryRoot,
            ROOT_SALT,
            ROOT_NONCE,
            0x55,
        ),
        wrapped_compartment(
            NativeSecureCompartment::DeviceKey,
            OPERATIONAL_SALT,
            OPERATIONAL_NONCE,
            0x66,
        ),
    )
    .expect("two-compartment wrapped keys")
}

fn sealed_factor(compartment: NativeSecureCompartment, reference: &[u8]) -> NativeSealedMaterialV1 {
    NativeSealedMaterialV1::new(
        NativePlatformFamily::MacosKeychain,
        compartment,
        reference.to_vec(),
    )
    .expect("bounded sealed factor")
}

fn v1() -> NativePlatformBoundVaultV1 {
    NativePlatformBoundVaultV1::new(
        NativePlatformFamily::MacosKeychain,
        sealed_factor(
            NativeSecureCompartment::RecoveryRoot,
            b"memory://physical-m1-v2-root",
        ),
        sealed_factor(
            NativeSecureCompartment::DeviceKey,
            b"memory://physical-m1-v2-operational",
        ),
        wrapped_keys(),
    )
    .expect("valid V1 platform-bound vault")
}

fn operational_vmk() -> NativeSecretBytes {
    NativeSecretBytes::new(vec![0x88; PHASE15Q_VAULT_MASTER_KEY_BYTES])
        .expect("test operational VMK")
}

fn device_payload() -> NativeOperationalDevicePayloadV1 {
    NativeOperationalDevicePayloadV1::new(
        NativeSecretBytes::new((0u8..32u8).collect()).expect("deterministic device seed"),
    )
    .expect("valid device payload")
}

fn encrypted_device_payload() -> svc_passport::native::NativeEncryptedOperationalDevicePayloadV1 {
    encrypt_native_operational_device_payload_v1(
        &operational_vmk(),
        &DEVICE_PAYLOAD_NONCE,
        &device_payload(),
    )
    .expect("encrypted operational device payload")
}

#[test]
fn physical_m1_v2_contract_is_explicit_and_reuses_v1_magic() {
    assert_eq!(
        PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_LABEL,
        "PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2",
    );

    assert_eq!(PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_MAGIC, b"RONPBF01",);

    assert_eq!(PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_VERSION, 2,);
}

#[test]
fn physical_m1_pure_migration_preserves_complete_v1_base() {
    let original_v1 = v1();

    let original_encoded =
        encode_native_platform_bound_vault(&original_v1).expect("encode original V1");

    let migrated = prepare_native_platform_bound_vault_v1_to_v2_migration(
        &original_v1,
        &encrypted_device_payload(),
    )
    .expect("prepare verified V2 migration");

    assert_eq!(
        migrated.base_v1().platform_family(),
        original_v1.platform_family(),
    );

    assert_eq!(
        migrated.base_v1().recovery_root_factor(),
        original_v1.recovery_root_factor(),
    );

    assert_eq!(
        migrated.base_v1().operational_factor(),
        original_v1.operational_factor(),
    );

    assert_eq!(
        migrated.base_v1().wrapped_keys(),
        original_v1.wrapped_keys(),
    );

    let migrated_base_encoded =
        encode_native_platform_bound_vault(migrated.base_v1()).expect("re-encode migrated V1 base");

    assert_eq!(
        migrated_base_encoded.as_slice(),
        original_encoded.as_slice(),
        "V1 base bytes must remain canonically identical",
    );
}

#[test]
fn physical_m1_v2_round_trip_restores_same_device_seed() {
    let migrated =
        prepare_native_platform_bound_vault_v1_to_v2_migration(&v1(), &encrypted_device_payload())
            .expect("prepare V2");

    let encoded = encode_native_platform_bound_vault_v2(&migrated).expect("encode V2");

    let decoded = decode_native_platform_bound_vault_v2(&encoded).expect("decode V2");

    assert_eq!(decoded, migrated);

    let decrypted = decrypt_native_operational_device_payload_v1(
        &operational_vmk(),
        decoded.operational_device_payload(),
    )
    .expect("decrypt migrated device payload");

    assert_eq!(
        decrypted.device_signing_seed().as_slice(),
        (0u8..32u8).collect::<Vec<_>>().as_slice(),
    );
}

#[test]
fn physical_m1_versioned_base_projection_reuses_canonical_v1_for_both_versions() {
    let original_v1 = v1();

    let encoded_v1 = encode_native_platform_bound_vault(&original_v1).expect("encode V1");

    let versioned_v1 =
        decode_native_platform_bound_vault_versioned(&encoded_v1).expect("decode versioned V1");

    assert_eq!(versioned_v1.base_v1(), &original_v1,);

    let migrated = prepare_native_platform_bound_vault_v1_to_v2_migration(
        &original_v1,
        &encrypted_device_payload(),
    )
    .expect("prepare V2");

    let encoded_v2 = encode_native_platform_bound_vault_v2(&migrated).expect("encode V2");

    let versioned_v2 =
        decode_native_platform_bound_vault_versioned(&encoded_v2).expect("decode versioned V2");

    assert_eq!(versioned_v2.base_v1(), migrated.base_v1(),);
}

#[test]
fn physical_m1_version_dispatch_reads_v1_and_v2_explicitly() {
    let original_v1 = v1();

    let encoded_v1 = encode_native_platform_bound_vault(&original_v1).expect("encode V1");

    match decode_native_platform_bound_vault_versioned(&encoded_v1).expect("dispatch V1") {
        NativePlatformBoundVaultVersioned::V1(decoded) => {
            assert_eq!(decoded, original_v1);
        }
        NativePlatformBoundVaultVersioned::V2(_) => {
            panic!("V1 must not dispatch as V2");
        }
    }

    let migrated =
        prepare_native_platform_bound_vault_v1_to_v2_migration(&v1(), &encrypted_device_payload())
            .expect("prepare V2");

    let encoded_v2 = encode_native_platform_bound_vault_v2(&migrated).expect("encode V2");

    match decode_native_platform_bound_vault_versioned(&encoded_v2).expect("dispatch V2") {
        NativePlatformBoundVaultVersioned::V2(decoded) => {
            assert_eq!(decoded, migrated);
        }
        NativePlatformBoundVaultVersioned::V1(_) => {
            panic!("V2 must not dispatch as V1");
        }
    }
}

#[test]
fn physical_m1_released_v1_decoder_stays_strict_against_v2() {
    let migrated =
        prepare_native_platform_bound_vault_v1_to_v2_migration(&v1(), &encrypted_device_payload())
            .expect("prepare V2");

    let encoded_v2 = encode_native_platform_bound_vault_v2(&migrated).expect("encode V2");

    assert_eq!(
        decode_native_platform_bound_vault(&encoded_v2),
        Err(NativePlatformBoundVaultError::UnsupportedVersion { actual: 2 },),
    );
}

#[test]
fn physical_m1_v2_decoder_rejects_v1_as_wrong_version() {
    let encoded_v1 = encode_native_platform_bound_vault(&v1()).expect("encode V1");

    assert_eq!(
        decode_native_platform_bound_vault_v2(&encoded_v1),
        Err(NativePlatformBoundVaultV2Error::UnsupportedVersion { actual: 1 },),
    );
}

#[test]
fn physical_m1_v2_rejects_truncation_and_trailing_bytes() {
    let migrated =
        prepare_native_platform_bound_vault_v1_to_v2_migration(&v1(), &encrypted_device_payload())
            .expect("prepare V2");

    let encoded = encode_native_platform_bound_vault_v2(&migrated).expect("encode V2");

    let mut truncated = encoded.as_slice().to_vec();
    truncated.pop();

    let truncated = svc_passport::native::NativeEncryptedVaultV1::new(truncated)
        .expect("bounded truncated fixture");

    assert_eq!(
        decode_native_platform_bound_vault_v2(&truncated),
        Err(NativePlatformBoundVaultV2Error::InvalidEncodedVault,),
    );

    let mut trailing = encoded.as_slice().to_vec();
    trailing.push(0);

    let trailing = svc_passport::native::NativeEncryptedVaultV1::new(trailing)
        .expect("bounded trailing fixture");

    assert_eq!(
        decode_native_platform_bound_vault_v2(&trailing),
        Err(NativePlatformBoundVaultV2Error::InvalidEncodedVault,),
    );
}

#[test]
fn physical_m1_v2_payload_tamper_survives_structure_but_fails_authentication() {
    let migrated =
        prepare_native_platform_bound_vault_v1_to_v2_migration(&v1(), &encrypted_device_payload())
            .expect("prepare V2");

    let encoded = encode_native_platform_bound_vault_v2(&migrated).expect("encode V2");

    let mut tampered = encoded.as_slice().to_vec();

    let last = tampered.last_mut().expect("V2 contains payload ciphertext");

    *last ^= 0x01;

    let tampered =
        svc_passport::native::NativeEncryptedVaultV1::new(tampered).expect("bounded tampered V2");

    let decoded = decode_native_platform_bound_vault_v2(&tampered)
        .expect("tamper remains structurally decodable");

    assert_eq!(
        decrypt_native_operational_device_payload_v1(
            &operational_vmk(),
            decoded.operational_device_payload(),
        ),
        Err(NativeOperationalDevicePayloadError::AuthenticationFailed,),
    );
}

#[test]
fn physical_m1_v2_debug_and_source_boundaries_remain_native_only() {
    let migrated =
        prepare_native_platform_bound_vault_v1_to_v2_migration(&v1(), &encrypted_device_payload())
            .expect("prepare V2");

    let debug = format!("{migrated:?}");

    assert!(debug.contains("REDACTED_PLATFORM_SEALED_MATERIAL"),);

    assert!(debug.contains("REDACTED_AUTHENTICATED_CIPHERTEXT"),);

    assert!(!debug.contains("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",));

    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/native/platform_bound_vault_v2.rs"),
    )
    .expect("V2 source");

    for required in [
        "encode_native_platform_bound_vault",
        "decode_native_platform_bound_vault",
        "encode_native_encrypted_operational_device_payload_v1",
        "decode_native_encrypted_operational_device_payload_v1",
        "PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_VERSION",
        "prepare_native_platform_bound_vault_v1_to_v2_migration",
    ] {
        assert!(
            source.contains(required),
            "V2 source missing required reuse marker {required}",
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
        "getrandom::",
        "#[tauri::command]",
        "tauri::",
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
            "V2 source gained forbidden runtime marker {forbidden}",
        );
    }
}
