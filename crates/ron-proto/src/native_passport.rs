//! RO:WHAT — Canonical Native Passport V1 authorization, challenge, device-bound capability, and request-proof wire/domain types.
//! RO:WHY — Physical M1 requires one strict protocol-owned identity authority schema before cryptographic verification or svc-passport runtime mutation is admitted.
//! RO:INTERACTS — canonical Passport/Device/Capability IDs, DeviceAuthorizationV1, PassportChallengeV1, device-bound capability/request-proof DTOs, ron-auth canonical transcripts, svc-passport runtime, and ron-policy scope ceilings.
//! RO:INVARIANTS — V1 only; bounded canonical IDs/context/scopes; challenge, authorization, capability, and request-proof structures validate strictly; unknown fields fail closed where defined; no policy or runtime authority lives here.
//! RO:METRICS — none.
//! RO:CONFIG — none; runtime TTL, freshness, route policy, network authority, and persistence remain outside ron-proto.
//! RO:SECURITY — public protocol material only; no private key, PIN, vault key, signing runtime, capability issuance, namespace mutation, wallet, or ledger authority.
//! RO:TEST — Native Passport authorization, challenge, capability, and request-proof wire tests.

use std::fmt;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::{DeviceClassV1, DeviceIdV1, Ed25519PublicKeyHex, PassportIdV1};

mod capability;
pub use capability::*;

mod challenge;
pub use challenge::*;

/// Canonical Native Passport device-authorization version.
pub const DEVICE_AUTHORIZATION_V1_VERSION: u16 = 1;

/// Authorization nonce size.
pub const DEVICE_AUTHORIZATION_V1_NONCE_BYTES: usize = 16;

/// Ed25519 signature size.
pub const DEVICE_AUTHORIZATION_V1_SIGNATURE_BYTES: usize = 64;

/// Maximum network/environment token length.
pub const DEVICE_AUTHORIZATION_V1_CONTEXT_MAX_BYTES: usize = 64;

/// Maximum one scope token length.
pub const DEVICE_AUTHORIZATION_V1_SCOPE_MAX_BYTES: usize = 96;

/// Maximum number of scopes in one authorization ceiling.
pub const DEVICE_AUTHORIZATION_V1_MAX_SCOPES: usize = 32;

/// Deterministic validation errors for DeviceAuthorizationV1.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DeviceAuthorizationValidationError {
    /// Protocol version was not V1.
    #[error("unsupported DeviceAuthorizationV1 version")]
    UnsupportedVersion,

    /// Network/environment label was malformed.
    #[error("invalid Native Passport context label")]
    InvalidContextLabel,

    /// Scope token was malformed.
    #[error("invalid Native Passport scope token")]
    InvalidScopeToken,

    /// Scope count exceeded the V1 bound.
    #[error("DeviceAuthorizationV1 scope ceiling exceeds maximum")]
    TooManyScopes,

    /// Duplicate scope was present.
    #[error("DeviceAuthorizationV1 contains a duplicate scope")]
    DuplicateScope,

    /// Scope ordering was not canonical UTF-8 lexical order.
    #[error("DeviceAuthorizationV1 scopes are not canonically sorted")]
    NonCanonicalScopeOrder,

    /// Authorization nonce was malformed or non-canonical.
    #[error("invalid DeviceAuthorizationV1 authorization nonce")]
    InvalidAuthorizationNonce,

    /// Root signature was malformed or non-canonical.
    #[error("invalid DeviceAuthorizationV1 Ed25519 signature")]
    InvalidRootSignature,

    /// Issuance timestamp must be non-zero.
    #[error("DeviceAuthorizationV1 issued_at_ms must be non-zero")]
    InvalidIssuedAt,

    /// Optional expiry must be later than issuance.
    #[error("DeviceAuthorizationV1 expires_at_ms must be later than issued_at_ms")]
    InvalidExpiry,
}

/// Strict bounded network/environment token.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct NativePassportContextLabelV1(String);

