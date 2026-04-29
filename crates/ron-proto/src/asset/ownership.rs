//! RO:WHAT — Ownership claim DTOs for binding an asset CID to passport/wallet identity.
//! RO:WHY — WEB3_2 needs signed ownership shapes without making ron-proto verify signatures.
//! RO:INTERACTS — asset manifests, svc-passport, ron-auth, ron-kms, svc-index.
//! RO:INVARIANTS — no crypto here; signatures are represented only; asset CID is canonical ContentId.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — claim fields are strict; verification belongs to auth/passport/KMS layers.
//! RO:TEST — asset_manifest.rs validates required claim fields and stable serde shape.

use serde::{Deserialize, Serialize};

use super::{require_non_empty_bounded, AssetKind, AssetValidationError, MAX_REF_BYTES};

/// Current ownership claim DTO version.
pub const ASSET_OWNERSHIP_CLAIM_VERSION: u16 = 1;

/// Ownership claim type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OwnershipClaimType {
    /// Original creator claim.
    OriginalCreator,
    /// Ownership transfer claim.
    Transfer,
    /// Curator/collection claim.
    Curator,
    /// License or reuse claim.
    LicenseGrant,
}

/// Signature reference carried by ownership claims.
///
/// This is intentionally not a verifier. Verification belongs to `ron-auth`,
/// `svc-passport`, and `ron-kms`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SignatureRefV1 {
    /// Signature algorithm tag.
    pub alg: String,
    /// Public key reference or key ID.
    pub public_key_ref: String,
    /// Signature bytes encoded by the signing layer.
    pub signature: String,
}

impl SignatureRefV1 {
    /// Validate required signature reference fields.
    pub fn validate(&self) -> Result<(), AssetValidationError> {
        require_non_empty_bounded("signature.alg", &self.alg, MAX_REF_BYTES)?;
        require_non_empty_bounded(
            "signature.public_key_ref",
            &self.public_key_ref,
            MAX_REF_BYTES,
        )?;
        require_non_empty_bounded("signature.signature", &self.signature, MAX_REF_BYTES)?;
        Ok(())
    }
}

/// Signed claim binding asset CID, kind, passport subject, wallet account, timestamp, and signature.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetOwnershipClaimV1 {
    /// DTO version. Must be `1`.
    pub version: u16,
    /// Asset content ID.
    pub asset_cid: crate::id::ContentId,
    /// Asset kind.
    pub asset_kind: AssetKind,
    /// Claim type.
    pub claim_type: OwnershipClaimType,
    /// Passport subject making the claim.
    pub passport_subject: String,
    /// Wallet account bound to the claim.
    pub wallet_account: String,
    /// Claim issuance timestamp in milliseconds since Unix epoch.
    pub issued_at_ms: u64,
    /// Optional expiration timestamp in milliseconds since Unix epoch.
    #[serde(default)]
    pub expires_at_ms: Option<u64>,
    /// Signature reference.
    pub signature: SignatureRefV1,
}

impl AssetOwnershipClaimV1 {
    /// Validate ownership claim shape.
    pub fn validate(&self) -> Result<(), AssetValidationError> {
        if self.version != ASSET_OWNERSHIP_CLAIM_VERSION {
            return Err(AssetValidationError::InvalidVersion {
                ty: "AssetOwnershipClaimV1",
                expected: ASSET_OWNERSHIP_CLAIM_VERSION,
                actual: self.version,
            });
        }

        require_non_empty_bounded(
            "ownership.passport_subject",
            &self.passport_subject,
            MAX_REF_BYTES,
        )?;
        require_non_empty_bounded(
            "ownership.wallet_account",
            &self.wallet_account,
            MAX_REF_BYTES,
        )?;

        if self.issued_at_ms == 0 {
            return Err(AssetValidationError::EmptyField {
                field: "ownership.issued_at_ms",
            });
        }

        if let Some(expires_at_ms) = self.expires_at_ms {
            if expires_at_ms <= self.issued_at_ms {
                return Err(AssetValidationError::InvalidTimeOrder {
                    field: "ownership.expires_at_ms",
                });
            }
        }

        self.signature.validate()
    }
}
