//! RO:WHAT — Public passport/profile DTOs for NEXT_LEVEL identity discovery.
//! RO:WHY — Cross-service profile/passport manifests must be strict before svc-passport persistence or gateway hydration.
//! RO:INTERACTS — id::ContentId, asset::AssetKind, future svc-passport, omnigate profile resolver, CrabLink profile UI.
//! RO:INVARIANTS — DTO-only; no IO; no username uniqueness; no key custody; no wallet/ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — public DTOs must not contain private keys, spend authority, private alt mappings, or recovery data.
//! RO:TEST — tests/identity_profile.rs.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{asset::AssetKind, id::ContentId};

/// Current public profile DTO version.
pub const PASSPORT_PUBLIC_PROFILE_VERSION: u16 = 1;
/// Current public passport manifest DTO version.
pub const PASSPORT_PUBLIC_MANIFEST_VERSION: u16 = 1;

/// Maximum canonical username length without leading `@`.
pub const MAX_USERNAME_BYTES: usize = 32;
/// Maximum handle length including leading `@`.
pub const MAX_HANDLE_BYTES: usize = 33;
/// Maximum display name length.
pub const MAX_DISPLAY_NAME_BYTES: usize = 96;
/// Maximum bio/about field length.
pub const MAX_PROFILE_BIO_BYTES: usize = 1_024;
/// Maximum generic public URL/reference/account string length.
pub const MAX_IDENTITY_REF_BYTES: usize = 512;
/// Maximum public assets/sites/proofs on one profile DTO.
pub const MAX_PUBLIC_PROFILE_REFS: usize = 128;
/// Maximum warning string length.
pub const MAX_IDENTITY_WARNING_BYTES: usize = 512;

/// Main or pseudonymous passport flavor.
///
/// This is display/contract metadata only. Key custody and authority live elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PassportKindV1 {
    /// Main identity passport.
    Main,
    /// Pseudonymous alt passport.
    Alt,
}

/// Backend truth state for a username/handle claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UsernameStatusV1 {
    /// Local draft only; not backend truth.
    LocalDraft,
    /// Requested but not confirmed.
    Requested,
    /// Backend-confirmed/reserved.
    Confirmed,
    /// Backend rejected the requested claim.
    Rejected,
    /// Backend says this username is unavailable.
    Unavailable,
    /// Backend source is unknown or legacy.
    BackendUnknown,
}

/// Public attribution confidence state.
///
/// This is descriptive only. Verification, policy, and disputes are service-owned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AttributionStatus {
    /// Not verified.
    Unverified,
    /// Self-declared by the publisher.
    SelfDeclared,
    /// Confirmed by the backend/source service.
    BackendConfirmed,
    /// Disputed or under review.
    Disputed,
    /// Revoked or no longer valid.
    Revoked,
}

/// Site-scoped public moderation role.
///
/// Possessing a passport does not grant global delete/ban/edit power.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SiteModeratorRoleV1 {
    /// Can view public content.
    Viewer,
    /// Can comment when the site allows it.
    Commenter,
    /// Can submit moderation signals.
    SignalModerator,
    /// Can help with queues.
    QueueModerator,
    /// Can moderate content in the site scope.
    ContentModerator,
    /// Trusted moderator in the site scope.
    TrustedModerator,
    /// Site admin moderator in the site scope.
    AdminModerator,
    /// Site owner role.
    SiteOwner,
}

/// Public site role assignment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SiteModeratorRoleAssignmentV1 {
    /// Public site crab URL or stable site name pointer.
    pub site: String,
    /// Role granted for this site scope.
    pub role: SiteModeratorRoleV1,
    /// Status/source confidence for the role.
    pub status: AttributionStatus,
}

impl SiteModeratorRoleAssignmentV1 {
    /// Validate bounded public site role fields.
    pub fn validate(&self) -> Result<(), IdentityValidationError> {
        require_non_empty_bounded("site_roles[].site", &self.site, MAX_IDENTITY_REF_BYTES)
    }
}

/// Public reputation summary.
///
/// Score calculation is not owned by ron-proto.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReputationSummaryV1 {
    /// Optional global reputation score.
    #[serde(default)]
    pub global_reputation_score: Option<u32>,
    /// Source/service label for the summary.
    pub source: String,
    /// Optional update timestamp in milliseconds since Unix epoch.
    #[serde(default)]
    pub updated_at_ms: Option<u64>,
    /// Non-fatal public warnings.
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl ReputationSummaryV1 {
    /// Validate bounded public reputation fields.
    pub fn validate(&self) -> Result<(), IdentityValidationError> {
        require_non_empty_bounded("reputation.source", &self.source, MAX_IDENTITY_REF_BYTES)?;
        validate_warning_vec("reputation.warnings[]", &self.warnings)
    }
}