impl NativePassportContextLabelV1 {
    /// Parse one canonical lowercase context label.
    pub fn parse(value: impl Into<String>) -> Result<Self, DeviceAuthorizationValidationError> {
        let value = value.into();

        if value.is_empty()
            || value.len() > DEVICE_AUTHORIZATION_V1_CONTEXT_MAX_BYTES
            || !value.bytes().all(is_context_byte)
        {
            return Err(DeviceAuthorizationValidationError::InvalidContextLabel);
        }

        Ok(Self(value))
    }

    /// Borrow canonical text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for NativePassportContextLabelV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// Strict syntax-only Native Passport scope token.
///
/// Authorization policy is intentionally not encoded here. `ron-policy` and
/// verification context decide which scopes a device class may receive.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct NativePassportScopeV1(String);

impl NativePassportScopeV1 {
    /// Parse one canonical scope token.
    pub fn parse(value: impl Into<String>) -> Result<Self, DeviceAuthorizationValidationError> {
        let value = value.into();
        let bytes = value.as_bytes();

        if bytes.is_empty()
            || bytes.len() > DEVICE_AUTHORIZATION_V1_SCOPE_MAX_BYTES
            || !bytes[0].is_ascii_lowercase()
            || !bytes.iter().copied().all(is_scope_byte)
        {
            return Err(DeviceAuthorizationValidationError::InvalidScopeToken);
        }

        Ok(Self(value))
    }

    /// Borrow canonical scope text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NativePassportScopeV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for NativePassportScopeV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// Canonically sorted/unique scope ceiling.
///
/// Empty is wire-valid because `recovery_only` may intentionally possess no
/// ordinary network capability. Class-specific minimum/maximum policy belongs
/// outside ron-proto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct DeviceAuthorizationScopeCeilingV1(Vec<NativePassportScopeV1>);

impl DeviceAuthorizationScopeCeilingV1 {
    /// Construct a bounded, strictly sorted scope ceiling.
    pub fn new(
        scopes: Vec<NativePassportScopeV1>,
    ) -> Result<Self, DeviceAuthorizationValidationError> {
        validate_scope_ceiling(&scopes)?;
        Ok(Self(scopes))
    }

    /// Borrow canonical scopes.
    #[must_use]
    pub fn as_slice(&self) -> &[NativePassportScopeV1] {
        &self.0
    }

    /// Consume into canonical scopes.
    #[must_use]
    pub fn into_vec(self) -> Vec<NativePassportScopeV1> {
        self.0
    }
}

impl<'de> Deserialize<'de> for DeviceAuthorizationScopeCeilingV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let scopes = Vec::<NativePassportScopeV1>::deserialize(deserializer)?;

        Self::new(scopes).map_err(serde::de::Error::custom)
    }
}

/// Exact 16-byte authorization nonce.
///
/// JSON representation is canonical base64url without padding.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct DeviceAuthorizationNonceV1([u8; DEVICE_AUTHORIZATION_V1_NONCE_BYTES]);

impl DeviceAuthorizationNonceV1 {
    /// Construct from exact nonce bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; DEVICE_AUTHORIZATION_V1_NONCE_BYTES]) -> Self {
        Self(bytes)
    }

    /// Parse canonical base64url without padding.
    pub fn parse_base64url(value: &str) -> Result<Self, DeviceAuthorizationValidationError> {
        let decoded = URL_SAFE_NO_PAD
            .decode(value.as_bytes())
            .map_err(|_| DeviceAuthorizationValidationError::InvalidAuthorizationNonce)?;

        let bytes: [u8; DEVICE_AUTHORIZATION_V1_NONCE_BYTES] = decoded
            .try_into()
            .map_err(|_| DeviceAuthorizationValidationError::InvalidAuthorizationNonce)?;

        if URL_SAFE_NO_PAD.encode(bytes) != value {
            return Err(DeviceAuthorizationValidationError::InvalidAuthorizationNonce);
        }

        Ok(Self(bytes))
    }

    /// Borrow exact nonce bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; DEVICE_AUTHORIZATION_V1_NONCE_BYTES] {
        &self.0
    }

    /// Return canonical JSON/wire text.
    #[must_use]
    pub fn to_base64url(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }
}

