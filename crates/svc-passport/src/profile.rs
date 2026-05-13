//! RO:WHAT — In-memory main-passport username/profile claim core for NEXT_LEVEL Phase 3.
//! RO:WHY — P3 Identity & Keys; Concerns: SEC/GOV/DX. Proves deterministic username claims before HTTP exposure.
//! RO:INTERACTS — future svc-passport HTTP profile routes, omnigate profile hydration, CrabLink first-passport UX.
//! RO:INVARIANTS — no wallet mutation; no spend authority; no private keys; no public main↔alt linkage.
//! RO:METRICS — none yet; route layer should increment passport ops/failures when exposed.
//! RO:CONFIG — none.
//! RO:SECURITY — username uniqueness is local to this store; production persistence must be durable and audited.
//! RO:TEST — tests/profile_claims.rs.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::RwLock,
};
use thiserror::Error;

/// Current public profile schema string for service-local read responses.
pub const PUBLIC_PROFILE_SCHEMA: &str = "svc-passport.public-profile.v1";

/// Minimum username length in bytes.
pub const USERNAME_MIN_BYTES: usize = 3;

/// Maximum username length in bytes.
pub const USERNAME_MAX_BYTES: usize = 32;

/// Maximum passport subject length in bytes.
pub const PASSPORT_SUBJECT_MAX_BYTES: usize = 256;

/// Maximum display name length in bytes.
pub const DISPLAY_NAME_MAX_BYTES: usize = 96;

/// Maximum profile bio length in bytes.
pub const PROFILE_BIO_MAX_BYTES: usize = 1024;

/// Maximum crab URL / public pointer length in bytes.
pub const PUBLIC_REF_MAX_BYTES: usize = 512;

/// Backend truth state for a username claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UsernameClaimStatus {
    /// Backend-confirmed/reserved.
    Confirmed,
    /// Backend rejected the requested claim.
    Rejected,
    /// Backend says the username is unavailable.
    Unavailable,
}

impl UsernameClaimStatus {
    /// Stable lowercase string for wire/UI display.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Rejected => "rejected",
            Self::Unavailable => "unavailable",
        }
    }
}

impl fmt::Display for UsernameClaimStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Main or pseudonymous passport kind.
///
/// Phase 3 only claims usernames for main passports. Alt support comes later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PassportKind {
    /// Main identity passport.
    Main,
    /// Alt identity passport.
    Alt,
}

/// Request to claim/reserve a main-passport username.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsernameClaimRequest {
    /// Canonical passport subject that will own this username.
    pub passport_subject: String,
    /// Requested username or handle. `@` prefix is allowed and normalized away.
    pub requested_username: String,
    /// Optional display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Optional public bio.
    #[serde(default)]
    pub bio: Option<String>,
    /// Optional public avatar crab URL.
    #[serde(default)]
    pub avatar_image: Option<String>,
}

/// Confirmed username claim record.
///
/// This is service-local identity/read-model data. It is not a private passport
/// key, not a wallet, and not spend authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsernameClaimRecord {
    /// Passport subject that owns this username.
    pub passport_subject: String,
    /// Passport kind.
    pub passport_kind: PassportKind,
    /// Canonical username without leading `@`.
    pub username: String,
    /// Canonical handle with leading `@`.
    pub handle: String,
    /// Claim status.
    pub username_status: UsernameClaimStatus,
    /// Optional public display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Optional public bio.
    #[serde(default)]
    pub bio: Option<String>,
    /// Optional avatar image crab URL.
    #[serde(default)]
    pub avatar_image: Option<String>,
    /// Public profile route.
    pub profile_crab_url: String,
    /// Optional public profile CID once published as a b3 object.
    #[serde(default)]
    pub public_profile_cid: Option<String>,
    /// Creation timestamp in milliseconds since Unix epoch.
    pub created_at_ms: u64,
    /// Last update timestamp in milliseconds since Unix epoch.
    pub updated_at_ms: u64,
}

