//! RO:WHAT — Derives canonical public Passport identity plus purpose-specific DeviceAuthorizationV1 and RegisterRoot proof signatures from the existing 256-bit recovery factor.
//! RO:WHY — Physical M1 needs root authorization without exposing a BIP-39 seed or root signing key to CrabLink/Tauri orchestration.
//! RO:INTERACTS — recovery mnemonic indices/words, BIP-39 PBKDF2-HMAC-SHA512 seed derivation, root identity derivation, device_authorization_signing, root_registration_proof_signing, ron-auth typed RegisterRoot transcripts, and ron-proto DeviceAuthorizationV1.
//! RO:INVARIANTS — only the internally generated canonical 24-word English mnemonic is accepted; the frozen empty BIP-39 passphrase and 2,048 PBKDF2 rounds remain unchanged; the transient 64-byte BIP-39 seed never crosses this module's public API; purpose-specific typed signers remain responsible for Passport/root/device binding.
//! RO:METRICS — none.
//! RO:CONFIG — native-passport feature only.
//! RO:SECURITY — recovery factor, phrase, PBKDF2 intermediates, BIP-39 seed, and root signing material remain native-only and zeroizing; only public signed DeviceAuthorizationV1 or RegisterRoot proof evidence may be returned; no vault, platform sealer, Tauri, HTTP, registry, replay, capability, username, wallet, or ledger mutation occurs here.
//! RO:TEST — tests/physical_m1_native_recovery_identity_derivation.rs, physical_m1_native_recovery_device_authorization_signing.rs, and physical_m1_native_recovery_root_registration_proof_signing.rs.

use hmac::{Hmac, Mac};
use sha2::Sha512;
use zeroize::Zeroizing;

use ron_auth::native_passport::RootRegistrationProofTranscriptV1;
use ron_proto::{DeviceAuthorizationSigningPayloadV1, DeviceAuthorizationV1};

use crate::native_plan::{
    BIP39_SEED_V1_PASSPHRASE_PROFILE, BIP39_SEED_V1_PBKDF2_ROUNDS, BIP39_SEED_V1_SALT_PREFIX,
    ROOT_IDENTITY_V1_BIP39_SEED_BYTES,
};

use super::{
    derive_native_recovery_mnemonic_indices, derive_native_root_public_identity_v1,
    device_authorization_signing::{
        sign_native_device_authorization_v1, NativeDeviceAuthorizationSigningError,
    },
    root_registration_proof_signing::{
        sign_native_root_registration_proof_v1, NativeRootRegistrationProofSigningError,
        NativeRootRegistrationProofSigningOutputV1,
    },
    with_native_recovery_mnemonic_phrase, NativeSecretBytes, RootPassportDescriptorV1,
};

type HmacSha512 = Hmac<Sha512>;

pub const PHYSICAL_M1_RECOVERY_IDENTITY_DERIVATION_LABEL: &str =
    "PHYSICAL_M1_NATIVE_RECOVERY_IDENTITY_DERIVATION_V1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeRecoveryIdentityDerivationError {
    RecoveryFactorInvalid,
    MnemonicWordMappingFailed,
    InvalidCanonicalMnemonic,
    HmacInitializationFailed,
    SeedBufferRejected,
    RootIdentityDerivationFailed,
}

/// Derive the public Passport descriptor from the existing Native Passport
/// recovery factor without returning recovery or root-secret material.
///
/// This is deliberately narrower than a general BIP-39 import API. The phrase
/// originates only from the locked internal English wordlist and the frozen
/// Native Passport V1 passphrase is empty.
pub fn derive_native_recovery_public_identity_v1(
    recovery_factor: &NativeSecretBytes,
) -> Result<RootPassportDescriptorV1, NativeRecoveryIdentityDerivationError> {
    let indices = derive_native_recovery_mnemonic_indices(recovery_factor)
        .map_err(|_| NativeRecoveryIdentityDerivationError::RecoveryFactorInvalid)?;

    with_native_recovery_mnemonic_phrase(&indices, |phrase, _fingerprint| {
        let bip39_seed = derive_bip39_seed_from_canonical_phrase(phrase)?;

        derive_native_root_public_identity_v1(&bip39_seed)
            .map_err(|_| NativeRecoveryIdentityDerivationError::RootIdentityDerivationFailed)
    })
    .map_err(|_| NativeRecoveryIdentityDerivationError::MnemonicWordMappingFailed)?
}

pub const PHYSICAL_M1_RECOVERY_DEVICE_AUTHORIZATION_SIGNING_LABEL: &str =
    "PHYSICAL_M1_NATIVE_RECOVERY_DEVICE_AUTHORIZATION_SIGNING_V1";

/// Failures while turning the Native Passport recovery factor into one
/// purpose-specific, public DeviceAuthorizationV1.
///
/// The detailed root/device binding failure remains the existing
/// `NativeDeviceAuthorizationSigningError`; no secret-derived material is
/// included in this error surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NativeRecoveryDeviceAuthorizationSigningError {
    RecoveryFactorInvalid,
    MnemonicWordMappingFailed,
    Bip39SeedDerivationFailed,
    DeviceAuthorizationSigning(NativeDeviceAuthorizationSigningError),
}