impl fmt::Debug for DeviceAuthorizationNonceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DeviceAuthorizationNonceV1(..)")
    }
}

impl Serialize for DeviceAuthorizationNonceV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_base64url())
    }
}

impl<'de> Deserialize<'de> for DeviceAuthorizationNonceV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        Self::parse_base64url(&value).map_err(serde::de::Error::custom)
    }
}

/// Exact 64-byte Ed25519 signature.
///
/// JSON representation is canonical base64url without padding.
#[derive(Clone, PartialEq, Eq)]
pub struct Ed25519SignatureV1([u8; DEVICE_AUTHORIZATION_V1_SIGNATURE_BYTES]);

impl Ed25519SignatureV1 {
    /// Construct from exact Ed25519 signature bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; DEVICE_AUTHORIZATION_V1_SIGNATURE_BYTES]) -> Self {
        Self(bytes)
    }

    /// Parse canonical base64url without padding.
    pub fn parse_base64url(value: &str) -> Result<Self, DeviceAuthorizationValidationError> {
        let decoded = URL_SAFE_NO_PAD
            .decode(value.as_bytes())
            .map_err(|_| DeviceAuthorizationValidationError::InvalidRootSignature)?;

        let bytes: [u8; DEVICE_AUTHORIZATION_V1_SIGNATURE_BYTES] = decoded
            .try_into()
            .map_err(|_| DeviceAuthorizationValidationError::InvalidRootSignature)?;

        if URL_SAFE_NO_PAD.encode(bytes) != value {
            return Err(DeviceAuthorizationValidationError::InvalidRootSignature);
        }

        Ok(Self(bytes))
    }

    /// Borrow exact signature bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; DEVICE_AUTHORIZATION_V1_SIGNATURE_BYTES] {
        &self.0
    }

    /// Return canonical JSON/wire text.
    #[must_use]
    pub fn to_base64url(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }
}

impl fmt::Debug for Ed25519SignatureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Ed25519SignatureV1(..)")
    }
}

impl Serialize for Ed25519SignatureV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_base64url())
    }
}

impl<'de> Deserialize<'de> for Ed25519SignatureV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        Self::parse_base64url(&value).map_err(serde::de::Error::custom)
    }
}

/// Unsigned fields covered by the Passport-root signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceAuthorizationSigningPayloadV1 {
    /// Protocol version.
    pub version: u16,

    /// Network binding.
    pub network_id: NativePassportContextLabelV1,

    /// Environment binding.
    pub environment: NativePassportContextLabelV1,

    /// Passport receiving this device authorization.
    pub passport_id: PassportIdV1,

    /// Passport root-key epoch.
    pub root_key_epoch: u64,

    /// Authorized device identifier.
    pub device_id: DeviceIdV1,

    /// Authorized device Ed25519 public key.
    pub device_public_key: Ed25519PublicKeyHex,

    /// Authorized device class.
    pub device_class: DeviceClassV1,

    /// Maximum scope vocabulary the device may later request.
    pub authorized_scope_ceiling: DeviceAuthorizationScopeCeilingV1,

    /// Unique authorization nonce.
    pub authorization_nonce: DeviceAuthorizationNonceV1,

    /// Authorization issue time in Unix milliseconds.
    pub issued_at_ms: u64,

    /// Optional expiration time in Unix milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_ms: Option<u64>,
}

impl DeviceAuthorizationSigningPayloadV1 {
    /// Validate non-cryptographic V1 invariants.
    pub fn validate(&self) -> Result<(), DeviceAuthorizationValidationError> {
        if self.version != DEVICE_AUTHORIZATION_V1_VERSION {
            return Err(DeviceAuthorizationValidationError::UnsupportedVersion);
        }

        validate_scope_ceiling(self.authorized_scope_ceiling.as_slice())?;

        if self.issued_at_ms == 0 {
            return Err(DeviceAuthorizationValidationError::InvalidIssuedAt);
        }

        if let Some(expires_at_ms) = self.expires_at_ms {
            if expires_at_ms <= self.issued_at_ms {
                return Err(DeviceAuthorizationValidationError::InvalidExpiry);
            }
        }

        Ok(())
    }
}

