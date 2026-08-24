//! RO:WHAT — Canonical Native Passport V1 service-challenge transcript construction and strict Ed25519 verification.
//! RO:WHY — Root/device signing must occur only after the client verifies an authentic, purpose-bound, unexpired svc-passport challenge using deterministic bytes owned by ron-auth.
//! RO:INTERACTS — ron-proto PassportChallengeV1/signing payload, typed service public key/context labels, B3 audit hashing, future svc-passport challenge signer/issuer, and native client pre-sign validation.
//! RO:INVARIANTS — deterministic length-prefixed binary V1 transcript only; ordinary JSON is never signable; every public challenge field is signature-bound; verifier trust inputs come from local/service configuration, not challenge authority claims.
//! RO:METRICS — none.
//! RO:CONFIG — trusted verifier clock skew is caller-owned and cannot exceed the V1 30-second protocol ceiling.
//! RO:SECURITY — public-data transcript construction and public-key verification only; no signing key, key loading, persistence, replay mutation, routes, capability issuance, wallet, or ledger authority.
//! RO:TEST — tests/native_passport_challenge_v1.rs.

#![forbid(unsafe_code)]

use ed25519_dalek::{Signature, VerifyingKey};
use ron_proto::{
    Ed25519PublicKeyHex, NativePassportContextLabelV1, PassportChallengeSigningPayloadV1,
    PassportChallengeV1, PassportChallengeValidationError, ServiceKeyIdV1,
    PASSPORT_CHALLENGE_V1_MAX_CLOCK_SKEW_MS,
};
use thiserror::Error;

/// Frozen domain for the real Native Passport V1 service challenge.
pub const PASSPORT_CHALLENGE_V1_TRANSCRIPT_DOMAIN: &str =
    "rustyonions.native-passport.service-challenge.v1";

/// Canonical V1 service-challenge transcript encoding.
pub const PASSPORT_CHALLENGE_V1_CANONICAL_TRANSCRIPT_ENCODING: &str = "length-prefixed-binary-v1";

/// Ordinary serialized JSON is never a service-challenge signing input.
pub const PASSPORT_CHALLENGE_V1_JSON_TRANSCRIPT_SIGNABLE: bool = false;

/// Trusted local context for strict service challenge verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassportChallengeVerificationContextV1<'a> {
    /// Trusted service challenge public key selected by local configuration.
    pub trusted_service_public_key: &'a Ed25519PublicKeyHex,

    /// Expected network.
    pub expected_network_id: &'a NativePassportContextLabelV1,

    /// Expected environment.
    pub expected_environment: &'a NativePassportContextLabelV1,

    /// Expected audience.
    pub expected_audience: &'a NativePassportContextLabelV1,

    /// Expected issuing service identity.
    pub expected_issuing_service_id: &'a NativePassportContextLabelV1,

    /// Expected trusted service-key identifier.
    pub expected_service_key_id: &'a ServiceKeyIdV1,

    /// Trusted current time.
    pub now_ms: u64,

    /// Locally accepted clock skew. This is verifier policy, not wire authority.
    pub max_clock_skew_ms: u64,
}

/// Canonical transcript construction failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PassportChallengeTranscriptError {
    /// Protocol challenge structure was invalid.
    #[error("invalid PassportChallengeV1: {0}")]
    InvalidChallenge(#[from] PassportChallengeValidationError),

    /// A length-prefixed field exceeded u16.
    #[error("PassportChallengeV1 transcript field exceeds u16")]
    FieldTooLong,

    /// A typed 32-byte lowercase-hex field unexpectedly failed decoding.
    #[error("PassportChallengeV1 digest field {0} failed decoding")]
    InvalidDigest(&'static str),
}

/// Strict service challenge verification failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PassportChallengeVerificationError {
    /// Challenge structure failed protocol validation.
    #[error("invalid PassportChallengeV1")]
    InvalidChallenge,

    /// Trusted service public key could not be decoded or is not valid Ed25519.
    #[error("trusted Passport challenge service public key is invalid")]
    InvalidTrustedServicePublicKey,

    /// Network did not match trusted local context.
    #[error("PassportChallengeV1 network mismatch")]
    NetworkMismatch,

    /// Environment did not match trusted local context.
    #[error("PassportChallengeV1 environment mismatch")]
    EnvironmentMismatch,

    /// Audience did not match trusted local context.
    #[error("PassportChallengeV1 audience mismatch")]
    AudienceMismatch,

    /// Issuing service identity did not match trusted local context.
    #[error("PassportChallengeV1 issuing service mismatch")]
    IssuingServiceMismatch,

    /// Service key identifier did not match trusted local context.
    #[error("PassportChallengeV1 service key identifier mismatch")]
    ServiceKeyIdMismatch,

    /// Caller attempted to configure more clock skew than V1 permits.
    #[error("PassportChallengeV1 verifier clock-skew policy exceeds protocol maximum")]
    ClockSkewPolicyExceeded,

    /// Challenge issue time is beyond accepted future skew.
    #[error("PassportChallengeV1 is not yet valid")]
    NotYetValid,

    /// Challenge expiry is exceeded beyond accepted skew.
    #[error("PassportChallengeV1 has expired")]
    Expired,

    /// Canonical transcript construction failed.
    #[error("PassportChallengeV1 transcript construction failed")]
    Transcript,

    /// Ed25519 service signature did not verify.
    #[error("PassportChallengeV1 service signature mismatch")]
    InvalidServiceSignature,
}