/// Read-only public profile response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicProfileResponse {
    /// Stable schema string.
    pub schema: &'static str,
    /// Passport subject that owns this profile.
    pub passport_subject: String,
    /// Passport kind.
    pub passport_kind: PassportKind,
    /// Canonical username without leading `@`.
    pub username: String,
    /// Canonical handle with leading `@`.
    pub handle: String,
    /// Username claim status.
    pub username_status: UsernameClaimStatus,
    /// Optional public display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Optional public bio.
    #[serde(default)]
    pub bio: Option<String>,
    /// Optional avatar image crab URL.
    #[serde(default)]
    pub avatar_image: Option<String>,
    /// Profile crab URL.
    pub profile_crab_url: String,
    /// Optional profile CID once public manifest publication exists.
    #[serde(default)]
    pub public_profile_cid: Option<String>,
    /// Reputation score placeholder. Remains `None` until backend computes real reputation.
    #[serde(default)]
    pub reputation_score: Option<u32>,
    /// Moderator score placeholder. Remains `None` until backend computes real moderation score.
    #[serde(default)]
    pub moderator_score: Option<u32>,
    /// Public warnings.
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl From<&UsernameClaimRecord> for PublicProfileResponse {
    fn from(record: &UsernameClaimRecord) -> Self {
        Self {
            schema: PUBLIC_PROFILE_SCHEMA,
            passport_subject: record.passport_subject.clone(),
            passport_kind: record.passport_kind,
            username: record.username.clone(),
            handle: record.handle.clone(),
            username_status: record.username_status,
            display_name: record.display_name.clone(),
            bio: record.bio.clone(),
            avatar_image: record.avatar_image.clone(),
            profile_crab_url: record.profile_crab_url.clone(),
            public_profile_cid: record.public_profile_cid.clone(),
            reputation_score: None,
            moderator_score: None,
            warnings: vec![
                "public profile is read-only".to_owned(),
                "reputation and moderation scores are not computed yet".to_owned(),
            ],
        }
    }
}

/// In-memory username claim store for Phase 3 tests/dev.
///
/// This is deliberately not the final production store. It proves validation,
/// duplicate handling, and safe public response shape before HTTP/storage wiring.
#[derive(Debug, Default)]
pub struct UsernameClaimStore {
    inner: RwLock<UsernameClaimStoreInner>,
}

#[derive(Debug, Default)]
struct UsernameClaimStoreInner {
    by_username: BTreeMap<String, UsernameClaimRecord>,
    by_passport_subject: BTreeMap<String, String>,
}

impl UsernameClaimStore {
    /// Create an empty in-memory claim store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Claim a main-passport username.
    ///
    /// Duplicate username claims fail deterministically unless they are an
    /// idempotent repeat for the same passport subject.
    pub fn claim_main_username(
        &self,
        request: UsernameClaimRequest,
        now_ms: u64,
    ) -> Result<UsernameClaimRecord, ProfileClaimError> {
        let passport_subject = normalize_passport_subject(&request.passport_subject)?;
        let username = normalize_username(&request.requested_username)?;
        let handle = format!("@{username}");

        validate_optional_public_text(
            "display_name",
            request.display_name.as_deref(),
            DISPLAY_NAME_MAX_BYTES,
        )?;
        validate_optional_public_text("bio", request.bio.as_deref(), PROFILE_BIO_MAX_BYTES)?;
        validate_optional_crab_url("avatar_image", request.avatar_image.as_deref())?;

        if now_ms == 0 {
            return Err(ProfileClaimError::InvalidTimestamp {
                field: "created_at_ms",
            });
        }

        let mut inner = self
            .inner
            .write()
            .map_err(|_| ProfileClaimError::StorePoisoned)?;

        if let Some(existing_owner) = inner.by_passport_subject.get(&passport_subject) {
            if existing_owner != &username {
                return Err(ProfileClaimError::PassportAlreadyHasUsername {
                    passport_subject,
                    username: existing_owner.clone(),
                });
            }

            let existing = inner.by_username.get(&username).cloned().ok_or(
                ProfileClaimError::StoreCorrupt {
                    reason: "passport index points to missing username",
                },
            )?;

            return Ok(existing);
        }

        if let Some(existing) = inner.by_username.get(&username) {
            return Err(ProfileClaimError::UsernameUnavailable {
                username: existing.username.clone(),
            });
        }

        let record = UsernameClaimRecord {
            passport_subject: passport_subject.clone(),
            passport_kind: PassportKind::Main,
            username: username.clone(),
            handle,
            username_status: UsernameClaimStatus::Confirmed,
            display_name: request.display_name.map(|value| value.trim().to_owned()),
            bio: request.bio.map(|value| value.trim().to_owned()),
            avatar_image: request.avatar_image.map(|value| value.trim().to_owned()),
            profile_crab_url: format!("crab://@{username}"),
            public_profile_cid: None,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        };

        inner
            .by_passport_subject
            .insert(passport_subject, username.clone());
        inner.by_username.insert(username, record.clone());

        Ok(record)
    }

