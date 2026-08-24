//! RO:WHAT — Main-passport username/profile claim authority with optional durable CN-4 snapshot backing.
//! RO:WHY — Preserve svc-passport claim semantics while making CrabNode username/profile truth restart-safe.
//! RO:INTERACTS — profile_persistence, HTTP profile routes, Omnigate hydration, CrabLink Passport UX.
//! RO:INVARIANTS — one subject→one main username; one username→one subject; same-subject retry is idempotent; disk commit precedes memory commit.
//! RO:METRICS — none yet; route layer owns HTTP operation/failure metrics.
//! RO:CONFIG — durable callers supply a service-owned profile state directory.
//! RO:SECURITY — no wallet/spend/private-key authority; corrupt durable state fails closed instead of resetting claims.
//! RO:TEST — profile_claims.rs and crabnode_cn4_durable_profile_store.rs.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::Path,
    sync::RwLock,
};
use thiserror::Error;

use crate::profile_persistence::{
    LoadedProfileSnapshot, ProfileSnapshotError, ProfileSnapshotPersistence,
};

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

/// Username/profile claim authority owned by svc-passport.
///
/// `new()` preserves the historical in-memory mode for focused tests and
/// explicitly ephemeral service use. `open_durable()` adds a restart-safe
/// backing store without changing claim semantics or creating another owner.
#[derive(Debug)]
pub struct UsernameClaimStore {
    inner: RwLock<UsernameClaimStoreInner>,
    persistence: Option<ProfileSnapshotPersistence>,
}

#[derive(Debug, Clone, Default)]
struct UsernameClaimStoreInner {
    generation: u64,
    by_username: BTreeMap<String, UsernameClaimRecord>,
    by_passport_subject: BTreeMap<String, String>,
}

impl Default for UsernameClaimStore {
    fn default() -> Self {
        Self {
            inner: RwLock::new(UsernameClaimStoreInner::default()),
            persistence: None,
        }
    }
}

