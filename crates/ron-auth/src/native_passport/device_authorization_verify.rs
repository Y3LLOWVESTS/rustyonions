//! RO:WHAT — Strict pure verification for root-signed Native Passport DeviceAuthorizationV1.
//! RO:WHY — Physical M1 must prove that an authorization came from the trusted Passport root and binds the expected device/context before the physical Mac gains network authority.
//! RO:INTERACTS — ron-proto DeviceAuthorizationV1, PassportIdV1, DeviceIdV1, Ed25519 public-key DTOs and frozen ID domains; sibling canonical authorization transcript builder; ed25519-dalek strict verification; future svc-passport authorization registry.
//! RO:INVARIANTS — trusted root key is external to the untrusted authorization; Passport ID derives from that trusted root; Device ID derives from the signed device key; network/environment/root epoch/time match caller expectations; Ed25519 verifies the exact canonical transcript bytes.
//! RO:METRICS — none; pure verifier.
//! RO:CONFIG — caller supplies trusted Passport/context state and bounded clock skew.
//! RO:SECURITY — no private key, signing, key generation, I/O, persistence, challenge consumption, capability issuance, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — tests/physical_m1_device_authorization_v1_verify.rs.

use ed25519_dalek::{Signature, VerifyingKey};
use ron_proto::{
    DeviceAuthorizationV1, DeviceIdV1, Ed25519PublicKeyHex, NativePassportContextLabelV1,
    PassportIdV1, DEVICE_ID_V1_ED25519_B3_PREFIX, DEVICE_ID_V1_HASH_DOMAIN,
    PASSPORT_ID_V1_HASH_DOMAIN, PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX,
};
use thiserror::Error;

use super::canonical_device_authorization_v1_transcript;

/// Trusted state required to verify one DeviceAuthorizationV1.
///
/// These values come from authoritative Passport state, not from the
/// attacker-controlled authorization record.
#[derive(Debug, Clone, Copy)]
pub struct DeviceAuthorizationVerificationContextV1<'a> {
    /// Expected trusted Passport subject.
    pub trusted_passport_id: &'a PassportIdV1,

    /// Trusted Passport root Ed25519 public key.
    pub trusted_root_public_key: &'a Ed25519PublicKeyHex,

    /// Expected trusted root-key epoch.
    pub trusted_root_key_epoch: u64,

    /// Expected network.
    pub expected_network_id: &'a NativePassportContextLabelV1,

    /// Expected deployment environment.
    pub expected_environment: &'a NativePassportContextLabelV1,

    /// Current Unix time in milliseconds.
    pub now_ms: u64,

    /// Maximum accepted clock skew in milliseconds.
    pub max_clock_skew_ms: u64,
}

/// Strict DeviceAuthorizationV1 verification failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DeviceAuthorizationVerificationError {
    /// DTO/domain validation failed.
    #[error("invalid DeviceAuthorizationV1")]
    InvalidAuthorization,

    /// Trusted root key could not form a valid Ed25519 verifier.
    #[error("invalid trusted Passport root public key")]
    InvalidTrustedRootPublicKey,

    /// Trusted Passport ID does not bind to the trusted root key.
    #[error("trusted Passport ID/root public key binding mismatch")]
    TrustedPassportRootBindingMismatch,

    /// Authorization targets another Passport.
    #[error("DeviceAuthorizationV1 Passport ID mismatch")]
    PassportIdMismatch,

    /// Device ID does not bind to its signed device public key.
    #[error("DeviceAuthorizationV1 Device ID/public key binding mismatch")]
    DeviceIdBindingMismatch,

    /// Network mismatch.
    #[error("DeviceAuthorizationV1 network mismatch")]
    NetworkMismatch,

    /// Environment mismatch.
    #[error("DeviceAuthorizationV1 environment mismatch")]
    EnvironmentMismatch,

    /// Trusted root epoch mismatch.
    #[error("DeviceAuthorizationV1 root-key epoch mismatch")]
    RootKeyEpochMismatch,

    /// Issue time is beyond allowed future skew.
    #[error("DeviceAuthorizationV1 is not yet valid")]
    NotYetValid,

    /// Authorization expiry is exceeded beyond allowed skew.
    #[error("DeviceAuthorizationV1 has expired")]
    Expired,

    /// Canonical transcript construction failed.
    #[error("DeviceAuthorizationV1 transcript construction failed")]
    Transcript,

    /// Strict Ed25519 verification failed.
    #[error("DeviceAuthorizationV1 root signature mismatch")]
    InvalidRootSignature,
}