    /// Get a claim by username or handle.
    pub fn get_by_username_or_handle(
        &self,
        username_or_handle: &str,
    ) -> Result<Option<UsernameClaimRecord>, ProfileClaimError> {
        let username = normalize_username(username_or_handle)?;
        let inner = self
            .inner
            .read()
            .map_err(|_| ProfileClaimError::StorePoisoned)?;

        Ok(inner.by_username.get(&username).cloned())
    }

    /// Build a read-only public profile response by username or handle.
    pub fn public_profile(
        &self,
        username_or_handle: &str,
    ) -> Result<Option<PublicProfileResponse>, ProfileClaimError> {
        Ok(self
            .get_by_username_or_handle(username_or_handle)?
            .as_ref()
            .map(PublicProfileResponse::from))
    }
}

/// Deterministic errors for Phase 3 username/profile claims.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProfileClaimError {
    /// Required field was empty.
    #[error("empty field: {field}")]
    EmptyField {
        /// Field name.
        field: &'static str,
    },
    /// Field exceeded byte limit.
    #[error("field too long: {field} max={max} actual={actual}")]
    FieldTooLong {
        /// Field name.
        field: &'static str,
        /// Max bytes.
        max: usize,
        /// Actual bytes.
        actual: usize,
    },
    /// Username was too short.
    #[error("username too short: min={min} actual={actual}")]
    UsernameTooShort {
        /// Minimum bytes.
        min: usize,
        /// Actual bytes.
        actual: usize,
    },
    /// Username was too long.
    #[error("username too long: max={max} actual={actual}")]
    UsernameTooLong {
        /// Maximum bytes.
        max: usize,
        /// Actual bytes.
        actual: usize,
    },
    /// Username start character failed validation.
    #[error("username must start with ASCII letter or digit")]
    InvalidUsernameStart,
    /// Username contained unsupported character.
    #[error("username contains invalid character")]
    InvalidUsernameCharacter,
    /// Username contained adjacent dots.
    #[error("username contains consecutive dots")]
    ConsecutiveDots,
    /// Username ended with punctuation.
    #[error("username has invalid trailing punctuation")]
    InvalidUsernameTrailingPunctuation,
    /// Username is reserved.
    #[error("username is reserved: {username}")]
    ReservedUsername {
        /// Reserved username.
        username: String,
    },
    /// Username already claimed by another passport.
    #[error("username unavailable: {username}")]
    UsernameUnavailable {
        /// Unavailable username.
        username: String,
    },
    /// Passport subject already owns a different username.
    #[error("passport already has username: {passport_subject} -> {username}")]
    PassportAlreadyHasUsername {
        /// Passport subject.
        passport_subject: String,
        /// Existing username.
        username: String,
    },
    /// Invalid crab URL in a public field.
    #[error("invalid crab url: {field}")]
    InvalidCrabUrl {
        /// Field name.
        field: &'static str,
    },
    /// Invalid timestamp.
    #[error("invalid timestamp: {field}")]
    InvalidTimestamp {
        /// Field name.
        field: &'static str,
    },
    /// Internal store lock was poisoned.
    #[error("username claim store poisoned")]
    StorePoisoned,
    /// Internal index inconsistency.
    #[error("username claim store corrupt: {reason}")]
    StoreCorrupt {
        /// Reason.
        reason: &'static str,
    },
}

impl ProfileClaimError {
    /// Stable error code for route/problem mapping.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EmptyField { .. } => "empty_field",
            Self::FieldTooLong { .. } => "field_too_long",
            Self::UsernameTooShort { .. } => "username_too_short",
            Self::UsernameTooLong { .. } => "username_too_long",
            Self::InvalidUsernameStart => "invalid_username_start",
            Self::InvalidUsernameCharacter => "invalid_username_character",
            Self::ConsecutiveDots => "consecutive_dots",
            Self::InvalidUsernameTrailingPunctuation => "invalid_username_trailing_punctuation",
            Self::ReservedUsername { .. } => "reserved_username",
            Self::UsernameUnavailable { .. } => "username_unavailable",
            Self::PassportAlreadyHasUsername { .. } => "passport_already_has_username",
            Self::InvalidCrabUrl { .. } => "invalid_crab_url",
            Self::InvalidTimestamp { .. } => "invalid_timestamp",
            Self::StorePoisoned => "store_poisoned",
            Self::StoreCorrupt { .. } => "store_corrupt",
        }
    }
}

