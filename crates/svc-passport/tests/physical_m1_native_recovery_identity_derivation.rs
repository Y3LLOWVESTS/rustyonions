//! RO:WHAT — Focused Physical M1 end-to-end vectors from recovery entropy to canonical public Passport identity.
//! RO:WHY — Proves the stored 256-bit recovery factor can deterministically become the real public Passport subject.
//! RO:INTERACTS — canonical 24-word recovery mapping, PBKDF2-HMAC-SHA512 BIP-39 seed derivation, Phase 0C root derivation, and Phase 0D Passport-ID derivation.
//! RO:INVARIANTS — zero/max entropy vectors are locked; the file is compiled only when native-passport is enabled and dev-kms is absent; malformed recovery-factor length fails closed.
//! RO:SECURITY — public deterministic fixtures only; no real vault, Keychain, PIN, Tauri command, username, wallet, or ledger mutation.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test physical_m1_native_recovery_identity_derivation.

#![cfg(all(feature = "native-passport", not(feature = "dev-kms")))]

use svc_passport::{
    native::{
        derive_native_passport_id_v1, derive_native_recovery_public_identity_v1,
        Ed25519PublicKeyHex, NativeRecoveryIdentityDerivationError, NativeSecretBytes,
        PHYSICAL_M1_RECOVERY_IDENTITY_DERIVATION_LABEL,
    },
    native_plan::{
        BIP39_SEED_V1_PASSPHRASE_PROFILE, BIP39_SEED_V1_PBKDF2_ROUNDS, BIP39_SEED_V1_SALT_PREFIX,
    },
};

const ZERO_FACTOR_ROOT_PUBLIC_KEY: &str =
    "222a0f7af9cc0bcbd908cc7a7ac0a070e1f163c88d4068932271639bef065efe";

const MAX_FACTOR_ROOT_PUBLIC_KEY: &str =
    "4fd1203c0693a262ffc50435657198cc3953bfbf6dafff866ea8a0a1e4da20eb";

#[test]
fn zero_recovery_factor_reproduces_locked_bip39_root_identity() {
    assert_eq!(
        PHYSICAL_M1_RECOVERY_IDENTITY_DERIVATION_LABEL,
        "PHYSICAL_M1_NATIVE_RECOVERY_IDENTITY_DERIVATION_V1",
    );

    assert_eq!(BIP39_SEED_V1_PBKDF2_ROUNDS, 2_048,);

    assert_eq!(BIP39_SEED_V1_SALT_PREFIX, "mnemonic",);

    assert_eq!(BIP39_SEED_V1_PASSPHRASE_PROFILE, "empty_string_v1",);

    let factor = NativeSecretBytes::new(vec![0u8; 32]).expect("zero recovery factor fixture");

    let identity =
        derive_native_recovery_public_identity_v1(&factor).expect("zero-factor identity");

    assert_eq!(
        identity.root_public_key.as_str(),
        ZERO_FACTOR_ROOT_PUBLIC_KEY,
    );

    let expected_key =
        Ed25519PublicKeyHex::parse(ZERO_FACTOR_ROOT_PUBLIC_KEY).expect("expected root public key");

    let expected_id = derive_native_passport_id_v1(&expected_key).expect("expected Passport ID");

    assert_eq!(identity.passport_id, expected_id,);

    assert!(identity.optional_handle.is_none(),);
}

#[test]
fn max_recovery_factor_reproduces_independent_bip39_root_identity() {
    let factor = NativeSecretBytes::new(vec![0xffu8; 32]).expect("max recovery factor fixture");

    let identity = derive_native_recovery_public_identity_v1(&factor).expect("max-factor identity");

    assert_eq!(
        identity.root_public_key.as_str(),
        MAX_FACTOR_ROOT_PUBLIC_KEY,
    );
}

#[test]
fn recovery_identity_derivation_is_deterministic() {
    let bytes: Vec<u8> = (0u8..32u8).collect();

    let first = NativeSecretBytes::new(bytes.clone()).expect("first factor");

    let second = NativeSecretBytes::new(bytes).expect("second factor");

    assert_eq!(
        derive_native_recovery_public_identity_v1(&first,).expect("first identity"),
        derive_native_recovery_public_identity_v1(&second,).expect("second identity"),
    );
}

#[test]
fn malformed_recovery_factor_fails_closed() {
    let factor = NativeSecretBytes::new(vec![9u8; 31]).expect("bounded malformed fixture");

    assert_eq!(
        derive_native_recovery_public_identity_v1(&factor,),
        Err(NativeRecoveryIdentityDerivationError::RecoveryFactorInvalid,),
    );
}
