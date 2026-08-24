//! RO:WHAT — Canonical Native Passport V1 device-session possession-proof transcript construction and strict Ed25519 verification.
//! RO:WHY — CN-4 physical M1 must prove control of an already root-authorized device key before any device-bound capability or username/profile mutation can be admitted.
//! RO:INTERACTS — `ron-proto` Passport/Device/Challenge/B3/Ed25519 DTOs, the existing Native Passport challenge-proof domain, `svc-passport` durable registered-device state, and future native DeviceKey signing composition.
//! RO:INVARIANTS — only deterministic length-prefixed binary V1 bytes are signable; the proof is Passport-bound, device-bound, device-public-key-bound, challenge-hash-bound, scope-bound, and time-bound; ordinary JSON and historical pipe text are never signable.
//! RO:METRICS — none.
//! RO:CONFIG — protocol scope bounds come from `ron-proto`; no runtime configuration or I/O.
//! RO:SECURITY — public-data transcript construction and public-key verification only; no private keys, vault/PIN access, signing-key loading, challenge consumption, capability issuance, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — `tests/native_passport_device_session_proof.rs`.

#![forbid(unsafe_code)]

use ed25519_dalek::{Signature, VerifyingKey};
use ron_proto::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, Ed25519PublicKeyHex, Ed25519SignatureV1,
    NativePassportContextLabelV1, NativePassportScopeV1, PassportIdV1,
    DEVICE_AUTHORIZATION_V1_MAX_SCOPES,
};
use thiserror::Error;

use super::root_registration_proof::{
    ROOT_REGISTRATION_PROOF_V1_CANONICAL_TRANSCRIPT_ENCODING,
    ROOT_REGISTRATION_PROOF_V1_TRANSCRIPT_DOMAIN,
};

/// Frozen Native Passport V1 domain shared by canonical challenge proofs.
pub const DEVICE_SESSION_PROOF_V1_TRANSCRIPT_DOMAIN: &str =
    ROOT_REGISTRATION_PROOF_V1_TRANSCRIPT_DOMAIN;

/// Canonical binary encoding shared by real Native Passport V1 challenge proofs.
pub const DEVICE_SESSION_PROOF_V1_CANONICAL_TRANSCRIPT_ENCODING: &str =
    ROOT_REGISTRATION_PROOF_V1_CANONICAL_TRANSCRIPT_ENCODING;

/// Historical Phase-0 pipe-delimited bytes must never be signed as a device-session proof.
pub const DEVICE_SESSION_PROOF_V1_LEGACY_PHASE0_TRANSCRIPT_SIGNABLE: bool = false;

/// Ordinary JSON bytes must never be signed as a device-session proof.
pub const DEVICE_SESSION_PROOF_V1_JSON_TRANSCRIPT_SIGNABLE: bool = false;

const DEVICE_SESSION_PROOF_V1_TRANSCRIPT_VERSION: u16 = 1;
const DEVICE_SESSION_PROOF_V1_KIND: &str = "device-session";
const MAX_TRANSCRIPT_TEXT_BYTES: usize = 256;

/// Public inputs covered by one Native Passport V1 device-session possession proof.
///
/// The `device_public_key` passed here is not caller authority at the server.
/// `svc-passport` must later construct this input from the exact currently
/// authorized durable device record before calling the strict verifier.
#[derive(Debug, Clone, Copy)]
pub struct DeviceSessionProofTranscriptV1<'a> {
    pub challenge_contract_domain: &'a str,
    pub challenge_contract_version: u16,
    pub proof_contract_domain: &'a str,
    pub proof_contract_version: u16,
    pub challenge_id: &'a ChallengeIdV1,
    pub network_id: &'a NativePassportContextLabelV1,
    pub environment: &'a NativePassportContextLabelV1,
    pub audience: &'a NativePassportContextLabelV1,
    pub passport_id: &'a PassportIdV1,
    pub device_id: &'a DeviceIdV1,
    pub device_public_key: &'a Ed25519PublicKeyHex,
    pub challenge_transcript_hash: &'a B3DigestHex,
    pub requested_scopes: &'a [NativePassportScopeV1],
    pub challenge_issued_at_ms: u64,
    pub challenge_expires_at_ms: u64,
    pub proof_created_at_ms: u64,
}

