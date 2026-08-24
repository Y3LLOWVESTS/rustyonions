//! RO:WHAT — Focused Physical M1 verification of the Native Passport RecoveryRoot PIN.
//! RO:WHY — Root-confirmed identity finalization must authenticate the native PIN before it may derive or persist canonical public identity.
//! RO:INTERACTS — RecoveryRoot VMK wrapping, XChaCha20-Poly1305 authenticated decryption, Argon2id PIN derivation, HKDF compartment binding, and NativeSecretBytes zeroizing custody.
//! RO:INVARIANTS — only RecoveryRoot envelopes are accepted; a wrong PIN fails authentication; successful verification returns no VMK or root secret.
//! RO:SECURITY — test fixtures only; no PlatformSealer, filesystem, Tauri, username, wallet, ledger, signing, or public root-unlock session.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test physical_m1_native_recovery_root_pin_verification.

#![cfg(feature = "native-passport")]

use svc_passport::native::{
    verify_native_recovery_root_pin, wrap_native_compartment_vmk, NativePinWrappedCompartmentVmkV1,
    NativeSecretBytes, NativeSecureCompartment, NativeVaultCryptoError,
    PHASE15Q_PLATFORM_FACTOR_BYTES, PHASE15Q_VAULT_MASTER_KEY_BYTES, PHASE6A_AEAD_NONCE_LEN,
    PHASE6A_KDF_SALT_LEN,
};

const CORRECT_PIN: &[u8] = b"physical-m1-root-pin";

const WRONG_PIN: &[u8] = b"physical-m1-wrong-pin";

const ROOT_SALT: [u8; PHASE6A_KDF_SALT_LEN] = [0x31; PHASE6A_KDF_SALT_LEN];

const ROOT_NONCE: [u8; PHASE6A_AEAD_NONCE_LEN] = [0x42; PHASE6A_AEAD_NONCE_LEN];

const OPERATIONAL_SALT: [u8; PHASE6A_KDF_SALT_LEN] = [0x53; PHASE6A_KDF_SALT_LEN];

const OPERATIONAL_NONCE: [u8; PHASE6A_AEAD_NONCE_LEN] = [0x64; PHASE6A_AEAD_NONCE_LEN];

fn secret(byte: u8, length: usize) -> NativeSecretBytes {
    NativeSecretBytes::new(vec![byte; length]).expect("bounded test secret")
}

fn wrapped_root() -> (NativePinWrappedCompartmentVmkV1, NativeSecretBytes) {
    let factor = secret(0x71, PHASE15Q_PLATFORM_FACTOR_BYTES);

    let vmk = secret(0x82, PHASE15Q_VAULT_MASTER_KEY_BYTES);

    let envelope = wrap_native_compartment_vmk(
        NativeSecureCompartment::RecoveryRoot,
        CORRECT_PIN,
        &factor,
        &ROOT_SALT,
        &ROOT_NONCE,
        &vmk,
    )
    .expect("wrap root VMK");

    (envelope, factor)
}

#[test]
fn correct_recovery_root_pin_verifies_without_returning_secret() {
    let (envelope, factor) = wrapped_root();

    assert_eq!(
        verify_native_recovery_root_pin(&envelope, CORRECT_PIN, &factor,),
        Ok(()),
    );
}

#[test]
fn incorrect_recovery_root_pin_fails_authenticated_decryption() {
    let (envelope, factor) = wrapped_root();

    assert_eq!(
        verify_native_recovery_root_pin(&envelope, WRONG_PIN, &factor,),
        Err(NativeVaultCryptoError::AuthenticationFailed,),
    );
}

#[test]
fn operational_envelope_is_rejected_as_wrong_compartment() {
    let factor = secret(0x93, PHASE15Q_PLATFORM_FACTOR_BYTES);

    let vmk = secret(0xa4, PHASE15Q_VAULT_MASTER_KEY_BYTES);

    let envelope = wrap_native_compartment_vmk(
        NativeSecureCompartment::DeviceKey,
        CORRECT_PIN,
        &factor,
        &OPERATIONAL_SALT,
        &OPERATIONAL_NONCE,
        &vmk,
    )
    .expect("wrap operational VMK");

    assert_eq!(
        verify_native_recovery_root_pin(&envelope, CORRECT_PIN, &factor,),
        Err(NativeVaultCryptoError::UnexpectedCompartment {
            expected: NativeSecureCompartment::RecoveryRoot,
            actual: NativeSecureCompartment::DeviceKey,
        },),
    );
}

#[test]
fn invalid_pin_length_fails_before_authenticated_decryption() {
    let (envelope, factor) = wrapped_root();

    let result = verify_native_recovery_root_pin(&envelope, b"x", &factor);

    assert!(matches!(
        result,
        Err(NativeVaultCryptoError::InvalidPinLength { .. })
    ),);
}