/// Sign one canonical DeviceAuthorizationV1 directly from the recovery factor.
///
/// The recovery mnemonic and BIP-39 seed exist only inside this synchronous
/// native call. The seed is passed immediately to the existing purpose-specific
/// DeviceAuthorization signer and is dropped before this function returns.
/// The only successful output is the public signed authorization record.
pub fn sign_native_recovery_device_authorization_v1(
    recovery_factor: &NativeSecretBytes,
    payload: DeviceAuthorizationSigningPayloadV1,
) -> Result<DeviceAuthorizationV1, NativeRecoveryDeviceAuthorizationSigningError> {
    let indices = derive_native_recovery_mnemonic_indices(recovery_factor)
        .map_err(|_| NativeRecoveryDeviceAuthorizationSigningError::RecoveryFactorInvalid)?;

    with_native_recovery_mnemonic_phrase(&indices, move |phrase, _fingerprint| {
        let bip39_seed = derive_bip39_seed_from_canonical_phrase(phrase).map_err(|_| {
            NativeRecoveryDeviceAuthorizationSigningError::Bip39SeedDerivationFailed
        })?;

        sign_native_device_authorization_v1(&bip39_seed, payload)
            .map_err(NativeRecoveryDeviceAuthorizationSigningError::DeviceAuthorizationSigning)
    })
    .map_err(|_| NativeRecoveryDeviceAuthorizationSigningError::MnemonicWordMappingFailed)?
}

pub const PHYSICAL_M1_RECOVERY_ROOT_REGISTRATION_PROOF_SIGNING_LABEL: &str =
    "PHYSICAL_M1_NATIVE_RECOVERY_ROOT_REGISTRATION_PROOF_SIGNING_V1";

/// Failures while converting the recovery factor into one purpose-specific,
/// public RegisterRoot proof.
///
/// No recovery-derived secret material is included in this error surface.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NativeRecoveryRootRegistrationProofSigningError {
    RecoveryFactorInvalid,
    MnemonicWordMappingFailed,
    Bip39SeedDerivationFailed,
    RootRegistrationProofSigning(NativeRootRegistrationProofSigningError),
}

/// Sign exactly one canonical RegisterRoot proof from the existing recovery
/// factor.
///
/// The recovery phrase and BIP-39 seed remain inside this synchronous native
/// call. The transient seed is passed directly to the already-reviewed
/// purpose-specific root-registration signer and is dropped before return.
/// Only public root identity and public Ed25519 proof evidence may leave.
pub fn sign_native_recovery_root_registration_proof_v1(
    recovery_factor: &NativeSecretBytes,
    transcript_input: &RootRegistrationProofTranscriptV1<'_>,
) -> Result<
    NativeRootRegistrationProofSigningOutputV1,
    NativeRecoveryRootRegistrationProofSigningError,
> {
    let indices = derive_native_recovery_mnemonic_indices(recovery_factor)
        .map_err(|_| NativeRecoveryRootRegistrationProofSigningError::RecoveryFactorInvalid)?;

    with_native_recovery_mnemonic_phrase(&indices, |phrase, _fingerprint| {
        let bip39_seed = derive_bip39_seed_from_canonical_phrase(phrase).map_err(|_| {
            NativeRecoveryRootRegistrationProofSigningError::Bip39SeedDerivationFailed
        })?;

        sign_native_root_registration_proof_v1(&bip39_seed, transcript_input)
            .map_err(NativeRecoveryRootRegistrationProofSigningError::RootRegistrationProofSigning)
    })
    .map_err(|_| NativeRecoveryRootRegistrationProofSigningError::MnemonicWordMappingFailed)?
}

fn derive_bip39_seed_from_canonical_phrase(
    phrase: &str,
) -> Result<NativeSecretBytes, NativeRecoveryIdentityDerivationError> {
    if phrase.split_ascii_whitespace().count() != 24
        || !phrase
            .bytes()
            .all(|byte| byte == b' ' || byte.is_ascii_lowercase())
    {
        return Err(NativeRecoveryIdentityDerivationError::InvalidCanonicalMnemonic);
    }

    debug_assert_eq!(BIP39_SEED_V1_PASSPHRASE_PROFILE, "empty_string_v1",);

    let base = HmacSha512::new_from_slice(phrase.as_bytes())
        .map_err(|_| NativeRecoveryIdentityDerivationError::HmacInitializationFailed)?;

    // dkLen is exactly one SHA-512 PRF block (64 bytes), so BIP-39 PBKDF2
    // requires only block index 1.
    let mut first = base.clone();

    first.update(BIP39_SEED_V1_SALT_PREFIX.as_bytes());

    first.update(&1u32.to_be_bytes());

    let first_output = first.finalize().into_bytes();

    let mut u = Zeroizing::new([0u8; ROOT_IDENTITY_V1_BIP39_SEED_BYTES]);

    u.copy_from_slice(first_output.as_slice());

    let mut derived = Zeroizing::new([0u8; ROOT_IDENTITY_V1_BIP39_SEED_BYTES]);

    derived.copy_from_slice(&*u);

    for _ in 1..BIP39_SEED_V1_PBKDF2_ROUNDS {
        let mut round = base.clone();

        round.update(&*u);

        let output = round.finalize().into_bytes();

        u.copy_from_slice(output.as_slice());

        for (accumulated, current) in derived.iter_mut().zip(u.iter()) {
            *accumulated ^= *current;
        }
    }

    NativeSecretBytes::new(derived.to_vec())
        .map_err(|_| NativeRecoveryIdentityDerivationError::SeedBufferRejected)
}
