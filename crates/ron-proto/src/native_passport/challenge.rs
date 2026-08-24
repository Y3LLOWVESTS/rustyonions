//! RO:WHAT — Canonical Native Passport V1 signed service-challenge wire/domain types.
//! RO:WHY — Clients must verify a purpose-bound one-time challenge before root or device signing, using one protocol-owned DTO rather than service-local placeholder contracts.
//! RO:INTERACTS — ChallengeIdV1, PassportIdV1, DeviceIdV1, B3DigestHex, NativePassportContextLabelV1, NativePassportScopeV1, Ed25519SignatureV1, ron-auth challenge transcripts/verifier, and svc-passport challenge issuance/replay.
//! RO:INVARIANTS — V1 only; closed purpose vocabulary; reserved wallet purpose is rejected; requested scopes are bounded/sorted/unique; purpose-required Passport/device/body bindings are present; nonce/body hashes are strict 32-byte lowercase hex DTOs; issue/expiry are bounded; service signature is exact Ed25519 material.
//! RO:METRICS — none.
//! RO:CONFIG — V1 challenge TTL is capped at 300 seconds; trusted verifier clock skew is capped at 30 seconds.
//! RO:SECURITY — public signed challenge material only; no service private key, root/device private key, PIN, vault material, capability, replay mutation, wallet, or ledger authority.
//! RO:TEST — tests/native_passport_challenge_v1_wire.rs.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{B3DigestHex, ChallengeIdV1, DeviceIdV1, PassportIdV1};

use super::{
    Ed25519SignatureV1, NativePassportContextLabelV1, NativePassportScopeV1,
    DEVICE_AUTHORIZATION_V1_MAX_SCOPES,
};

/// Canonical Native Passport signed service-challenge protocol version.
pub const PASSPORT_CHALLENGE_V1_VERSION: u16 = 1;

/// Maximum lifetime of one V1 challenge.
pub const PASSPORT_CHALLENGE_V1_MAX_TTL_MS: u64 = 300_000;

/// Maximum verifier-controlled clock skew accepted by V1.
pub const PASSPORT_CHALLENGE_V1_MAX_CLOCK_SKEW_MS: u64 = 30_000;

/// V1 service challenge signatures are Ed25519.
pub const PASSPORT_CHALLENGE_V1_SERVICE_SIGNATURE_ALGORITHM: &str = "ed25519";

/// Maximum canonical service signing-key identifier length.
pub const PASSPORT_CHALLENGE_V1_SERVICE_KEY_ID_MAX_BYTES: usize = 128;

/// Canonical identifier for the concrete service key signing a challenge.
///
/// This is deliberately separate from network/environment context labels.
/// Existing svc-passport KMS identifiers use the namespace
/// `ed25519/default/v{n}`, so `/` is canonical here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ServiceKeyIdV1(String);

impl ServiceKeyIdV1 {
    /// Parse one bounded canonical service signing-key identifier.
    pub fn parse(value: impl Into<String>) -> Result<Self, PassportChallengeValidationError> {
        let value = value.into();
        let bytes = value.as_bytes();

        if bytes.is_empty()
            || bytes.len() > PASSPORT_CHALLENGE_V1_SERVICE_KEY_ID_MAX_BYTES
            || !matches!(
                bytes[0],
                b'a'..=b'z' | b'0'..=b'9'
            )
            || !bytes.iter().copied().all(is_service_key_id_byte)
        {
            return Err(PassportChallengeValidationError::InvalidServiceKeyId);
        }

        Ok(Self(value))
    }

