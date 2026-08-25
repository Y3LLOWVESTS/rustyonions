//! RO:WHAT — Canonical Native Passport V1 device-bound capability and request-proof wire DTOs.
//! RO:WHY — CN-4 requires protocol-owned capability identity and DeviceKey-bound request evidence before svc-passport may issue reusable authority or admit username mutation.
//! RO:INTERACTS — canonical Passport/Device/Capability IDs, Native Passport context/scope DTOs, Ed25519 signature DTOs, future ron-auth canonical transcripts, and future svc-passport durable capability runtime.
//! RO:INVARIANTS — V1 only; capabilities are Passport- and Device-bound; scopes are non-empty, bounded, sorted, and unique; issue/expiry and policy version are valid; request proofs bind one capability ID, canonical method/path, query/body hashes, timestamp, nonce, Device ID, and exact Ed25519 signature bytes.
//! RO:METRICS — none.
//! RO:CONFIG — none; TTL ceilings, freshness windows, route scope policy, and runtime network authority remain outside ron-proto.
//! RO:SECURITY — public protocol material only; no key custody, signing, KMS, persistence, replay mutation, capability issuance, username mutation, wallet, or ledger authority.
//! RO:TEST — tests/native_passport_capability_v1_wire.rs.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{B3DigestHex, CapabilityIdV1, DeviceIdV1, PassportIdV1};

use super::{
    Ed25519SignatureV1, NativePassportContextLabelV1, NativePassportScopeV1,
    DEVICE_AUTHORIZATION_V1_MAX_SCOPES,
};

/// Canonical Native Passport device-bound capability version.
pub const NATIVE_PASSPORT_DEVICE_BOUND_CAPABILITY_V1_VERSION: u16 = 1;

/// Canonical Native Passport protected-request proof version.
pub const PASSPORT_REQUEST_PROOF_V1_VERSION: u16 = 1;

/// Maximum canonical HTTP-style method token length.
pub const PASSPORT_REQUEST_PROOF_V1_MAX_METHOD_BYTES: usize = 16;

/// Maximum canonical path bytes carried by one request proof.
pub const PASSPORT_REQUEST_PROOF_V1_MAX_PATH_BYTES: usize = 4096;

/// Structural validation failures for capability/request-proof wire DTOs.
///
/// Cryptographic verification, expiry relative to trusted current time,
/// revocation, scope authorization, nonce replay, and route authorization are
/// deliberately runtime responsibilities.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum NativePassportCapabilityValidationError {
    #[error("unsupported Native Passport device-bound capability version")]
    UnsupportedCapabilityVersion,

    #[error("unsupported PassportRequestProofV1 version")]
    UnsupportedRequestProofVersion,

    #[error("Native Passport capability scope list must not be empty")]
    EmptyCapabilityScopes,

    #[error("Native Passport capability scope list exceeds maximum")]
    TooManyCapabilityScopes,

    #[error("Native Passport capability contains a duplicate scope")]
    DuplicateCapabilityScope,

    #[error("Native Passport capability scopes are not canonically sorted")]
    NonCanonicalCapabilityScopeOrder,

    #[error("Native Passport capability issued_at_ms must be non-zero")]
    InvalidCapabilityIssuedAt,

    #[error("Native Passport capability expiry must be later than issuance")]
    InvalidCapabilityExpiry,

    #[error("Native Passport capability policy version must be non-zero")]
    InvalidCapabilityPolicyVersion,

    #[error("PassportRequestProofV1 request method is not canonical")]
    InvalidRequestMethod,

    #[error("PassportRequestProofV1 canonical path is invalid")]
    InvalidCanonicalPath,

    #[error("PassportRequestProofV1 timestamp_ms must be non-zero")]
    InvalidRequestTimestamp,
}

/// Server-authoritative Native Passport V1 device-bound capability record.
///
/// This is not a bearer credential and carries no private material. Runtime
/// authority remains in svc-passport durable capability state. Protected use
/// requires a matching [`PassportRequestProofV1`] signed by the bound DeviceKey.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativePassportDeviceBoundCapabilityV1 {
    pub version: u16,
    pub capability_id: CapabilityIdV1,
    pub passport_id: PassportIdV1,
    pub device_id: DeviceIdV1,
    pub audience: NativePassportContextLabelV1,
    pub environment: NativePassportContextLabelV1,
    pub scopes: Vec<NativePassportScopeV1>,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub policy_version: u16,

    /// Optional Passport root epoch bound into capability authority.
    ///
    /// Epoch zero is valid. Absence means a future runtime deliberately chose
    /// not to bind the capability to a root epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_key_epoch: Option<u64>,
}

