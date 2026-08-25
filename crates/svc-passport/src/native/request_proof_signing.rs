//! RO:WHAT — Purpose-specific native DeviceKey signing for the CN-4 protected username PassportRequestProofV1.
//! RO:WHY — CrabLink must authorize the exact protected profile claim without exporting DeviceKey material or gaining a generic signer.
//! RO:INTERACTS — NativeSecretBytes, native device identity, strict native→ron-proto identity reparsing, ron-auth request-proof canonicalization/verification, and PassportRequestProofV1.
//! RO:INVARIANTS — 32-byte DeviceKey seed; Device ID derives from that seed; POST profile-claim purpose and empty query are fixed; only canonical ron-auth bytes are signed.
//! RO:METRICS — none; pure native signing primitive.
//! RO:CONFIG — compiled only through the native-passport feature; protected path is fixed to the CN-4 public profile-claim contract.
//! RO:SECURITY — no generic-message signing, seed export, HTTP, capability issuance, replay mutation, namespace mutation, RecoveryRoot, wallet, or ledger authority.
//! RO:TEST — focused module tests prove exact-purpose signing, stale-path rejection, query rejection, device binding, and strict cross-verification.

#![forbid(unsafe_code)]

use ed25519_dalek::{Signer as _, SigningKey};
use ron_auth::native_passport::{
    canonical_passport_request_proof_v1_transcript, verify_passport_request_proof_v1_strict,
    PassportRequestProofVerificationContextV1,
};
use ron_proto::{
    DeviceIdV1 as ProtoDeviceIdV1, Ed25519PublicKeyHex as ProtoEd25519PublicKeyHex,
    Ed25519SignatureV1, PassportRequestProofV1,
};
use zeroize::Zeroizing;

use super::{
    derive_native_device_public_identity_v1, NativeSecretBytes, DEVICE_ID_V1_SIGNING_SEED_BYTES,
};

pub const NATIVE_USERNAME_CLAIM_REQUEST_METHOD_V1: &str = "POST";

pub const NATIVE_USERNAME_CLAIM_CANONICAL_PATH_V1: &str = "/identity/passport/profile/claim";

pub const PHYSICAL_M1_USERNAME_REQUEST_PROOF_SIGNING_LABEL: &str =
    "PHYSICAL_M1_NATIVE_USERNAME_REQUEST_PROOF_SIGNING_V1";

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NativeUsernameRequestProofSigningError {
    InvalidDeviceSigningSeedLength { actual: usize, expected: usize },
    InvalidRequestProof,
    PurposeMismatch,
    DeviceIdentityDerivationFailed,
    ProtocolIdentityConversionFailed,
    DeviceBindingMismatch,
    TranscriptRejected,
    ProducedSignatureRejected,
}

/// Convert the already-derived native public device identity back through the
/// strict canonical ron-proto parsers used by request-proof wire contracts.
///
/// svc-passport still carries an older native DTO wrapper for historical
/// Native Passport phases. Request proofs use ron-proto as wire truth, so this
/// boundary reparses canonical text instead of treating the two Rust newtypes
/// as interchangeable or weakening either validator.
fn canonical_request_proof_device_identity(
    device_signing_seed: &NativeSecretBytes,
) -> Result<(ProtoDeviceIdV1, ProtoEd25519PublicKeyHex), NativeUsernameRequestProofSigningError> {
    let native_identity = derive_native_device_public_identity_v1(device_signing_seed)
        .map_err(|_| NativeUsernameRequestProofSigningError::DeviceIdentityDerivationFailed)?;

    let device_id = ProtoDeviceIdV1::parse(native_identity.device_id.as_str())
        .map_err(|_| NativeUsernameRequestProofSigningError::ProtocolIdentityConversionFailed)?;

    let device_public_key = ProtoEd25519PublicKeyHex::parse(
        native_identity.device_public_key.as_str(),
    )
    .map_err(|_| NativeUsernameRequestProofSigningError::ProtocolIdentityConversionFailed)?;

    Ok((device_id, device_public_key))
}