    /// Borrow canonical service-key identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ServiceKeyIdV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ServiceKeyIdV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

fn is_service_key_id_byte(byte: u8) -> bool {
    matches!(
        byte,
        b'a'..=b'z'
            | b'0'..=b'9'
            | b'.'
            | b'_'
            | b':'
            | b'-'
            | b'/'
    )
}

/// Closed Native Passport challenge purpose vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassportChallengePurposeV1 {
    /// Register a Passport root after proof of concrete root-key control.
    RegisterRoot,
    /// Authorize a device with Passport-root authority.
    AuthorizeDevice,
    /// Revoke an already-authorized device.
    RevokeDevice,
    /// Prove a routine device session.
    ProveSession,
    /// Request a device-bound capability.
    IssueCapability,
    /// Refresh a device-bound capability.
    RefreshCapability,
    /// Claim a username.
    ClaimUsername,
    /// Transfer a username.
    TransferUsername,
    /// Release a username.
    ReleaseUsername,
    /// Publish a public profile transition.
    PublishProfile,
    /// Register a site transition.
    RegisterSite,
    /// Update a site transition.
    UpdateSite,
    /// Reserved until a separate economic authorization design enables it.
    WalletAuthorizationRequestReserved,
}

impl PassportChallengePurposeV1 {
    /// Stable canonical transcript spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RegisterRoot => "register_root",
            Self::AuthorizeDevice => "authorize_device",
            Self::RevokeDevice => "revoke_device",
            Self::ProveSession => "prove_session",
            Self::IssueCapability => "issue_capability",
            Self::RefreshCapability => "refresh_capability",
            Self::ClaimUsername => "claim_username",
            Self::TransferUsername => "transfer_username",
            Self::ReleaseUsername => "release_username",
            Self::PublishProfile => "publish_profile",
            Self::RegisterSite => "register_site",
            Self::UpdateSite => "update_site",
            Self::WalletAuthorizationRequestReserved => "wallet_authorization_request_reserved",
        }
    }

    /// Whether this V1 purpose must identify a Passport.
    #[must_use]
    pub const fn requires_passport_binding(self) -> bool {
        matches!(
            self,
            Self::RegisterRoot
                | Self::AuthorizeDevice
                | Self::RevokeDevice
                | Self::ProveSession
                | Self::IssueCapability
                | Self::RefreshCapability
                | Self::ClaimUsername
                | Self::TransferUsername
                | Self::ReleaseUsername
                | Self::PublishProfile
                | Self::RegisterSite
                | Self::UpdateSite
        )
    }

    /// Whether this V1 purpose must directly identify a device.
    ///
    /// `authorize_device` binds the new device through the operation body,
    /// while `register_root` binds the initial registration body.
    #[must_use]
    pub const fn requires_device_binding(self) -> bool {
        matches!(
            self,
            Self::RevokeDevice
                | Self::ProveSession
                | Self::IssueCapability
                | Self::RefreshCapability
                | Self::PublishProfile
                | Self::RegisterSite
                | Self::UpdateSite
        )
    }

    /// Whether this V1 purpose must bind an exact operation body.
    #[must_use]
    pub const fn requires_operation_body_hash(self) -> bool {
        matches!(
            self,
            Self::RegisterRoot
                | Self::AuthorizeDevice
                | Self::RevokeDevice
                | Self::IssueCapability
                | Self::RefreshCapability
                | Self::ClaimUsername
                | Self::TransferUsername
                | Self::ReleaseUsername
                | Self::PublishProfile
                | Self::RegisterSite
                | Self::UpdateSite
        )
    }

    /// Wallet authorization remains explicitly reserved.
    #[must_use]
    pub const fn is_reserved(self) -> bool {
        matches!(self, Self::WalletAuthorizationRequestReserved)
    }
}

