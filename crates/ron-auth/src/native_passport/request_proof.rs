//! RO:WHAT — Canonical Native Passport V1 protected-request transcript construction and strict DeviceKey verification.
//! RO:WHY — A device-bound capability must remain useless for sensitive operations unless the exact request is approved by the capability-bound DeviceKey.
//! RO:INTERACTS — ron-proto PassportRequestProofV1/CapabilityIdV1/DeviceIdV1/B3/Ed25519 DTOs, svc-passport durable capability/device state, future request-nonce replay storage, and CrabLink native DeviceKey signing.
//! RO:INVARIANTS — deterministic length-prefixed binary V1 only; capability ID, Device ID, method, path, query hash, body hash, timestamp, and nonce are signature-bound; trusted device public key and expected request values come from server authority, not the proof.
//! RO:METRICS — none; pure verifier.
//! RO:CONFIG — maximum accepted verifier clock skew is 30 seconds, preserving the established Native Passport request-proof contract ceiling.
//! RO:SECURITY — public-data transcript and Ed25519 verification only; no private key, signing, persistence, replay mutation, capability issuance/revocation, namespace mutation, wallet, or ledger authority.
//! RO:TEST — tests/native_passport_request_proof.rs.

#![forbid(unsafe_code)]

use ed25519_dalek::{Signature, VerifyingKey};
use ron_proto::{
    B3DigestHex, CapabilityIdV1, DeviceIdV1, Ed25519PublicKeyHex, PassportRequestProofV1,
};
use thiserror::Error;

/// Frozen Native Passport V1 protected-request signature domain.
pub const PASSPORT_REQUEST_PROOF_V1_TRANSCRIPT_DOMAIN: &str =
    "rustyonions.native-passport.request-proof.v1";

/// Canonical encoding for protected-request DeviceKey signatures.
pub const PASSPORT_REQUEST_PROOF_V1_CANONICAL_TRANSCRIPT_ENCODING: &str =
    "length-prefixed-binary-v1";

/// Ordinary JSON serialization is never a request-proof signing input.
pub const PASSPORT_REQUEST_PROOF_V1_JSON_TRANSCRIPT_SIGNABLE: bool = false;

/// Maximum verifier clock skew accepted by Native Passport V1 request proofs.
pub const PASSPORT_REQUEST_PROOF_V1_MAX_CLOCK_SKEW_MS: u64 = 30_000;

/// Trusted server-side context for one protected request.
///
/// Capability/device/request expectations must be obtained from authoritative
/// runtime state and the actual request being admitted. None are taken from
/// the proof as authority.
#[derive(Debug, Clone, Copy)]
pub struct PassportRequestProofVerificationContextV1<'a> {
    pub trusted_device_public_key: &'a Ed25519PublicKeyHex,
    pub expected_capability_id: &'a CapabilityIdV1,
    pub expected_device_id: &'a DeviceIdV1,
    pub expected_request_method: &'a str,
    pub expected_canonical_path: &'a str,
    pub expected_canonical_query_hash: &'a B3DigestHex,
    pub expected_body_hash: &'a B3DigestHex,
    pub now_ms: u64,
    pub max_clock_skew_ms: u64,
}

/// Canonical transcript construction failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PassportRequestProofTranscriptError {
    #[error("invalid PassportRequestProofV1")]
    InvalidProof,

    #[error("PassportRequestProofV1 transcript field exceeds u16")]
    FieldTooLong,

    #[error("PassportRequestProofV1 digest field {0} is invalid")]
    InvalidDigest(&'static str),
}

/// Strict protected-request verification failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PassportRequestProofVerificationError {
    #[error("invalid PassportRequestProofV1")]
    InvalidProof,

    #[error("trusted request-proof device public key is invalid")]
    InvalidTrustedDevicePublicKey,

    #[error("PassportRequestProofV1 capability ID mismatch")]
    CapabilityIdMismatch,

    #[error("PassportRequestProofV1 Device ID mismatch")]
    DeviceIdMismatch,

    #[error("PassportRequestProofV1 request method mismatch")]
    RequestMethodMismatch,

    #[error("PassportRequestProofV1 canonical path mismatch")]
    CanonicalPathMismatch,

    #[error("PassportRequestProofV1 canonical query hash mismatch")]
    CanonicalQueryHashMismatch,

    #[error("PassportRequestProofV1 body hash mismatch")]
    BodyHashMismatch,

    #[error("PassportRequestProofV1 verifier clock-skew policy exceeds protocol maximum")]
    ClockSkewPolicyExceeded,

    #[error("PassportRequestProofV1 timestamp is in the future")]
    NotYetValid,

    #[error("PassportRequestProofV1 timestamp is stale")]
    Stale,

    #[error("PassportRequestProofV1 transcript construction failed")]
    Transcript,

    #[error("PassportRequestProofV1 DeviceKey signature mismatch")]
    InvalidDeviceSignature,
}