/// Sign exactly one CN-4 protected username/profile request proof.
///
/// The caller supplies the canonical request-proof DTO with an empty placeholder
/// signature. This primitive fixes the request purpose, requires the canonical
/// empty-query digest, proves that the Device ID derives from the supplied
/// native seed, signs only ron-auth canonical transcript bytes, and strictly
/// verifies the resulting public signature before returning it.
///
/// Only the public Ed25519 signature leaves this function.
pub fn sign_native_username_claim_request_proof_v1(
    device_signing_seed: &NativeSecretBytes,
    proof: &PassportRequestProofV1,
) -> Result<Ed25519SignatureV1, NativeUsernameRequestProofSigningError> {
    if device_signing_seed.len() != DEVICE_ID_V1_SIGNING_SEED_BYTES {
        return Err(
            NativeUsernameRequestProofSigningError::InvalidDeviceSigningSeedLength {
                actual: device_signing_seed.len(),
                expected: DEVICE_ID_V1_SIGNING_SEED_BYTES,
            },
        );
    }

    proof
        .validate()
        .map_err(|_| NativeUsernameRequestProofSigningError::InvalidRequestProof)?;

    let empty_query_hash = blake3::hash(b"").to_hex();

    if proof.request_method != NATIVE_USERNAME_CLAIM_REQUEST_METHOD_V1
        || proof.canonical_path != NATIVE_USERNAME_CLAIM_CANONICAL_PATH_V1
        || proof.canonical_query_hash.as_str() != empty_query_hash.as_str()
    {
        return Err(NativeUsernameRequestProofSigningError::PurposeMismatch);
    }

    let (trusted_device_id, trusted_device_public_key) =
        canonical_request_proof_device_identity(device_signing_seed)?;

    if trusted_device_id != proof.device_id {
        return Err(NativeUsernameRequestProofSigningError::DeviceBindingMismatch);
    }

    let canonical = canonical_passport_request_proof_v1_transcript(proof)
        .map_err(|_| NativeUsernameRequestProofSigningError::TranscriptRejected)?;

    let mut seed = Zeroizing::new([0_u8; DEVICE_ID_V1_SIGNING_SEED_BYTES]);
    seed.copy_from_slice(device_signing_seed.as_slice());

    let signing_key = SigningKey::from_bytes(&seed);

    let signature = Ed25519SignatureV1::from_bytes(signing_key.sign(&canonical).to_bytes());

    let mut signed_proof = proof.clone();
    signed_proof.device_signature = signature.clone();

    verify_passport_request_proof_v1_strict(
        &signed_proof,
        PassportRequestProofVerificationContextV1 {
            trusted_device_public_key: &trusted_device_public_key,
            expected_capability_id: &signed_proof.capability_id,
            expected_device_id: &signed_proof.device_id,
            expected_request_method: NATIVE_USERNAME_CLAIM_REQUEST_METHOD_V1,
            expected_canonical_path: NATIVE_USERNAME_CLAIM_CANONICAL_PATH_V1,
            expected_canonical_query_hash: &signed_proof.canonical_query_hash,
            expected_body_hash: &signed_proof.body_hash,
            now_ms: signed_proof.timestamp_ms,
            max_clock_skew_ms: 0,
        },
    )
    .map_err(|_| NativeUsernameRequestProofSigningError::ProducedSignatureRejected)?;

    Ok(signature)
}

#[cfg(test)]
mod tests {
    use ron_auth::native_passport::{
        verify_passport_request_proof_v1_strict, PassportRequestProofVerificationContextV1,
    };
    use ron_proto::{
        B3DigestHex, CapabilityIdV1, DeviceIdV1, Ed25519SignatureV1, PassportRequestProofV1,
        PASSPORT_REQUEST_PROOF_V1_VERSION,
    };

    use super::*;

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn device_seed(byte: u8) -> NativeSecretBytes {
        NativeSecretBytes::new(vec![byte; DEVICE_ID_V1_SIGNING_SEED_BYTES])
            .expect("test DeviceKey seed")
    }