/// Deterministic validation failures for `PassportChallengeV1`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PassportChallengeValidationError {
    /// Protocol version was not V1.
    #[error("unsupported PassportChallengeV1 version")]
    UnsupportedVersion,

    /// Service signing-key identifier was malformed.
    #[error("PassportChallengeV1 service_key_id is invalid")]
    InvalidServiceKeyId,

    /// Wallet authorization is not enabled by this protocol phase.
    #[error("PassportChallengeV1 wallet authorization purpose is reserved")]
    ReservedWalletPurpose,

    /// Requested scope set was empty.
    #[error("PassportChallengeV1 requested scope set is empty")]
    EmptyRequestedScopes,

    /// Requested scope count exceeded the V1 bound.
    #[error("PassportChallengeV1 requested scope count exceeds maximum")]
    TooManyRequestedScopes,

    /// Duplicate requested scope was present.
    #[error("PassportChallengeV1 contains a duplicate requested scope")]
    DuplicateRequestedScope,

    /// Requested scope order was not canonical UTF-8 lexical order.
    #[error("PassportChallengeV1 requested scopes are not canonically sorted")]
    NonCanonicalRequestedScopeOrder,

    /// Purpose requires a Passport binding.
    #[error("PassportChallengeV1 purpose requires passport_id")]
    MissingPassportBinding,

    /// Purpose requires a Device binding.
    #[error("PassportChallengeV1 purpose requires device_id")]
    MissingDeviceBinding,

    /// Purpose requires an exact operation body hash.
    #[error("PassportChallengeV1 purpose requires operation_body_hash")]
    MissingOperationBodyHash,

    /// Issuance timestamp must be non-zero.
    #[error("PassportChallengeV1 issued_at_ms must be non-zero")]
    InvalidIssuedAt,

    /// Expiry must be strictly later than issuance.
    #[error("PassportChallengeV1 expiry window is invalid")]
    InvalidExpiry,

    /// Challenge lifetime exceeded the V1 maximum.
    #[error("PassportChallengeV1 lifetime exceeds maximum")]
    ChallengeTtlTooLong,
}

/// Exact public fields covered by the service challenge signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassportChallengeSigningPayloadV1 {
    /// Protocol version.
    pub version: u16,

    /// Unique one-time challenge identifier.
    pub challenge_id: ChallengeIdV1,

    /// Network binding.
    pub network_id: NativePassportContextLabelV1,

    /// Environment binding.
    pub environment: NativePassportContextLabelV1,

    /// Intended service/client audience.
    pub audience: NativePassportContextLabelV1,

    /// Service identity that issued the challenge.
    pub issuing_service_id: NativePassportContextLabelV1,

    /// Trusted service signing-key identifier.
    pub service_key_id: ServiceKeyIdV1,

    /// Exact requested operation purpose.
    pub purpose: PassportChallengePurposeV1,

    /// Canonically sorted and unique requested scopes.
    pub requested_scopes: Vec<NativePassportScopeV1>,

    /// Optional Passport binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passport_id: Option<PassportIdV1>,

    /// Optional Device binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<DeviceIdV1>,

    /// Optional exact operation body hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_body_hash: Option<B3DigestHex>,

    /// Fresh public challenge nonce encoded as exact 32-byte lowercase hex.
    pub nonce: B3DigestHex,

    /// Issue time in milliseconds.
    pub issued_at_ms: u64,

    /// Expiry time in milliseconds.
    pub expires_at_ms: u64,
}

impl PassportChallengeSigningPayloadV1 {
    /// Validate protocol-owned V1 structure and purpose bindings.
    pub fn validate(&self) -> Result<(), PassportChallengeValidationError> {
        if self.version != PASSPORT_CHALLENGE_V1_VERSION {
            return Err(PassportChallengeValidationError::UnsupportedVersion);
        }

        if self.purpose.is_reserved() {
            return Err(PassportChallengeValidationError::ReservedWalletPurpose);
        }

        validate_requested_scopes(&self.requested_scopes)?;

        if self.purpose.requires_passport_binding() && self.passport_id.is_none() {
            return Err(PassportChallengeValidationError::MissingPassportBinding);
        }

        if self.purpose.requires_device_binding() && self.device_id.is_none() {
            return Err(PassportChallengeValidationError::MissingDeviceBinding);
        }

        if self.purpose.requires_operation_body_hash() && self.operation_body_hash.is_none() {
            return Err(PassportChallengeValidationError::MissingOperationBodyHash);
        }

        if self.issued_at_ms == 0 {
            return Err(PassportChallengeValidationError::InvalidIssuedAt);
        }

        if self.expires_at_ms <= self.issued_at_ms {
            return Err(PassportChallengeValidationError::InvalidExpiry);
        }

        if self.expires_at_ms - self.issued_at_ms > PASSPORT_CHALLENGE_V1_MAX_TTL_MS {
            return Err(PassportChallengeValidationError::ChallengeTtlTooLong);
        }

        Ok(())
    }
}