/// Build the exact bytes covered by the svc-passport challenge signature.
///
/// V1 layout:
///
/// ```text
/// lp(domain)
/// u16_be(version)
/// lp(network_id)
/// lp(environment)
/// lp(audience)
/// lp(issuing_service_id)
/// lp(service_key_id)
/// lp(purpose)
/// u8(passport_id_present)
/// [lp(passport_id)]
/// u8(device_id_present)
/// [lp(device_id)]
/// lp(challenge_id)
/// lp(raw_32_byte_nonce)
/// u16_be(scope_count)
/// repeated lp(scope)
/// u8(operation_body_hash_present)
/// [lp(raw_32_byte_operation_body_hash)]
/// u64_be(issued_at_ms)
/// u64_be(expires_at_ms)
/// ```
///
/// `lp(x)` means `u16_be(len(x)) || x`.
pub fn canonical_passport_challenge_v1_transcript(
    payload: &PassportChallengeSigningPayloadV1,
) -> Result<Vec<u8>, PassportChallengeTranscriptError> {
    payload.validate()?;

    let nonce = decode_lower_hex_32("nonce", payload.nonce.as_str())?;

    let operation_body_hash = payload
        .operation_body_hash
        .as_ref()
        .map(|value| decode_lower_hex_32("operation_body_hash", value.as_str()))
        .transpose()?;

    let scope_count = u16::try_from(payload.requested_scopes.len())
        .map_err(|_| PassportChallengeTranscriptError::FieldTooLong)?;

    let mut transcript = Vec::with_capacity(768);

    push_len_prefixed(
        &mut transcript,
        PASSPORT_CHALLENGE_V1_TRANSCRIPT_DOMAIN.as_bytes(),
    )?;

    transcript.extend_from_slice(&payload.version.to_be_bytes());

    push_len_prefixed(&mut transcript, payload.network_id.as_str().as_bytes())?;
    push_len_prefixed(&mut transcript, payload.environment.as_str().as_bytes())?;
    push_len_prefixed(&mut transcript, payload.audience.as_str().as_bytes())?;
    push_len_prefixed(
        &mut transcript,
        payload.issuing_service_id.as_str().as_bytes(),
    )?;
    push_len_prefixed(&mut transcript, payload.service_key_id.as_str().as_bytes())?;
    push_len_prefixed(&mut transcript, payload.purpose.as_str().as_bytes())?;

    match payload.passport_id.as_ref() {
        None => transcript.push(0),
        Some(passport_id) => {
            transcript.push(1);
            push_len_prefixed(&mut transcript, passport_id.as_str().as_bytes())?;
        }
    }

    match payload.device_id.as_ref() {
        None => transcript.push(0),
        Some(device_id) => {
            transcript.push(1);
            push_len_prefixed(&mut transcript, device_id.as_str().as_bytes())?;
        }
    }

    push_len_prefixed(&mut transcript, payload.challenge_id.as_str().as_bytes())?;
    push_len_prefixed(&mut transcript, &nonce)?;

    transcript.extend_from_slice(&scope_count.to_be_bytes());

    for scope in &payload.requested_scopes {
        push_len_prefixed(&mut transcript, scope.as_str().as_bytes())?;
    }

    match operation_body_hash {
        None => transcript.push(0),
        Some(body_hash) => {
            transcript.push(1);
            push_len_prefixed(&mut transcript, &body_hash)?;
        }
    }

    transcript.extend_from_slice(&payload.issued_at_ms.to_be_bytes());
    transcript.extend_from_slice(&payload.expires_at_ms.to_be_bytes());

    Ok(transcript)
}