/// Normalize a requested username or handle to lowercase username without `@`.
pub fn normalize_username(input: &str) -> Result<String, ProfileClaimError> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err(ProfileClaimError::EmptyField { field: "username" });
    }

    let username = raw.strip_prefix('@').unwrap_or(raw).to_ascii_lowercase();
    validate_username(&username)?;
    Ok(username)
}

/// Normalize a username/handle to canonical handle with leading `@`.
pub fn normalize_handle(input: &str) -> Result<String, ProfileClaimError> {
    normalize_username(input).map(|username| format!("@{username}"))
}

fn validate_username(username: &str) -> Result<(), ProfileClaimError> {
    if username.is_empty() {
        return Err(ProfileClaimError::EmptyField { field: "username" });
    }

    if username.len() < USERNAME_MIN_BYTES {
        return Err(ProfileClaimError::UsernameTooShort {
            min: USERNAME_MIN_BYTES,
            actual: username.len(),
        });
    }

    if username.len() > USERNAME_MAX_BYTES {
        return Err(ProfileClaimError::UsernameTooLong {
            max: USERNAME_MAX_BYTES,
            actual: username.len(),
        });
    }

    let bytes = username.as_bytes();

    if !bytes[0].is_ascii_alphanumeric() {
        return Err(ProfileClaimError::InvalidUsernameStart);
    }

    if matches!(bytes[bytes.len() - 1], b'.' | b'-' | b'_') {
        return Err(ProfileClaimError::InvalidUsernameTrailingPunctuation);
    }

    let mut previous_dot = false;
    for byte in bytes {
        let valid = byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(*byte, b'_' | b'-' | b'.');

        if !valid {
            return Err(ProfileClaimError::InvalidUsernameCharacter);
        }

        if previous_dot && *byte == b'.' {
            return Err(ProfileClaimError::ConsecutiveDots);
        }
        previous_dot = *byte == b'.';
    }

    if reserved_usernames().contains(username) {
        return Err(ProfileClaimError::ReservedUsername {
            username: username.to_owned(),
        });
    }

    Ok(())
}

fn normalize_passport_subject(input: &str) -> Result<String, ProfileClaimError> {
    let value = input.trim();
    require_non_empty_bounded("passport_subject", value, PASSPORT_SUBJECT_MAX_BYTES)?;
    Ok(value.to_owned())
}

fn validate_optional_public_text(
    field: &'static str,
    value: Option<&str>,
    max: usize,
) -> Result<(), ProfileClaimError> {
    if let Some(value) = value {
        validate_bounded(field, value.trim(), max)?;
    }
    Ok(())
}

fn require_non_empty_bounded(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), ProfileClaimError> {
    if value.trim().is_empty() {
        return Err(ProfileClaimError::EmptyField { field });
    }

    validate_bounded(field, value, max)
}

fn validate_bounded(field: &'static str, value: &str, max: usize) -> Result<(), ProfileClaimError> {
    let actual = value.len();
    if actual > max {
        return Err(ProfileClaimError::FieldTooLong { field, max, actual });
    }

    Ok(())
}

fn validate_optional_crab_url(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), ProfileClaimError> {
    let Some(value) = value else {
        return Ok(());
    };

    let value = value.trim();
    require_non_empty_bounded(field, value, PUBLIC_REF_MAX_BYTES)?;

    if !value.starts_with("crab://") {
        return Err(ProfileClaimError::InvalidCrabUrl { field });
    }

    Ok(())
}

fn reserved_usernames() -> &'static BTreeSet<String> {
    static RESERVED: std::sync::OnceLock<BTreeSet<String>> = std::sync::OnceLock::new();

    RESERVED.get_or_init(|| {
        [
            "admin",
            "alt",
            "api",
            "app",
            "article",
            "asset",
            "assets",
            "b3",
            "comment",
            "crab",
            "gateway",
            "image",
            "mail",
            "manifest",
            "mod",
            "moderator",
            "music",
            "passport",
            "podcast",
            "post",
            "profile",
            "root",
            "ron",
            "site",
            "sites",
            "stream",
            "support",
            "sys",
            "system",
            "video",
            "wallet",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    })
}