/// Deterministic construction/verification errors for a device-session proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DeviceSessionProofError {
    #[error("device-session proof contract text is invalid")]
    InvalidContractText,

    #[error("device-session proof requires at least one scope")]
    MissingScopes,

    #[error("device-session proof scope count exceeds the protocol maximum")]
    TooManyScopes,

    #[error("device-session proof contains a duplicate scope")]
    DuplicateScope,

    #[error("device-session proof challenge time window is invalid")]
    InvalidChallengeTimeWindow,

    #[error("device-session proof creation time is outside the challenge window")]
    InvalidProofCreatedAt,

    #[error("device-session proof device public key is invalid")]
    InvalidDevicePublicKey,

    #[error("device-session proof challenge transcript hash is invalid")]
    InvalidChallengeTranscriptHash,

    #[error("device-session proof transcript field exceeds the binary V1 framing bound")]
    TranscriptFieldTooLong,

    #[error("device-session proof device public key is not a valid Ed25519 key")]
    InvalidEd25519DevicePublicKey,

    #[error("device-session proof signature mismatch")]
    InvalidSignature,
}

/// Build the exact deterministic bytes signed by an authorized device.
///
/// Layout:
///
/// ```text
/// lp(challenge-proof domain)
/// u16_be(transcript version)
/// lp("device-session")
/// lp(challenge contract domain)
/// u16_be(challenge contract version)
/// lp(proof contract domain)
/// u16_be(proof contract version)
/// lp(challenge id)
/// lp(network id)
/// lp(environment)
/// lp(audience)
/// lp(passport id)
/// lp(device id)
/// lp(raw 32-byte device public key)
/// lp(raw 32-byte challenge transcript hash)
/// u16_be(scope count)
/// repeated lp(scope), lexicographically canonicalized
/// u64_be(challenge issued at ms)
/// u64_be(challenge expires at ms)
/// u64_be(proof created at ms)
/// ```
///
/// `lp(x)` is `u16_be(len(x)) || x`.
pub fn canonical_device_session_proof_v1_transcript(
    input: &DeviceSessionProofTranscriptV1<'_>,
) -> Result<Vec<u8>, DeviceSessionProofError> {
    validate_input(input)?;

    let device_public_key = decode_lower_hex_32(input.device_public_key.as_str())
        .ok_or(DeviceSessionProofError::InvalidDevicePublicKey)?;

    let challenge_transcript_hash =
        decode_lower_hex_32(input.challenge_transcript_hash.as_str())
            .ok_or(DeviceSessionProofError::InvalidChallengeTranscriptHash)?;

    let scopes = canonical_scopes(input.requested_scopes)?;

    let mut transcript = Vec::with_capacity(1024);

    push_len_prefixed(
        &mut transcript,
        DEVICE_SESSION_PROOF_V1_TRANSCRIPT_DOMAIN.as_bytes(),
    )?;

    transcript.extend_from_slice(&DEVICE_SESSION_PROOF_V1_TRANSCRIPT_VERSION.to_be_bytes());

    push_len_prefixed(&mut transcript, DEVICE_SESSION_PROOF_V1_KIND.as_bytes())?;

    push_len_prefixed(&mut transcript, input.challenge_contract_domain.as_bytes())?;

    transcript.extend_from_slice(&input.challenge_contract_version.to_be_bytes());

    push_len_prefixed(&mut transcript, input.proof_contract_domain.as_bytes())?;

    transcript.extend_from_slice(&input.proof_contract_version.to_be_bytes());

    push_len_prefixed(&mut transcript, input.challenge_id.as_str().as_bytes())?;

    push_len_prefixed(&mut transcript, input.network_id.as_str().as_bytes())?;

    push_len_prefixed(&mut transcript, input.environment.as_str().as_bytes())?;

    push_len_prefixed(&mut transcript, input.audience.as_str().as_bytes())?;

    push_len_prefixed(&mut transcript, input.passport_id.as_str().as_bytes())?;

    push_len_prefixed(&mut transcript, input.device_id.as_str().as_bytes())?;

    push_len_prefixed(&mut transcript, &device_public_key)?;
    push_len_prefixed(&mut transcript, &challenge_transcript_hash)?;

    let scope_count =
        u16::try_from(scopes.len()).map_err(|_| DeviceSessionProofError::TooManyScopes)?;

    transcript.extend_from_slice(&scope_count.to_be_bytes());

    for scope in scopes {
        push_len_prefixed(&mut transcript, scope.as_str().as_bytes())?;
    }

    transcript.extend_from_slice(&input.challenge_issued_at_ms.to_be_bytes());
    transcript.extend_from_slice(&input.challenge_expires_at_ms.to_be_bytes());
    transcript.extend_from_slice(&input.proof_created_at_ms.to_be_bytes());

    Ok(transcript)
}