impl UsernameClaimStore {
    /// Create an empty in-memory claim store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Open or create a durable username/profile claim store.
    ///
    /// Existing snapshots are validated and both indexes are rebuilt from the
    /// canonical persisted claim records. Any unsupported/corrupt state fails
    /// closed; it is never silently replaced with an empty store.
    pub fn open_durable(root: impl AsRef<Path>) -> Result<Self, ProfileClaimError> {
        let (persistence, loaded) =
            ProfileSnapshotPersistence::open(root).map_err(profile_snapshot_error)?;

        let inner = inner_from_loaded_snapshot(loaded)?;

        Ok(Self {
            inner: RwLock::new(inner),
            persistence: Some(persistence),
        })
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

        let mut candidate = inner.clone();

        candidate
            .by_passport_subject
            .insert(passport_subject, username.clone());

        candidate.by_username.insert(username, record.clone());

        if let Some(persistence) = &self.persistence {
            let next_generation =
                inner
                    .generation
                    .checked_add(1)
                    .ok_or(ProfileClaimError::StoreUnavailable {
                        operation: "advance profile snapshot generation",
                    })?;

            let expected_claims = inner.by_username.values().cloned().collect::<Vec<_>>();

            let next_claims = candidate.by_username.values().cloned().collect::<Vec<_>>();

            persistence
                .persist(
                    inner.generation,
                    &expected_claims,
                    next_generation,
                    &next_claims,
                )
                .map_err(profile_snapshot_error)?;

            candidate.generation = next_generation;
        }

        *inner = candidate;

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

    /// Resolve a confirmed public profile from an existing Passport-subject claim.
    ///
    /// This reuses the reverse claim index already owned by svc-passport.
    /// It does not derive a username from the Passport subject and does not
    /// accept client-supplied username headers as identity authority.
    pub fn public_profile_for_passport_subject(
        &self,
        passport_subject: &str,
    ) -> Result<Option<PublicProfileResponse>, ProfileClaimError> {
        let passport_subject = normalize_passport_subject(passport_subject)?;

        let inner = self
            .inner
            .read()
            .map_err(|_| ProfileClaimError::StorePoisoned)?;

        let Some(username) = inner.by_passport_subject.get(&passport_subject) else {
            return Ok(None);
        };

        let record = inner
            .by_username
            .get(username)
            .ok_or(ProfileClaimError::StoreCorrupt {
                reason: "passport index points to missing username",
            })?;

        if record.passport_subject != passport_subject {
            return Err(ProfileClaimError::StoreCorrupt {
                reason: "passport and username indexes disagree",
            });
        }

        Ok(Some(PublicProfileResponse::from(record)))
    }
}

fn profile_snapshot_error(error: ProfileSnapshotError) -> ProfileClaimError {
    match error {
        ProfileSnapshotError::Unavailable(operation) => {
            ProfileClaimError::StoreUnavailable { operation }
        }

        ProfileSnapshotError::Corrupt(reason) => ProfileClaimError::StoreCorrupt { reason },
    }
}

fn inner_from_loaded_snapshot(
    loaded: LoadedProfileSnapshot,
) -> Result<UsernameClaimStoreInner, ProfileClaimError> {
    let mut inner = UsernameClaimStoreInner {
        generation: loaded.generation,
        ..UsernameClaimStoreInner::default()
    };

    let mut previous_username: Option<String> = None;

    for record in loaded.claims {
        validate_persisted_claim_record(&record)?;

        if let Some(previous) = &previous_username {
            if previous >= &record.username {
                return Err(ProfileClaimError::StoreCorrupt {
                    reason: "persisted claims are not in canonical username order",
                });
            }
        }

        previous_username = Some(record.username.clone());

        if inner
            .by_passport_subject
            .insert(record.passport_subject.clone(), record.username.clone())
            .is_some()
        {
            return Err(ProfileClaimError::StoreCorrupt {
                reason: "multiple usernames claim the same passport subject",
            });
        }

        if inner
            .by_username
            .insert(record.username.clone(), record)
            .is_some()
        {
            return Err(ProfileClaimError::StoreCorrupt {
                reason: "duplicate username exists in durable profile state",
            });
        }
    }

    Ok(inner)
}

fn validate_persisted_claim_record(record: &UsernameClaimRecord) -> Result<(), ProfileClaimError> {
    if record.passport_kind != PassportKind::Main {
        return Err(ProfileClaimError::StoreCorrupt {
            reason: "durable main-username claim has non-main passport kind",
        });
    }

    if record.username_status != UsernameClaimStatus::Confirmed {
        return Err(ProfileClaimError::StoreCorrupt {
            reason: "durable username claim is not confirmed",
        });
    }

    let passport_subject = normalize_passport_subject(&record.passport_subject).map_err(|_| {
        ProfileClaimError::StoreCorrupt {
            reason: "durable passport subject is invalid",
        }
    })?;

    if passport_subject != record.passport_subject {
        return Err(ProfileClaimError::StoreCorrupt {
            reason: "durable passport subject is not canonical",
        });
    }

    let username =
        normalize_username(&record.username).map_err(|_| ProfileClaimError::StoreCorrupt {
            reason: "durable username is invalid",
        })?;

    if username != record.username {
        return Err(ProfileClaimError::StoreCorrupt {
            reason: "durable username is not canonical",
        });
    }

    if record.handle != format!("@{}", record.username,) {
        return Err(ProfileClaimError::StoreCorrupt {
            reason: "durable username handle disagrees with username",
        });
    }

    if record.profile_crab_url != format!("crab://@{}", record.username,) {
        return Err(ProfileClaimError::StoreCorrupt {
            reason: "durable profile URL disagrees with username",
        });
    }

    validate_persisted_optional_text(
        record.display_name.as_deref(),
        DISPLAY_NAME_MAX_BYTES,
        "durable display name is invalid",
    )?;

    validate_persisted_optional_text(
        record.bio.as_deref(),
        PROFILE_BIO_MAX_BYTES,
        "durable profile bio is invalid",
    )?;

    if let Some(avatar) = record.avatar_image.as_deref() {
        if avatar.trim() != avatar
            || validate_optional_crab_url("avatar_image", Some(avatar)).is_err()
        {
            return Err(ProfileClaimError::StoreCorrupt {
                reason: "durable avatar URL is invalid",
            });
        }
    }

    if let Some(cid) = record.public_profile_cid.as_deref() {
        if !is_canonical_b3_cid(cid) {
            return Err(ProfileClaimError::StoreCorrupt {
                reason: "durable public profile CID is invalid",
            });
        }
    }

    if record.created_at_ms == 0 || record.updated_at_ms < record.created_at_ms {
        return Err(ProfileClaimError::StoreCorrupt {
            reason: "durable profile timestamps are invalid",
        });
    }

    Ok(())
}

fn validate_persisted_optional_text(
    value: Option<&str>,
    max_bytes: usize,
    corrupt_reason: &'static str,
) -> Result<(), ProfileClaimError> {
    if let Some(value) = value {
        if value.trim() != value || value.is_empty() || value.len() > max_bytes {
            return Err(ProfileClaimError::StoreCorrupt {
                reason: corrupt_reason,
            });
        }
    }

    Ok(())
}

fn is_canonical_b3_cid(value: &str) -> bool {
    let Some(hash) = value.strip_prefix("b3:") else {
        return false;
    };

    hash.len() == 64
        && hash.bytes().all(|byte| {
            matches!(
                byte,
                b'0'..=b'9'
                    | b'a'..=b'f'
            )
        })
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
    /// Durable store could not complete an operation.
    #[error("username claim store unavailable during {operation}")]
    StoreUnavailable {
        /// Stable internal operation label; public HTTP responses remain generic.
        operation: &'static str,
    },
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
            Self::StoreUnavailable { .. } => "store_unavailable",
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

    if username == "ledger" {
        return Err(ProfileClaimError::ReservedUsername {
            username: username.to_owned(),
        });
    }
    if username.contains("__") {
        return Err(ProfileClaimError::InvalidUsernameCharacter);
    }
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
