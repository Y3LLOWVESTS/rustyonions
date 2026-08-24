//! RO:WHAT — Purpose-specific Native Passport V1 root-registration proof signing boundary.
//! RO:WHY — RegisterRoot must prove control of the concrete Passport root key without exposing a generic root signing capability.
//! RO:INTERACTS — root_identity transient root-key custody, ron-auth canonical root-registration proof transcript and strict verifier, proof_signing_adapter signature representation, and future recovery-root orchestration/server registration.
//! RO:INVARIANTS — the derived Passport ID and public root must exactly match the transcript before signing; only the canonical ron-auth transcript is signed; the produced signature is strictly reverified before return; no arbitrary-message signing.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — the supplied BIP-39 seed remains borrowed native secret material; derived signing material remains transient inside root_identity; only public root identity plus a public signature leave this function; no persistence, HTTP, replay, capability, username, wallet, or ledger mutation.
//! RO:TEST — tests/physical_m1_native_root_registration_proof_signing.rs.

use ron_auth::native_passport::{
    verify_root_registration_proof_v1_strict, RootRegistrationProofTranscriptV1,
};

use super::{
    derive_native_root_public_identity_v1,
    root_identity::sign_native_root_registration_proof_payload_v1, NativeProofSignedPayloadHex,
    NativeSecretBytes, RootPassportDescriptorV1,
};

pub const PHYSICAL_M1_ROOT_REGISTRATION_PROOF_SIGNING_LABEL: &str =
    "PHYSICAL_M1_NATIVE_ROOT_REGISTRATION_PROOF_SIGNING_V1";

/// Public result of a successful purpose-specific root-registration proof.
///
/// The root descriptor and signature are public cryptographic evidence. No
/// secret root, recovery, PIN, seed, or vault material is returned.
pub struct NativeRootRegistrationProofSigningOutputV1 {
    pub root_identity: RootPassportDescriptorV1,
    pub signed_payload_hex: NativeProofSignedPayloadHex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeRootRegistrationProofSigningError {
    RootIdentityDerivationFailed,
    PassportBindingMismatch,
    RootPublicKeyBindingMismatch,
    RootProofSigningFailed,
    StrictVerificationFailed,
    SignedPayloadEncodingFailed,
}

/// Produce one real Native Passport V1 RegisterRoot signature.
///
/// This operation is intentionally narrower than the generic Phase-9 adapter:
/// it accepts only the typed ron-auth root-registration transcript, derives the
/// real root identity from existing custody, requires the transcript to name
/// that exact Passport/root pair, signs canonical bytes, and strictly verifies
/// the signature before returning public evidence.
pub fn sign_native_root_registration_proof_v1(
    bip39_seed: &NativeSecretBytes,
    transcript_input: &RootRegistrationProofTranscriptV1<'_>,
) -> Result<NativeRootRegistrationProofSigningOutputV1, NativeRootRegistrationProofSigningError> {
    let root_identity = derive_native_root_public_identity_v1(bip39_seed)
        .map_err(|_| NativeRootRegistrationProofSigningError::RootIdentityDerivationFailed)?;

    if root_identity.passport_id.as_str() != transcript_input.passport_id.as_str() {
        return Err(NativeRootRegistrationProofSigningError::PassportBindingMismatch);
    }

    if root_identity.root_public_key.as_str() != transcript_input.root_public_key.as_str() {
        return Err(NativeRootRegistrationProofSigningError::RootPublicKeyBindingMismatch);
    }

    let signature = sign_native_root_registration_proof_payload_v1(bip39_seed, transcript_input)
        .map_err(|_| NativeRootRegistrationProofSigningError::RootProofSigningFailed)?;

    verify_root_registration_proof_v1_strict(transcript_input, &signature)
        .map_err(|_| NativeRootRegistrationProofSigningError::StrictVerificationFailed)?;

    let signed_payload_hex = NativeProofSignedPayloadHex::parse(
        "root_registration_proof_signed_payload_hex",
        lower_hex(&signature),
    )
    .map_err(|_| NativeRootRegistrationProofSigningError::SignedPayloadEncodingFailed)?;

    Ok(NativeRootRegistrationProofSigningOutputV1 {
        root_identity,
        signed_payload_hex,
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