/// BLAKE3 audit digest of the canonical device-session proof transcript.
///
/// Ed25519 signs the canonical transcript bytes themselves. This digest is
/// deterministic public evidence and later envelope-binding material only.
pub fn device_session_proof_v1_transcript_b3_hex(
    input: &DeviceSessionProofTranscriptV1<'_>,
) -> Result<String, DeviceSessionProofError> {
    let transcript = canonical_device_session_proof_v1_transcript(input)?;

    Ok(blake3::hash(&transcript).to_hex().to_string())
}

/// Strictly verify device possession over the canonical V1 session transcript.
///
/// The caller must supply the device public key from trusted durable
/// authorization state rather than trusting a proof-envelope key claim.
pub fn verify_device_session_proof_v1_strict(
    input: &DeviceSessionProofTranscriptV1<'_>,
    signature: &Ed25519SignatureV1,
) -> Result<(), DeviceSessionProofError> {
    let transcript = canonical_device_session_proof_v1_transcript(input)?;

    let public_key_bytes = decode_lower_hex_32(input.device_public_key.as_str())
        .ok_or(DeviceSessionProofError::InvalidDevicePublicKey)?;

    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|_| DeviceSessionProofError::InvalidEd25519DevicePublicKey)?;

    let signature = Signature::from_bytes(signature.as_bytes());

    verifying_key
        .verify_strict(&transcript, &signature)
        .map_err(|_| DeviceSessionProofError::InvalidSignature)
}

fn validate_input(
    input: &DeviceSessionProofTranscriptV1<'_>,
) -> Result<(), DeviceSessionProofError> {
    validate_contract_text(input.challenge_contract_domain)?;
    validate_contract_text(input.proof_contract_domain)?;

    if input.challenge_contract_version == 0 || input.proof_contract_version == 0 {
        return Err(DeviceSessionProofError::InvalidContractText);
    }

    if input.challenge_issued_at_ms == 0
        || input.challenge_expires_at_ms <= input.challenge_issued_at_ms
    {
        return Err(DeviceSessionProofError::InvalidChallengeTimeWindow);
    }

    if input.proof_created_at_ms < input.challenge_issued_at_ms
        || input.proof_created_at_ms > input.challenge_expires_at_ms
    {
        return Err(DeviceSessionProofError::InvalidProofCreatedAt);
    }

    let _ = canonical_scopes(input.requested_scopes)?;

    if decode_lower_hex_32(input.device_public_key.as_str()).is_none() {
        return Err(DeviceSessionProofError::InvalidDevicePublicKey);
    }

    if decode_lower_hex_32(input.challenge_transcript_hash.as_str()).is_none() {
        return Err(DeviceSessionProofError::InvalidChallengeTranscriptHash);
    }

    Ok(())
}

fn validate_contract_text(value: &str) -> Result<(), DeviceSessionProofError> {
    let bytes = value.as_bytes();

    if bytes.is_empty()
        || bytes.len() > MAX_TRANSCRIPT_TEXT_BYTES
        || !bytes.iter().all(|byte| byte.is_ascii_graphic())
    {
        return Err(DeviceSessionProofError::InvalidContractText);
    }

    Ok(())
}

fn canonical_scopes(
    requested_scopes: &[NativePassportScopeV1],
) -> Result<Vec<&NativePassportScopeV1>, DeviceSessionProofError> {
    if requested_scopes.is_empty() {
        return Err(DeviceSessionProofError::MissingScopes);
    }

    if requested_scopes.len() > DEVICE_AUTHORIZATION_V1_MAX_SCOPES {
        return Err(DeviceSessionProofError::TooManyScopes);
    }

    let mut scopes = requested_scopes.iter().collect::<Vec<_>>();

    scopes.sort_unstable_by(|left, right| left.as_str().cmp(right.as_str()));

    if scopes
        .windows(2)
        .any(|pair| pair[0].as_str() == pair[1].as_str())
    {
        return Err(DeviceSessionProofError::DuplicateScope);
    }

    Ok(scopes)
}

fn push_len_prefixed(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), DeviceSessionProofError> {
    let length =
        u16::try_from(bytes.len()).map_err(|_| DeviceSessionProofError::TranscriptFieldTooLong)?;

    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);

    Ok(())
}

fn decode_lower_hex_32(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }

    let mut decoded = [0_u8; 32];
    let bytes = value.as_bytes();

    for (index, destination) in decoded.iter_mut().enumerate() {
        let high = lower_hex_nibble(bytes[index * 2])?;
        let low = lower_hex_nibble(bytes[index * 2 + 1])?;

        *destination = (high << 4) | low;
    }

    Some(decoded)
}

fn lower_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