impl NativePassportDeviceBoundCapabilityV1 {
    /// Validate protocol-owned structural invariants.
    pub fn validate(&self) -> Result<(), NativePassportCapabilityValidationError> {
        if self.version != NATIVE_PASSPORT_DEVICE_BOUND_CAPABILITY_V1_VERSION {
            return Err(NativePassportCapabilityValidationError::UnsupportedCapabilityVersion);
        }

        validate_capability_scopes(&self.scopes)?;

        if self.issued_at_ms == 0 {
            return Err(NativePassportCapabilityValidationError::InvalidCapabilityIssuedAt);
        }

        if self.expires_at_ms <= self.issued_at_ms {
            return Err(NativePassportCapabilityValidationError::InvalidCapabilityExpiry);
        }

        if self.policy_version == 0 {
            return Err(NativePassportCapabilityValidationError::InvalidCapabilityPolicyVersion);
        }

        Ok(())
    }
}

/// Canonical DeviceKey-signed evidence for one protected request.
///
/// The capability itself stays server-authoritative. This DTO proves that the
/// bound DeviceKey approved the exact request target/content at one timestamp
/// and nonce. ron-auth will own canonical transcript bytes and strict Ed25519
/// verification in the next slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassportRequestProofV1 {
    pub version: u16,
    pub capability_id: CapabilityIdV1,
    pub request_method: String,
    pub canonical_path: String,
    pub canonical_query_hash: B3DigestHex,
    pub body_hash: B3DigestHex,
    pub timestamp_ms: u64,
    pub request_nonce: B3DigestHex,
    pub device_id: DeviceIdV1,
    pub device_signature: Ed25519SignatureV1,
}

impl PassportRequestProofV1 {
    /// Validate protocol-owned structural invariants.
    pub fn validate(&self) -> Result<(), NativePassportCapabilityValidationError> {
        if self.version != PASSPORT_REQUEST_PROOF_V1_VERSION {
            return Err(NativePassportCapabilityValidationError::UnsupportedRequestProofVersion);
        }

        validate_request_method(&self.request_method)?;
        validate_canonical_path(&self.canonical_path)?;

        if self.timestamp_ms == 0 {
            return Err(NativePassportCapabilityValidationError::InvalidRequestTimestamp);
        }

        Ok(())
    }
}

fn validate_capability_scopes(
    scopes: &[NativePassportScopeV1],
) -> Result<(), NativePassportCapabilityValidationError> {
    if scopes.is_empty() {
        return Err(NativePassportCapabilityValidationError::EmptyCapabilityScopes);
    }

    if scopes.len() > DEVICE_AUTHORIZATION_V1_MAX_SCOPES {
        return Err(NativePassportCapabilityValidationError::TooManyCapabilityScopes);
    }

    for pair in scopes.windows(2) {
        let left = pair[0].as_str();
        let right = pair[1].as_str();

        if left == right {
            return Err(NativePassportCapabilityValidationError::DuplicateCapabilityScope);
        }

        if left > right {
            return Err(NativePassportCapabilityValidationError::NonCanonicalCapabilityScopeOrder);
        }
    }

    Ok(())
}

fn validate_request_method(method: &str) -> Result<(), NativePassportCapabilityValidationError> {
    let bytes = method.as_bytes();

    if bytes.is_empty()
        || bytes.len() > PASSPORT_REQUEST_PROOF_V1_MAX_METHOD_BYTES
        || !bytes.iter().all(u8::is_ascii_uppercase)
    {
        return Err(NativePassportCapabilityValidationError::InvalidRequestMethod);
    }

    Ok(())
}

fn validate_canonical_path(path: &str) -> Result<(), NativePassportCapabilityValidationError> {
    let bytes = path.as_bytes();

    if bytes.is_empty()
        || bytes.len() > PASSPORT_REQUEST_PROOF_V1_MAX_PATH_BYTES
        || !path.starts_with('/')
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_graphic() && !matches!(byte, b'?' | b'#'))
    {
        return Err(NativePassportCapabilityValidationError::InvalidCanonicalPath);
    }

    Ok(())
}