/// Return the BLAKE3 audit digest of the canonical challenge transcript.
///
/// Ed25519 signs the transcript bytes themselves. This digest is evidence for
/// proof binding and replay records, not a substitute signing input.
pub fn passport_challenge_v1_transcript_b3_hex(
    payload: &PassportChallengeSigningPayloadV1,
) -> Result<String, PassportChallengeTranscriptError> {
    let transcript = canonical_passport_challenge_v1_transcript(payload)?;

    Ok(blake3::hash(&transcript).to_hex().to_string())
}

/// Strictly verify an authentic, context-bound, timely V1 service challenge.
///
/// Success proves challenge structure, trusted context, time validity, and
/// service signature only. Purpose-specific client intent comparison,
/// replay-state mutation, proof acceptance, and capability issuance remain
/// separate authority decisions.
pub fn verify_passport_challenge_v1_strict(
    challenge: &PassportChallengeV1,
    context: PassportChallengeVerificationContextV1<'_>,
) -> Result<(), PassportChallengeVerificationError> {
    challenge
        .validate()
        .map_err(|_| PassportChallengeVerificationError::InvalidChallenge)?;

    if context.max_clock_skew_ms > PASSPORT_CHALLENGE_V1_MAX_CLOCK_SKEW_MS {
        return Err(PassportChallengeVerificationError::ClockSkewPolicyExceeded);
    }

    if &challenge.network_id != context.expected_network_id {
        return Err(PassportChallengeVerificationError::NetworkMismatch);
    }

    if &challenge.environment != context.expected_environment {
        return Err(PassportChallengeVerificationError::EnvironmentMismatch);
    }

    if &challenge.audience != context.expected_audience {
        return Err(PassportChallengeVerificationError::AudienceMismatch);
    }

    if &challenge.issuing_service_id != context.expected_issuing_service_id {
        return Err(PassportChallengeVerificationError::IssuingServiceMismatch);
    }

    if &challenge.service_key_id != context.expected_service_key_id {
        return Err(PassportChallengeVerificationError::ServiceKeyIdMismatch);
    }

    let latest_acceptable_issue = context.now_ms.saturating_add(context.max_clock_skew_ms);

    if challenge.issued_at_ms > latest_acceptable_issue {
        return Err(PassportChallengeVerificationError::NotYetValid);
    }

    let latest_acceptable_expiry = challenge
        .expires_at_ms
        .saturating_add(context.max_clock_skew_ms);

    if context.now_ms > latest_acceptable_expiry {
        return Err(PassportChallengeVerificationError::Expired);
    }

    let trusted_service_key =
        decode_trusted_service_public_key(context.trusted_service_public_key)?;

    let verifying_key = VerifyingKey::from_bytes(&trusted_service_key)
        .map_err(|_| PassportChallengeVerificationError::InvalidTrustedServicePublicKey)?;

    let transcript = canonical_passport_challenge_v1_transcript(&challenge.signing_payload())
        .map_err(|_| PassportChallengeVerificationError::Transcript)?;

    let signature = Signature::from_bytes(challenge.service_signature.as_bytes());

    verifying_key
        .verify_strict(&transcript, &signature)
        .map_err(|_| PassportChallengeVerificationError::InvalidServiceSignature)
}

fn push_len_prefixed(
    output: &mut Vec<u8>,
    bytes: &[u8],
) -> Result<(), PassportChallengeTranscriptError> {
    let length =
        u16::try_from(bytes.len()).map_err(|_| PassportChallengeTranscriptError::FieldTooLong)?;

    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);

    Ok(())
}

fn decode_lower_hex_32(
    field: &'static str,
    value: &str,
) -> Result<[u8; 32], PassportChallengeTranscriptError> {
    decode_lower_hex_32_inner(value).ok_or(PassportChallengeTranscriptError::InvalidDigest(field))
}

fn decode_trusted_service_public_key(
    value: &Ed25519PublicKeyHex,
) -> Result<[u8; 32], PassportChallengeVerificationError> {
    decode_lower_hex_32_inner(value.as_str())
        .ok_or(PassportChallengeVerificationError::InvalidTrustedServicePublicKey)
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