/// Strictly verify a root-signed DeviceAuthorizationV1.
///
/// Success proves cryptographic/context validity only. Device-class policy,
/// scope authorization, replay persistence, capability issuance, and server
/// registry mutation remain separate authority decisions.
pub fn verify_device_authorization_v1_strict(
    authorization: &DeviceAuthorizationV1,
    context: DeviceAuthorizationVerificationContextV1<'_>,
) -> Result<(), DeviceAuthorizationVerificationError> {
    authorization
        .validate()
        .map_err(|_| DeviceAuthorizationVerificationError::InvalidAuthorization)?;

    let trusted_root_bytes = decode_lower_hex_32(context.trusted_root_public_key.as_str())
        .ok_or(DeviceAuthorizationVerificationError::InvalidTrustedRootPublicKey)?;

    let trusted_root = VerifyingKey::from_bytes(&trusted_root_bytes)
        .map_err(|_| DeviceAuthorizationVerificationError::InvalidTrustedRootPublicKey)?;

    let derived_passport_id = derive_frozen_passport_id_v1(context.trusted_root_public_key)?;

    if &derived_passport_id != context.trusted_passport_id {
        return Err(DeviceAuthorizationVerificationError::TrustedPassportRootBindingMismatch);
    }

    if &authorization.passport_id != context.trusted_passport_id {
        return Err(DeviceAuthorizationVerificationError::PassportIdMismatch);
    }

    let derived_device_id = derive_frozen_device_id_v1(&authorization.device_public_key)?;

    if derived_device_id != authorization.device_id {
        return Err(DeviceAuthorizationVerificationError::DeviceIdBindingMismatch);
    }

    if &authorization.network_id != context.expected_network_id {
        return Err(DeviceAuthorizationVerificationError::NetworkMismatch);
    }

    if &authorization.environment != context.expected_environment {
        return Err(DeviceAuthorizationVerificationError::EnvironmentMismatch);
    }

    if authorization.root_key_epoch != context.trusted_root_key_epoch {
        return Err(DeviceAuthorizationVerificationError::RootKeyEpochMismatch);
    }

    let latest_acceptable_issue = context.now_ms.saturating_add(context.max_clock_skew_ms);

    if authorization.issued_at_ms > latest_acceptable_issue {
        return Err(DeviceAuthorizationVerificationError::NotYetValid);
    }

    if let Some(expires_at_ms) = authorization.expires_at_ms {
        let latest_acceptable_expiry = expires_at_ms.saturating_add(context.max_clock_skew_ms);

        if context.now_ms > latest_acceptable_expiry {
            return Err(DeviceAuthorizationVerificationError::Expired);
        }
    }

    let transcript = canonical_device_authorization_v1_transcript(&authorization.signing_payload())
        .map_err(|_| DeviceAuthorizationVerificationError::Transcript)?;

    let signature = Signature::from_bytes(authorization.root_signature.as_bytes());

    trusted_root
        .verify_strict(&transcript, &signature)
        .map_err(|_| DeviceAuthorizationVerificationError::InvalidRootSignature)
}

/// Derive the frozen Physical M1 Passport ID binding.
///
/// This preserves the already-proven Phase 0D derivation:
/// `domain|main|ed25519|root_public_key_hex`.
fn derive_frozen_passport_id_v1(
    root_public_key: &Ed25519PublicKeyHex,
) -> Result<PassportIdV1, DeviceAuthorizationVerificationError> {
    let hash_input = format!(
        "{}|main|ed25519|{}",
        PASSPORT_ID_V1_HASH_DOMAIN,
        root_public_key.as_str(),
    );

    let digest = blake3::hash(hash_input.as_bytes()).to_hex().to_string();

    PassportIdV1::parse(format!(
        "{}{}",
        PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX, digest,
    ))
    .map_err(|_| DeviceAuthorizationVerificationError::TrustedPassportRootBindingMismatch)
}

/// Derive the frozen Physical M1 Device ID binding.
///
/// This preserves the already-proven Phase 0F derivation:
/// `domain|ed25519|device_public_key_hex`.
fn derive_frozen_device_id_v1(
    device_public_key: &Ed25519PublicKeyHex,
) -> Result<DeviceIdV1, DeviceAuthorizationVerificationError> {
    let hash_input = format!(
        "{}|ed25519|{}",
        DEVICE_ID_V1_HASH_DOMAIN,
        device_public_key.as_str(),
    );

    let digest = blake3::hash(hash_input.as_bytes()).to_hex().to_string();

    DeviceIdV1::parse(format!("{}{}", DEVICE_ID_V1_ED25519_B3_PREFIX, digest,))
        .map_err(|_| DeviceAuthorizationVerificationError::DeviceIdBindingMismatch)
}

fn decode_lower_hex_32(value: &str) -> Option<[u8; 32]> {
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