    fn digest(field: &'static str, value: &str) -> B3DigestHex {
        B3DigestHex::parse(field, value).expect("test digest")
    }

    fn empty_query_hash() -> B3DigestHex {
        let hash = blake3::hash(b"").to_hex();

        B3DigestHex::parse("canonical_query_hash", hash.as_str()).expect("empty query digest")
    }

    fn unsigned_proof(device_id: DeviceIdV1) -> PassportRequestProofV1 {
        PassportRequestProofV1 {
            version: PASSPORT_REQUEST_PROOF_V1_VERSION,
            capability_id: CapabilityIdV1::parse(format!("capability:v1:b3:{HEX_A}"))
                .expect("capability ID"),
            request_method: NATIVE_USERNAME_CLAIM_REQUEST_METHOD_V1.to_owned(),
            canonical_path: NATIVE_USERNAME_CLAIM_CANONICAL_PATH_V1.to_owned(),
            canonical_query_hash: empty_query_hash(),
            body_hash: digest("body_hash", HEX_C),
            timestamp_ms: 1_000_000,
            request_nonce: digest("request_nonce", HEX_D),
            device_id,
            device_signature: Ed25519SignatureV1::from_bytes([0_u8; 64]),
        }
    }

    #[test]
    fn exact_profile_claim_request_proof_signs_and_cross_verifies() {
        let seed = device_seed(0x63);

        let (device_id, device_public_key) =
            canonical_request_proof_device_identity(&seed).expect("canonical device identity");

        let mut proof = unsigned_proof(device_id);

        proof.device_signature = sign_native_username_claim_request_proof_v1(&seed, &proof)
            .expect("purpose-specific request proof signature");

        verify_passport_request_proof_v1_strict(
            &proof,
            PassportRequestProofVerificationContextV1 {
                trusted_device_public_key: &device_public_key,
                expected_capability_id: &proof.capability_id,
                expected_device_id: &proof.device_id,
                expected_request_method: NATIVE_USERNAME_CLAIM_REQUEST_METHOD_V1,
                expected_canonical_path: NATIVE_USERNAME_CLAIM_CANONICAL_PATH_V1,
                expected_canonical_query_hash: &proof.canonical_query_hash,
                expected_body_hash: &proof.body_hash,
                now_ms: proof.timestamp_ms,
                max_clock_skew_ms: 0,
            },
        )
        .expect("strict ron-auth cross-verification");
    }

    #[test]
    fn stale_username_claim_path_is_not_signable() {
        let seed = device_seed(0x63);

        let (device_id, _) =
            canonical_request_proof_device_identity(&seed).expect("canonical device identity");

        let mut proof = unsigned_proof(device_id);

        proof.canonical_path = "/identity/passport/username/claim".to_owned();

        assert_eq!(
            sign_native_username_claim_request_proof_v1(&seed, &proof),
            Err(NativeUsernameRequestProofSigningError::PurposeMismatch),
        );
    }

    #[test]
    fn nonempty_query_binding_is_not_signable() {
        let seed = device_seed(0x63);

        let (device_id, _) =
            canonical_request_proof_device_identity(&seed).expect("canonical device identity");

        let mut proof = unsigned_proof(device_id);

        proof.canonical_query_hash = digest("canonical_query_hash", HEX_C);

        assert_eq!(
            sign_native_username_claim_request_proof_v1(&seed, &proof),
            Err(NativeUsernameRequestProofSigningError::PurposeMismatch),
        );
    }

    #[test]
    fn foreign_device_id_is_not_signable() {
        let seed = device_seed(0x63);
        let other_seed = device_seed(0x64);

        let (other_device_id, _) = canonical_request_proof_device_identity(&other_seed)
            .expect("other canonical device identity");

        let proof = unsigned_proof(other_device_id);

        assert_eq!(
            sign_native_username_claim_request_proof_v1(&seed, &proof),
            Err(NativeUsernameRequestProofSigningError::DeviceBindingMismatch),
        );
    }
}
