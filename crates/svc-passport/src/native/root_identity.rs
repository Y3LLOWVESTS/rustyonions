//! RO:WHAT — Owns Native Passport V1 root-key derivation, public-root derivation, and the narrow DeviceAuthorizationV1 root-signing operation.
//! RO:WHY — Physical M1 requires the recovery-derived root key to authorize devices without exporting the root signing seed or creating a generic ambient signing capability.
//! RO:INTERACTS — Phase 0C HKDF constants, NativeSecretBytes, Ed25519, RootPassportDescriptorV1, Passport-ID derivation, ron-proto DeviceAuthorizationSigningPayloadV1, and ron-auth canonical authorization transcript construction.
//! RO:INVARIANTS — exact HKDF-SHA256 salt/info; 64-byte BIP-39 seed; temporary 32-byte Ed25519 signing seed; public identity derivation unchanged; the only sibling-visible signing primitive is typed specifically to DeviceAuthorizationV1.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the native-passport feature.
//! RO:SECURITY — root signing seed remains temporary zeroizing native memory and is never returned, serialized, logged, persisted, placed in a DTO, or exposed to frontend code; no filesystem, Keychain, network, wallet, ledger, capability, or username authority.
//! RO:TEST — tests/physical_m1_native_root_identity_derivation.rs and tests/physical_m1_native_device_authorization_signing.rs.

use ed25519_dalek::{Signer as _, SigningKey};
use hkdf::Hkdf;
use ron_auth::native_passport::{
    canonical_device_authorization_v1_transcript, canonical_root_registration_proof_v1_transcript,
    RootRegistrationProofTranscriptV1,
};
use ron_proto::DeviceAuthorizationSigningPayloadV1;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::native_plan::{
    ROOT_IDENTITY_V1_BIP39_SEED_BYTES, ROOT_IDENTITY_V1_HKDF_INFO, ROOT_IDENTITY_V1_HKDF_SALT,
    ROOT_IDENTITY_V1_SIGNING_SEED_BYTES,
};

use super::{
    derive_native_passport_id_v1, Ed25519PublicKeyHex, NativeSecretBytes, RootPassportDescriptorV1,
};

pub const PHYSICAL_M1_ROOT_IDENTITY_DERIVATION_LABEL: &str =
    "PHYSICAL_M1_NATIVE_ROOT_IDENTITY_DERIVATION_V1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeRootIdentityDerivationError {
    InvalidBip39SeedLength { actual: usize, expected: usize },
    HkdfExpandFailed,
    InvalidRootPublicKey,
    InvalidPassportId,
}

/// Internal failure for the purpose-specific root DeviceAuthorization signer.
///
/// This type deliberately stays inside the native module; callers receive the
/// higher-level device-authorization signing taxonomy instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NativeRootDeviceAuthorizationSigningError {
    InvalidAuthorizationTranscript,
    RootIdentityDerivationFailed,
}

/// Derive the public Passport descriptor without exporting root secret material.
pub fn derive_native_root_public_identity_v1(
    bip39_seed: &NativeSecretBytes,
) -> Result<RootPassportDescriptorV1, NativeRootIdentityDerivationError> {
    with_native_root_signing_key_v1(bip39_seed, |signing_key| {
        derive_public_identity_from_root_signing_key_v1(signing_key)
    })?
}

/// Sign exactly the ron-auth canonical DeviceAuthorizationV1 transcript.
///
/// This is intentionally `pub(super)`: sibling Native Passport orchestration
/// may authorize a device, but no public raw-message root-signing primitive is
/// exposed from svc-passport.
pub(super) fn sign_native_root_device_authorization_payload_v1(
    bip39_seed: &NativeSecretBytes,
    payload: &DeviceAuthorizationSigningPayloadV1,
) -> Result<[u8; 64], NativeRootDeviceAuthorizationSigningError> {
    let transcript = canonical_device_authorization_v1_transcript(payload)
        .map_err(|_| NativeRootDeviceAuthorizationSigningError::InvalidAuthorizationTranscript)?;

    with_native_root_signing_key_v1(bip39_seed, |signing_key| {
        signing_key.sign(&transcript).to_bytes()
    })
    .map_err(|_| NativeRootDeviceAuthorizationSigningError::RootIdentityDerivationFailed)
}