/// Root-signed canonical Native Passport device authorization.
///
/// The trusted Passport root public key is deliberately not carried as an
/// authority-bearing field here. Verification receives the trusted root record
/// separately and validates Passport ID/root-key binding before accepting this
/// authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceAuthorizationV1 {
    /// Protocol version.
    pub version: u16,

    /// Network binding.
    pub network_id: NativePassportContextLabelV1,

    /// Environment binding.
    pub environment: NativePassportContextLabelV1,

    /// Passport receiving this device authorization.
    pub passport_id: PassportIdV1,

    /// Passport root-key epoch.
    pub root_key_epoch: u64,

    /// Authorized device identifier.
    pub device_id: DeviceIdV1,

    /// Authorized device Ed25519 public key.
    pub device_public_key: Ed25519PublicKeyHex,

    /// Authorized device class.
    pub device_class: DeviceClassV1,

    /// Maximum scope vocabulary the device may later request.
    pub authorized_scope_ceiling: DeviceAuthorizationScopeCeilingV1,

    /// Unique authorization nonce.
    pub authorization_nonce: DeviceAuthorizationNonceV1,

    /// Authorization issue time in Unix milliseconds.
    pub issued_at_ms: u64,

    /// Optional expiration time in Unix milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_ms: Option<u64>,

    /// Passport-root Ed25519 signature over the canonical transcript.
    pub root_signature: Ed25519SignatureV1,
}

impl DeviceAuthorizationV1 {
    /// Construct a signed authorization from validated signing fields.
    pub fn from_signing_payload(
        payload: DeviceAuthorizationSigningPayloadV1,
        root_signature: Ed25519SignatureV1,
    ) -> Result<Self, DeviceAuthorizationValidationError> {
        payload.validate()?;

        Ok(Self {
            version: payload.version,
            network_id: payload.network_id,
            environment: payload.environment,
            passport_id: payload.passport_id,
            root_key_epoch: payload.root_key_epoch,
            device_id: payload.device_id,
            device_public_key: payload.device_public_key,
            device_class: payload.device_class,
            authorized_scope_ceiling: payload.authorized_scope_ceiling,
            authorization_nonce: payload.authorization_nonce,
            issued_at_ms: payload.issued_at_ms,
            expires_at_ms: payload.expires_at_ms,
            root_signature,
        })
    }

    /// Recover the exact unsigned fields covered by the root signature.
    #[must_use]
    pub fn signing_payload(&self) -> DeviceAuthorizationSigningPayloadV1 {
        DeviceAuthorizationSigningPayloadV1 {
            version: self.version,
            network_id: self.network_id.clone(),
            environment: self.environment.clone(),
            passport_id: self.passport_id.clone(),
            root_key_epoch: self.root_key_epoch,
            device_id: self.device_id.clone(),
            device_public_key: self.device_public_key.clone(),
            device_class: self.device_class,
            authorized_scope_ceiling: self.authorized_scope_ceiling.clone(),
            authorization_nonce: self.authorization_nonce.clone(),
            issued_at_ms: self.issued_at_ms,
            expires_at_ms: self.expires_at_ms,
        }
    }

    /// Validate non-cryptographic V1 invariants.
    pub fn validate(&self) -> Result<(), DeviceAuthorizationValidationError> {
        self.signing_payload().validate()
    }
}

fn validate_scope_ceiling(
    scopes: &[NativePassportScopeV1],
) -> Result<(), DeviceAuthorizationValidationError> {
    if scopes.len() > DEVICE_AUTHORIZATION_V1_MAX_SCOPES {
        return Err(DeviceAuthorizationValidationError::TooManyScopes);
    }

    for pair in scopes.windows(2) {
        let left = pair[0].as_str();
        let right = pair[1].as_str();

        if left == right {
            return Err(DeviceAuthorizationValidationError::DuplicateScope);
        }

        if left > right {
            return Err(DeviceAuthorizationValidationError::NonCanonicalScopeOrder);
        }
    }

    Ok(())
}

fn is_context_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b':' | b'-')
}

fn is_scope_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b':' | b'-')
}