/// Public moderation summary.
///
/// Enforcement is not owned by ron-proto.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModeratorSummaryV1 {
    /// Optional global moderator score.
    #[serde(default)]
    pub global_moderator_score: Option<u32>,
    /// Public site-scoped roles.
    #[serde(default)]
    pub site_roles: Vec<SiteModeratorRoleAssignmentV1>,
    /// Source/service label for the summary.
    pub source: String,
    /// Optional update timestamp in milliseconds since Unix epoch.
    #[serde(default)]
    pub updated_at_ms: Option<u64>,
    /// Non-fatal public warnings.
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl ModeratorSummaryV1 {
    /// Validate bounded public moderation fields.
    pub fn validate(&self) -> Result<(), IdentityValidationError> {
        require_non_empty_bounded("moderation.source", &self.source, MAX_IDENTITY_REF_BYTES)?;

        if self.site_roles.len() > MAX_PUBLIC_PROFILE_REFS {
            return Err(IdentityValidationError::TooManyRefs {
                field: "moderation.site_roles",
                max: MAX_PUBLIC_PROFILE_REFS,
                actual: self.site_roles.len(),
            });
        }

        for role in &self.site_roles {
            role.validate()?;
        }

        validate_warning_vec("moderation.warnings[]", &self.warnings)
    }
}

/// Public asset reference used by profiles and public catalogues.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PublicAssetReferenceV1 {
    /// Optional asset CID when known.
    #[serde(default)]
    pub asset_cid: Option<ContentId>,
    /// Asset kind.
    pub asset_kind: AssetKind,
    /// Public crab URL.
    pub crab_url: String,
    /// Optional display title.
    #[serde(default)]
    pub title: Option<String>,
}

impl PublicAssetReferenceV1 {
    /// Validate bounded public asset reference fields.
    pub fn validate(&self) -> Result<(), IdentityValidationError> {
        validate_crab_url("public_assets[].crab_url", &self.crab_url)?;
        validate_optional_bounded(
            "public_assets[].title",
            self.title.as_deref(),
            MAX_DISPLAY_NAME_BYTES,
        )
    }
}

/// Public profile/passport display DTO.
///
/// This is safe for gateway/CrabLink hydration. It does not contain private keys,
/// private alt mappings, spend authority, recovery metadata, or wallet balances.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PassportPublicProfileV1 {
    /// DTO version. Must be `1`.
    pub version: u16,
    /// Passport subject, e.g. `passport:main:alice`.
    pub passport_subject: String,
    /// Passport flavor.
    pub passport_kind: PassportKindV1,
    /// Canonical username without leading `@`.
    pub username: String,
    /// Canonical handle with leading `@`.
    pub handle: String,
    /// Backend truth state for the username.
    pub username_status: UsernameStatusV1,
    /// Public display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Public bio/about text.
    #[serde(default)]
    pub bio: Option<String>,
    /// Optional avatar image crab URL.
    #[serde(default)]
    pub avatar_image: Option<String>,
    /// Optional CID of this public profile manifest/page.
    #[serde(default)]
    pub public_profile_cid: Option<ContentId>,
    /// Optional public payout account/pointer if intentionally published.
    #[serde(default)]
    pub public_payout_account: Option<String>,
    /// Public site references.
    #[serde(default)]
    pub public_sites: Vec<String>,
    /// Public asset references.
    #[serde(default)]
    pub public_assets: Vec<PublicAssetReferenceV1>,
    /// Optional public reputation summary.
    #[serde(default)]
    pub reputation_summary: Option<ReputationSummaryV1>,
    /// Optional public moderator summary.
    #[serde(default)]
    pub moderator_summary: Option<ModeratorSummaryV1>,
    /// Public attribution status.
    pub attribution_status: AttributionStatus,
    /// Non-fatal public warnings.
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl PassportPublicProfileV1 {
    /// Validate public profile shape.
    pub fn validate(&self) -> Result<(), IdentityValidationError> {
        if self.version != PASSPORT_PUBLIC_PROFILE_VERSION {
            return Err(IdentityValidationError::InvalidVersion {
                ty: "PassportPublicProfileV1",
                expected: PASSPORT_PUBLIC_PROFILE_VERSION,
                actual: self.version,
            });
        }

        require_non_empty_bounded(
            "passport_subject",
            &self.passport_subject,
            MAX_IDENTITY_REF_BYTES,
        )?;
        validate_username("username", &self.username)?;
        validate_handle_pair("handle", &self.handle, &self.username)?;

        validate_optional_bounded(
            "display_name",
            self.display_name.as_deref(),
            MAX_DISPLAY_NAME_BYTES,
        )?;
        validate_optional_bounded("bio", self.bio.as_deref(), MAX_PROFILE_BIO_BYTES)?;
        validate_optional_crab_url("avatar_image", self.avatar_image.as_deref())?;
        validate_optional_bounded(
            "public_payout_account",
            self.public_payout_account.as_deref(),
            MAX_IDENTITY_REF_BYTES,
        )?;

        validate_public_sites(&self.public_sites)?;
        validate_public_assets(&self.public_assets)?;

        if let Some(reputation) = &self.reputation_summary {
            reputation.validate()?;
        }

        if let Some(moderation) = &self.moderator_summary {
            moderation.validate()?;
        }

        validate_warning_vec("warnings[]", &self.warnings)
    }

    /// Canonical profile crab URL.
    #[must_use]
    pub fn canonical_profile_crab_url(&self) -> String {
        format!("crab://@{}", self.username)
    }
}

