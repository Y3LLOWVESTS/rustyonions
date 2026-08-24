//! RO:WHAT — Exact canonical signing transcript construction for Native Passport DeviceAuthorizationV1.
//! RO:WHY — A real Passport root signature must bind deterministic bytes owned by ron-auth rather than JSON, delimiter text, or client-specific serialization.
//! RO:INTERACTS — ron-proto DeviceAuthorizationSigningPayloadV1, canonical DeviceClassV1/scope tokens, future Passport-ID/root-key and Device-ID/device-key verification, and svc-passport root signing.
//! RO:INVARIANTS — domain separated; variable fields use u16 big-endian length prefixes; integers are fixed big-endian; device public key and nonce enter as raw bytes; scopes are already strictly sorted by ron-proto; optional expiry has an explicit presence byte.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — pure deterministic public-data transform only; no key custody, signing, I/O, HTTP, persistence, capability issuance, wallet mutation, or ledger mutation.
//! RO:TEST — tests/physical_m1_device_authorization_v1_transcript.rs.

use ron_proto::{DeviceAuthorizationSigningPayloadV1, DeviceAuthorizationValidationError};
use thiserror::Error;

use super::DEVICE_AUTHORIZATION_V1_TRANSCRIPT_DOMAIN;

/// Canonical DeviceAuthorizationV1 transcript construction error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DeviceAuthorizationTranscriptError {
    /// Wire/domain payload failed strict validation.
    #[error("invalid DeviceAuthorizationV1 payload: {0}")]
    InvalidPayload(#[from] DeviceAuthorizationValidationError),

    /// A length-prefixed field exceeded u16.
    #[error("DeviceAuthorizationV1 transcript field exceeds u16")]
    FieldTooLong,

    /// Typed public-key text unexpectedly failed byte decoding.
    #[error("DeviceAuthorizationV1 device public key encoding failed")]
    InvalidDevicePublicKey,
}

/// Build the exact bytes covered by the Passport-root signature.
///
/// V1 layout:
///
/// ```text
/// lp(domain)
/// u16_be(version)
/// lp(network_id)
/// lp(environment)
/// lp(passport_id)
/// u64_be(root_key_epoch)
/// lp(device_id)
/// lp(raw_32_byte_device_public_key)
/// lp(device_class)
/// u16_be(scope_count)
/// repeated lp(scope)
/// lp(raw_16_byte_authorization_nonce)
/// u64_be(issued_at_ms)
/// u8(expires_present)
/// [u64_be(expires_at_ms)]
/// ```
///
/// `lp(x)` means `u16_be(len(x)) || x`.
pub fn canonical_device_authorization_v1_transcript(
    payload: &DeviceAuthorizationSigningPayloadV1,
) -> Result<Vec<u8>, DeviceAuthorizationTranscriptError> {
    payload.validate()?;

    let device_public_key = decode_lower_hex_32(payload.device_public_key.as_str())?;

    let scope_count = u16::try_from(payload.authorized_scope_ceiling.as_slice().len())
        .map_err(|_| DeviceAuthorizationTranscriptError::FieldTooLong)?;

    let mut transcript = Vec::with_capacity(512);

    push_len_prefixed(
        &mut transcript,
        DEVICE_AUTHORIZATION_V1_TRANSCRIPT_DOMAIN.as_bytes(),
    )?;

    transcript.extend_from_slice(&payload.version.to_be_bytes());

    push_len_prefixed(&mut transcript, payload.network_id.as_str().as_bytes())?;

    push_len_prefixed(&mut transcript, payload.environment.as_str().as_bytes())?;

    push_len_prefixed(&mut transcript, payload.passport_id.as_str().as_bytes())?;

    transcript.extend_from_slice(&payload.root_key_epoch.to_be_bytes());

    push_len_prefixed(&mut transcript, payload.device_id.as_str().as_bytes())?;

    push_len_prefixed(&mut transcript, &device_public_key)?;

    push_len_prefixed(&mut transcript, payload.device_class.as_str().as_bytes())?;

    transcript.extend_from_slice(&scope_count.to_be_bytes());

    for scope in payload.authorized_scope_ceiling.as_slice() {
        push_len_prefixed(&mut transcript, scope.as_str().as_bytes())?;
    }

    push_len_prefixed(&mut transcript, payload.authorization_nonce.as_bytes())?;

    transcript.extend_from_slice(&payload.issued_at_ms.to_be_bytes());

    match payload.expires_at_ms {
        None => transcript.push(0),
        Some(expires_at_ms) => {
            transcript.push(1);
            transcript.extend_from_slice(&expires_at_ms.to_be_bytes());
        }
    }

    Ok(transcript)
}

/// BLAKE3 audit digest of the canonical signing transcript.
///
/// Ed25519 verification will cover the transcript bytes themselves, not this
/// digest. The digest is useful only for deterministic evidence/tests.
pub fn device_authorization_v1_transcript_b3_hex(
    payload: &DeviceAuthorizationSigningPayloadV1,
) -> Result<String, DeviceAuthorizationTranscriptError> {
    let transcript = canonical_device_authorization_v1_transcript(payload)?;

    Ok(blake3::hash(&transcript).to_hex().to_string())
}

fn push_len_prefixed(
    output: &mut Vec<u8>,
    bytes: &[u8],
) -> Result<(), DeviceAuthorizationTranscriptError> {
    let length =
        u16::try_from(bytes.len()).map_err(|_| DeviceAuthorizationTranscriptError::FieldTooLong)?;

    output.extend_from_slice(&length.to_be_bytes());

    output.extend_from_slice(bytes);

    Ok(())
}

fn decode_lower_hex_32(value: &str) -> Result<[u8; 32], DeviceAuthorizationTranscriptError> {
    if value.len() != 64 {
        return Err(DeviceAuthorizationTranscriptError::InvalidDevicePublicKey);
    }

    let source = value.as_bytes();
    let mut output = [0_u8; 32];

    for index in 0..32 {
        let high = lower_hex_nibble(source[index * 2])
            .ok_or(DeviceAuthorizationTranscriptError::InvalidDevicePublicKey)?;

        let low = lower_hex_nibble(source[index * 2 + 1])
            .ok_or(DeviceAuthorizationTranscriptError::InvalidDevicePublicKey)?;

        output[index] = (high << 4) | low;
    }

    Ok(output)
}

fn lower_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