/// Service-signed one-time Native Passport challenge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassportChallengeV1 {
    /// Protocol version.
    pub version: u16,

    /// Unique one-time challenge identifier.
    pub challenge_id: ChallengeIdV1,

    /// Network binding.
    pub network_id: NativePassportContextLabelV1,

    /// Environment binding.
    pub environment: NativePassportContextLabelV1,

    /// Intended service/client audience.
    pub audience: NativePassportContextLabelV1,

    /// Service identity that issued the challenge.
    pub issuing_service_id: NativePassportContextLabelV1,

    /// Trusted service signing-key identifier.
    pub service_key_id: ServiceKeyIdV1,

    /// Exact requested operation purpose.
    pub purpose: PassportChallengePurposeV1,

    /// Canonically sorted and unique requested scopes.
    pub requested_scopes: Vec<NativePassportScopeV1>,

    /// Optional Passport binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passport_id: Option<PassportIdV1>,

    /// Optional Device binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<DeviceIdV1>,

    /// Optional exact operation body hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_body_hash: Option<B3DigestHex>,

    /// Fresh public challenge nonce.
    pub nonce: B3DigestHex,

    /// Issue time in milliseconds.
    pub issued_at_ms: u64,

    /// Expiry time in milliseconds.
    pub expires_at_ms: u64,

    /// Exact Ed25519 service signature over the canonical V1 transcript.
    pub service_signature: Ed25519SignatureV1,
}

impl PassportChallengeV1 {
    /// Reconstruct exactly the public fields covered by the service signature.
    #[must_use]
    pub fn signing_payload(&self) -> PassportChallengeSigningPayloadV1 {
        PassportChallengeSigningPayloadV1 {
            version: self.version,
            challenge_id: self.challenge_id.clone(),
            network_id: self.network_id.clone(),
            environment: self.environment.clone(),
            audience: self.audience.clone(),
            issuing_service_id: self.issuing_service_id.clone(),
            service_key_id: self.service_key_id.clone(),
            purpose: self.purpose,
            requested_scopes: self.requested_scopes.clone(),
            passport_id: self.passport_id.clone(),
            device_id: self.device_id.clone(),
            operation_body_hash: self.operation_body_hash.clone(),
            nonce: self.nonce.clone(),
            issued_at_ms: self.issued_at_ms,
            expires_at_ms: self.expires_at_ms,
        }
    }

    /// Validate all unsigned V1 structure before cryptographic verification.
    pub fn validate(&self) -> Result<(), PassportChallengeValidationError> {
        self.signing_payload().validate()
    }
}

fn validate_requested_scopes(
    scopes: &[NativePassportScopeV1],
) -> Result<(), PassportChallengeValidationError> {
    if scopes.is_empty() {
        return Err(PassportChallengeValidationError::EmptyRequestedScopes);
    }

    if scopes.len() > DEVICE_AUTHORIZATION_V1_MAX_SCOPES {
        return Err(PassportChallengeValidationError::TooManyRequestedScopes);
    }

    for pair in scopes.windows(2) {
        if pair[0] == pair[1] {
            return Err(PassportChallengeValidationError::DuplicateRequestedScope);
        }

        if pair[0] > pair[1] {
            return Err(PassportChallengeValidationError::NonCanonicalRequestedScopeOrder);
        }
    }

    Ok(())
}