/// Public passport/profile manifest root.
///
/// This is an optional b3-addressable public object. It is not the private
/// passport record and not wallet/spend authority.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PassportPublicManifestV1 {
    /// DTO version. Must be `1`.
    pub version: u16,
    /// CID of this public manifest, if already stored.
    #[serde(default)]
    pub manifest_cid: Option<ContentId>,
    /// Public profile payload.
    pub profile: PassportPublicProfileV1,
    /// Public proof references only; never private keys or bearer capabilities.
    #[serde(default)]
    pub public_proof_refs: Vec<String>,
    /// Optional public asset catalogue CID.
    #[serde(default)]
    pub public_asset_catalogue_cid: Option<ContentId>,
    /// Creation timestamp in milliseconds since Unix epoch.
    pub created_at_ms: u64,
    /// Last update timestamp in milliseconds since Unix epoch.
    pub updated_at_ms: u64,
    /// Non-fatal public warnings.
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl PassportPublicManifestV1 {
    /// Validate public passport manifest shape.
    pub fn validate(&self) -> Result<(), IdentityValidationError> {
        if self.version != PASSPORT_PUBLIC_MANIFEST_VERSION {
            return Err(IdentityValidationError::InvalidVersion {
                ty: "PassportPublicManifestV1",
                expected: PASSPORT_PUBLIC_MANIFEST_VERSION,
                actual: self.version,
            });
        }

        self.profile.validate()?;

        if self.created_at_ms == 0 {
            return Err(IdentityValidationError::EmptyField {
                field: "created_at_ms",
            });
        }

        if self.updated_at_ms < self.created_at_ms {
            return Err(IdentityValidationError::InvalidTimeOrder {
                field: "updated_at_ms",
            });
        }

        if let (Some(manifest_cid), Some(profile_cid)) =
            (&self.manifest_cid, &self.profile.public_profile_cid)
        {
            if manifest_cid != profile_cid {
                return Err(IdentityValidationError::CidMismatch {
                    field: "profile.public_profile_cid",
                });
            }
        }

        validate_ref_vec("public_proof_refs[]", &self.public_proof_refs)?;
        validate_warning_vec("warnings[]", &self.warnings)
    }
}

/// Deterministic validation errors for public identity DTOs.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdentityValidationError {
    /// Version field did not match the DTO version.
    #[error("invalid {ty} version: expected {expected}, got {actual}")]
    InvalidVersion {
        /// DTO type name.
        ty: &'static str,
        /// Expected version.
        expected: u16,
        /// Actual version.
        actual: u16,
    },
    /// Required field was empty or all whitespace.
    #[error("empty field: {field}")]
    EmptyField {
        /// Field name.
        field: &'static str,
    },
    /// Bounded field exceeded max length.
    #[error("field too long: {field} max={max} actual={actual}")]
    FieldTooLong {
        /// Field name.
        field: &'static str,
        /// Max bytes.
        max: usize,
        /// Actual bytes.
        actual: usize,
    },
    /// Too many references were provided.
    #[error("too many refs: {field} max={max} actual={actual}")]
    TooManyRefs {
        /// Field name.
        field: &'static str,
        /// Max refs.
        max: usize,
        /// Actual refs.
        actual: usize,
    },
    /// Username syntax failed the DTO-level shape check.
    #[error("invalid username: {field}")]
    InvalidUsername {
        /// Field name.
        field: &'static str,
    },
    /// Handle syntax failed the DTO-level shape check.
    #[error("invalid handle: {field}")]
    InvalidHandle {
        /// Field name.
        field: &'static str,
    },
    /// Handle and username did not match.
    #[error("handle does not match username")]
    HandleMismatch,
    /// Public crab URL did not start with `crab://`.
    #[error("invalid crab url: {field}")]
    InvalidCrabUrl {
        /// Field name.
        field: &'static str,
    },
    /// CID fields contradicted each other.
    #[error("cid mismatch: {field}")]
    CidMismatch {
        /// Field name.
        field: &'static str,
    },
    /// Timestamp ordering was invalid.
    #[error("invalid time order: {field}")]
    InvalidTimeOrder {
        /// Field/relation name.
        field: &'static str,
    },
}