/// Internal failure for the purpose-specific root-registration proof signer.
///
/// The lower signer remains sibling-visible only. Callers use the higher-level
/// root-registration proof signing API, which checks the concrete public root
/// and Passport binding and strictly verifies the produced signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NativeRootRegistrationProofPayloadSigningError {
    InvalidProofTranscript,
    RootIdentityDerivationFailed,
}

/// Sign exactly the ron-auth canonical Native Passport V1 root-registration
/// proof transcript.
///
/// This is deliberately `pub(super)`. It is not a general root-message signer
/// and cannot be used to sign arbitrary caller bytes.
pub(super) fn sign_native_root_registration_proof_payload_v1(
    bip39_seed: &NativeSecretBytes,
    transcript_input: &RootRegistrationProofTranscriptV1<'_>,
) -> Result<[u8; 64], NativeRootRegistrationProofPayloadSigningError> {
    let transcript = canonical_root_registration_proof_v1_transcript(transcript_input)
        .map_err(|_| NativeRootRegistrationProofPayloadSigningError::InvalidProofTranscript)?;

    with_native_root_signing_key_v1(bip39_seed, |signing_key| {
        signing_key.sign(&transcript).to_bytes()
    })
    .map_err(|_| NativeRootRegistrationProofPayloadSigningError::RootIdentityDerivationFailed)
}

/// Single implementation of the frozen Phase 0C HKDF root-key derivation.
///
/// The signing key exists only for the duration of `operation`. The derived
/// 32-byte seed is zeroized on drop and neither value leaves this module.
fn with_native_root_signing_key_v1<T>(
    bip39_seed: &NativeSecretBytes,
    operation: impl FnOnce(&SigningKey) -> T,
) -> Result<T, NativeRootIdentityDerivationError> {
    if bip39_seed.len() != ROOT_IDENTITY_V1_BIP39_SEED_BYTES {
        return Err(NativeRootIdentityDerivationError::InvalidBip39SeedLength {
            actual: bip39_seed.len(),
            expected: ROOT_IDENTITY_V1_BIP39_SEED_BYTES,
        });
    }

    let hkdf = Hkdf::<Sha256>::new(
        Some(ROOT_IDENTITY_V1_HKDF_SALT.as_bytes()),
        bip39_seed.as_slice(),
    );

    let mut root_signing_seed = Zeroizing::new([0u8; ROOT_IDENTITY_V1_SIGNING_SEED_BYTES]);

    hkdf.expand(
        ROOT_IDENTITY_V1_HKDF_INFO.as_bytes(),
        &mut *root_signing_seed,
    )
    .map_err(|_| NativeRootIdentityDerivationError::HkdfExpandFailed)?;

    let signing_key = SigningKey::from_bytes(&root_signing_seed);

    Ok(operation(&signing_key))
}

fn derive_public_identity_from_root_signing_key_v1(
    signing_key: &SigningKey,
) -> Result<RootPassportDescriptorV1, NativeRootIdentityDerivationError> {
    let verifying_key = signing_key.verifying_key();

    let root_public_key = Ed25519PublicKeyHex::parse(lower_hex(&verifying_key.to_bytes()))
        .map_err(|_| NativeRootIdentityDerivationError::InvalidRootPublicKey)?;

    let passport_id = derive_native_passport_id_v1(&root_public_key)
        .map_err(|_| NativeRootIdentityDerivationError::InvalidPassportId)?;

    Ok(RootPassportDescriptorV1 {
        passport_id,
        root_public_key,
        optional_handle: None,
    })
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }

    output
}