/// Construct the exact bytes signed by the bound DeviceKey.
///
/// V1 layout:
///
/// ```text
/// lp(domain)
/// u16_be(version)
/// lp(capability_id)
/// lp(request_method)
/// lp(canonical_path)
/// lp(raw_32_byte_canonical_query_hash)
/// lp(raw_32_byte_body_hash)
/// u64_be(timestamp_ms)
/// lp(raw_32_byte_request_nonce)
/// lp(device_id)
/// ```
///
/// `lp(x)` means `u16_be(len(x)) || x`.
pub fn canonical_passport_request_proof_v1_transcript(
    proof: &PassportRequestProofV1,
) -> Result<Vec<u8>, PassportRequestProofTranscriptError> {
    proof
        .validate()
        .map_err(|_| PassportRequestProofTranscriptError::InvalidProof)?;

    let query_hash =
        decode_lower_hex_32("canonical_query_hash", proof.canonical_query_hash.as_str())?;

    let body_hash = decode_lower_hex_32("body_hash", proof.body_hash.as_str())?;

    let request_nonce = decode_lower_hex_32("request_nonce", proof.request_nonce.as_str())?;

    let mut transcript = Vec::with_capacity(768);

    push_len_prefixed(
        &mut transcript,
        PASSPORT_REQUEST_PROOF_V1_TRANSCRIPT_DOMAIN.as_bytes(),
    )?;

    transcript.extend_from_slice(&proof.version.to_be_bytes());

    push_len_prefixed(&mut transcript, proof.capability_id.as_str().as_bytes())?;

    push_len_prefixed(&mut transcript, proof.request_method.as_bytes())?;

    push_len_prefixed(&mut transcript, proof.canonical_path.as_bytes())?;

    push_len_prefixed(&mut transcript, &query_hash)?;
    push_len_prefixed(&mut transcript, &body_hash)?;

    transcript.extend_from_slice(&proof.timestamp_ms.to_be_bytes());

    push_len_prefixed(&mut transcript, &request_nonce)?;

    push_len_prefixed(&mut transcript, proof.device_id.as_str().as_bytes())?;

    Ok(transcript)
}

/// Return a deterministic BLAKE3 audit digest of the canonical request proof.
///
/// The DeviceKey signs the canonical bytes themselves. This digest is only
/// public audit/interop evidence and is never a substitute signing input.
pub fn passport_request_proof_v1_transcript_b3_hex(
    proof: &PassportRequestProofV1,
) -> Result<String, PassportRequestProofTranscriptError> {
    let transcript = canonical_passport_request_proof_v1_transcript(proof)?;

    Ok(blake3::hash(&transcript).to_hex().to_string())
}

/// Strictly verify one protected request against trusted server context.
///
/// Successful verification proves exact request binding, freshness within the
/// configured V1 skew ceiling, and control of the trusted DeviceKey. Replay
/// consumption, capability lookup/revocation, scope checks, audience and
/// environment checks remain svc-passport runtime responsibilities.
pub fn verify_passport_request_proof_v1_strict(
    proof: &PassportRequestProofV1,
    context: PassportRequestProofVerificationContextV1<'_>,
) -> Result<(), PassportRequestProofVerificationError> {
    proof
        .validate()
        .map_err(|_| PassportRequestProofVerificationError::InvalidProof)?;

    if context.max_clock_skew_ms > PASSPORT_REQUEST_PROOF_V1_MAX_CLOCK_SKEW_MS {
        return Err(PassportRequestProofVerificationError::ClockSkewPolicyExceeded);
    }

    if &proof.capability_id != context.expected_capability_id {
        return Err(PassportRequestProofVerificationError::CapabilityIdMismatch);
    }

    if &proof.device_id != context.expected_device_id {
        return Err(PassportRequestProofVerificationError::DeviceIdMismatch);
    }

    if proof.request_method != context.expected_request_method {
        return Err(PassportRequestProofVerificationError::RequestMethodMismatch);
    }

    if proof.canonical_path != context.expected_canonical_path {
        return Err(PassportRequestProofVerificationError::CanonicalPathMismatch);
    }

    if &proof.canonical_query_hash != context.expected_canonical_query_hash {
        return Err(PassportRequestProofVerificationError::CanonicalQueryHashMismatch);
    }

    if &proof.body_hash != context.expected_body_hash {
        return Err(PassportRequestProofVerificationError::BodyHashMismatch);
    }

    let latest_acceptable_timestamp = context.now_ms.saturating_add(context.max_clock_skew_ms);

    if proof.timestamp_ms > latest_acceptable_timestamp {
        return Err(PassportRequestProofVerificationError::NotYetValid);
    }

    let latest_acceptable_age = proof.timestamp_ms.saturating_add(context.max_clock_skew_ms);

    if context.now_ms > latest_acceptable_age {
        return Err(PassportRequestProofVerificationError::Stale);
    }

    let device_public_key =
        decode_lower_hex_32_inner(context.trusted_device_public_key.as_str())
            .ok_or(PassportRequestProofVerificationError::InvalidTrustedDevicePublicKey)?;

    let verifying_key = VerifyingKey::from_bytes(&device_public_key)
        .map_err(|_| PassportRequestProofVerificationError::InvalidTrustedDevicePublicKey)?;

    let transcript = canonical_passport_request_proof_v1_transcript(proof)
        .map_err(|_| PassportRequestProofVerificationError::Transcript)?;

    let signature = Signature::from_bytes(proof.device_signature.as_bytes());

    verifying_key
        .verify_strict(&transcript, &signature)
        .map_err(|_| PassportRequestProofVerificationError::InvalidDeviceSignature)
}

fn push_len_prefixed(
    output: &mut Vec<u8>,
    bytes: &[u8],
) -> Result<(), PassportRequestProofTranscriptError> {
    let length = u16::try_from(bytes.len())
        .map_err(|_| PassportRequestProofTranscriptError::FieldTooLong)?;

    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);

    Ok(())
}

fn decode_lower_hex_32(
    field: &'static str,
    value: &str,
) -> Result<[u8; 32], PassportRequestProofTranscriptError> {
    decode_lower_hex_32_inner(value)
        .ok_or(PassportRequestProofTranscriptError::InvalidDigest(field))
}

fn decode_lower_hex_32_inner(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }

    let source = value.as_bytes();
    let mut output = [0_u8; 32];

    for index in 0..32 {
        let high = lower_hex_nibble(source[index * 2])?;
        let low = lower_hex_nibble(source[index * 2 + 1])?;

        output[index] = (high << 4) | low;
    }

    Some(output)
}

fn lower_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