fn require_non_empty_bounded(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), IdentityValidationError> {
    if value.trim().is_empty() {
        return Err(IdentityValidationError::EmptyField { field });
    }
    validate_bounded(field, value, max)
}

fn validate_optional_bounded(
    field: &'static str,
    value: Option<&str>,
    max: usize,
) -> Result<(), IdentityValidationError> {
    if let Some(value) = value {
        validate_bounded(field, value, max)?;
    }
    Ok(())
}

fn validate_bounded(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), IdentityValidationError> {
    let actual = value.len();
    if actual > max {
        return Err(IdentityValidationError::FieldTooLong { field, max, actual });
    }
    Ok(())
}

fn validate_username(field: &'static str, username: &str) -> Result<(), IdentityValidationError> {
    require_non_empty_bounded(field, username, MAX_USERNAME_BYTES)?;

    let bytes = username.as_bytes();
    if bytes.len() < 3 {
        return Err(IdentityValidationError::InvalidUsername { field });
    }

    if !bytes[0].is_ascii_alphanumeric() {
        return Err(IdentityValidationError::InvalidUsername { field });
    }

    if matches!(bytes[bytes.len() - 1], b'.' | b'-' | b'_') {
        return Err(IdentityValidationError::InvalidUsername { field });
    }

    let mut previous_dot = false;
    for byte in bytes {
        let valid = byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(*byte, b'_' | b'-' | b'.');

        if !valid {
            return Err(IdentityValidationError::InvalidUsername { field });
        }

        if previous_dot && *byte == b'.' {
            return Err(IdentityValidationError::InvalidUsername { field });
        }
        previous_dot = *byte == b'.';
    }

    Ok(())
}

fn validate_handle_pair(
    field: &'static str,
    handle: &str,
    username: &str,
) -> Result<(), IdentityValidationError> {
    require_non_empty_bounded(field, handle, MAX_HANDLE_BYTES)?;

    let Some(stripped) = handle.strip_prefix('@') else {
        return Err(IdentityValidationError::InvalidHandle { field });
    };

    validate_username(field, stripped)?;

    if stripped != username {
        return Err(IdentityValidationError::HandleMismatch);
    }

    Ok(())
}

fn validate_optional_crab_url(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), IdentityValidationError> {
    if let Some(value) = value {
        validate_crab_url(field, value)?;
    }
    Ok(())
}

fn validate_crab_url(field: &'static str, value: &str) -> Result<(), IdentityValidationError> {
    require_non_empty_bounded(field, value, MAX_IDENTITY_REF_BYTES)?;
    if !value.starts_with("crab://") {
        return Err(IdentityValidationError::InvalidCrabUrl { field });
    }
    Ok(())
}

fn validate_public_sites(sites: &[String]) -> Result<(), IdentityValidationError> {
    if sites.len() > MAX_PUBLIC_PROFILE_REFS {
        return Err(IdentityValidationError::TooManyRefs {
            field: "public_sites",
            max: MAX_PUBLIC_PROFILE_REFS,
            actual: sites.len(),
        });
    }

    for site in sites {
        validate_crab_url("public_sites[]", site)?;
    }

    Ok(())
}

fn validate_public_assets(
    assets: &[PublicAssetReferenceV1],
) -> Result<(), IdentityValidationError> {
    if assets.len() > MAX_PUBLIC_PROFILE_REFS {
        return Err(IdentityValidationError::TooManyRefs {
            field: "public_assets",
            max: MAX_PUBLIC_PROFILE_REFS,
            actual: assets.len(),
        });
    }

    for asset in assets {
        asset.validate()?;
    }

    Ok(())
}

fn validate_ref_vec(field: &'static str, refs: &[String]) -> Result<(), IdentityValidationError> {
    if refs.len() > MAX_PUBLIC_PROFILE_REFS {
        return Err(IdentityValidationError::TooManyRefs {
            field,
            max: MAX_PUBLIC_PROFILE_REFS,
            actual: refs.len(),
        });
    }

    for value in refs {
        require_non_empty_bounded(field, value, MAX_IDENTITY_REF_BYTES)?;
    }

    Ok(())
}

fn validate_warning_vec(
    field: &'static str,
    warnings: &[String],
) -> Result<(), IdentityValidationError> {
    if warnings.len() > MAX_PUBLIC_PROFILE_REFS {
        return Err(IdentityValidationError::TooManyRefs {
            field,
            max: MAX_PUBLIC_PROFILE_REFS,
            actual: warnings.len(),
        });
    }

    for warning in warnings {
        require_non_empty_bounded(field, warning, MAX_IDENTITY_WARNING_BYTES)?;
    }

    Ok(())
}
