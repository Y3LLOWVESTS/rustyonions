//! RO:WHAT — Pure `@username` parser/normalizer for RON passport discovery handles.
//! RO:WHY — Pillar 9, Concerns: SEC/DX/GOV; make human identity pointers deterministic before persistence.
//! RO:INTERACTS — crab::CrabLink profile routes, future svc-passport username claims, CrabLink first-passport UX.
//! RO:INVARIANTS — no IO; no async; username is a human pointer, not canonical identity or wallet authority.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects reserved/system names and confusing punctuation fail-closed.
//! RO:TEST — tests/username.rs and tests/crab_links.rs profile-route cases.

use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// Minimum normalized username length in bytes.
pub const USERNAME_MIN_BYTES: usize = 3;
/// Maximum normalized username length in bytes.
pub const USERNAME_MAX_BYTES: usize = 32;

/// Canonical normalized RON username without the leading `@`.
///
/// `RonUsername` is a human/discovery pointer for a passport. It is not the
/// canonical identity key, not wallet authority, and not proof of ownership
/// until a service such as `svc-passport` persists and confirms the claim.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RonUsername(String);

impl RonUsername {
    /// Parse a username or handle and return canonical lowercase form.
    pub fn parse(input: &str) -> Result<Self, UsernameParseError> {
        let raw = input.trim();
        if raw.is_empty() {
            return Err(UsernameParseError::Empty);
        }

        let name = raw.strip_prefix('@').unwrap_or(raw).to_ascii_lowercase();
        validate_username(&name)?;
        Ok(Self(name))
    }

    /// Borrow the canonical username without leading `@`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Render the canonical handle with leading `@`.
    #[must_use]
    pub fn handle(&self) -> String {
        format!("@{}", self.0)
    }

    /// Render the canonical public identity/discovery URL.
    #[must_use]
    pub fn crab_url(&self) -> String {
        format!("crab://@{}", self.0)
    }

    /// Consume and return the inner canonical username string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for RonUsername {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for RonUsername {
    type Err = UsernameParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Normalize to the canonical username without leading `@`.
pub fn normalize_username(input: &str) -> Result<String, UsernameParseError> {
    RonUsername::parse(input).map(|username| username.into_string())
}

/// Normalize to the canonical handle with leading `@`.
pub fn normalize_handle(input: &str) -> Result<String, UsernameParseError> {
    RonUsername::parse(input).map(|username| username.handle())
}

/// Deterministic parser errors for RON usernames.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum UsernameParseError {
    /// Username was empty after trimming and removing optional `@`.
    #[error("empty username")]
    Empty,
    /// Username was shorter than the configured minimum.
    #[error("username too short: min={min} actual={actual}")]
    TooShort {
        /// Minimum accepted byte length.
        min: usize,
        /// Actual byte length.
        actual: usize,
    },
    /// Username exceeded the configured maximum.
    #[error("username too long: max={max} actual={actual}")]
    TooLong {
        /// Maximum accepted byte length.
        max: usize,
        /// Actual byte length.
        actual: usize,
    },
    /// Username did not start with a letter or number.
    #[error("username must start with an ASCII letter or digit")]
    InvalidStart,
    /// Username ended with punctuation that is easy to confuse.
    #[error("username has invalid trailing punctuation")]
    InvalidTrailingPunctuation,
    /// Username contained a character outside the allowed set.
    #[error("username contains invalid character")]
    InvalidCharacter,
    /// Username contained adjacent dots.
    #[error("username contains consecutive dots")]
    ConsecutiveDots,
    /// Username is reserved for product/system use.
    #[error("username is reserved: {name}")]
    Reserved {
        /// Rejected normalized username.
        name: String,
    },
}

impl UsernameParseError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            UsernameParseError::Empty => "empty",
            UsernameParseError::TooShort { .. } => "too_short",
            UsernameParseError::TooLong { .. } => "too_long",
            UsernameParseError::InvalidStart => "invalid_start",
            UsernameParseError::InvalidTrailingPunctuation => "invalid_trailing_punctuation",
            UsernameParseError::InvalidCharacter => "invalid_character",
            UsernameParseError::ConsecutiveDots => "consecutive_dots",
            UsernameParseError::Reserved { .. } => "reserved",
        }
    }
}

fn validate_username(name: &str) -> Result<(), UsernameParseError> {
    if name.is_empty() {
        return Err(UsernameParseError::Empty);
    }

    if name.len() < USERNAME_MIN_BYTES {
        return Err(UsernameParseError::TooShort {
            min: USERNAME_MIN_BYTES,
            actual: name.len(),
        });
    }

    if name.len() > USERNAME_MAX_BYTES {
        return Err(UsernameParseError::TooLong {
            max: USERNAME_MAX_BYTES,
            actual: name.len(),
        });
    }

    let bytes = name.as_bytes();
    if !bytes[0].is_ascii_alphanumeric() {
        return Err(UsernameParseError::InvalidStart);
    }

    if matches!(bytes[bytes.len() - 1], b'.' | b'-' | b'_') {
        return Err(UsernameParseError::InvalidTrailingPunctuation);
    }

    let mut previous_dot = false;
    for byte in bytes {
        let valid = byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(*byte, b'_' | b'-' | b'.');

        if !valid {
            return Err(UsernameParseError::InvalidCharacter);
        }

        if previous_dot && *byte == b'.' {
            return Err(UsernameParseError::ConsecutiveDots);
        }
        previous_dot = *byte == b'.';
    }

    if RESERVED_USERNAMES.binary_search(&name).is_ok() {
        return Err(UsernameParseError::Reserved {
            name: name.to_owned(),
        });
    }

    Ok(())
}

/// Reserved usernames in sorted order for deterministic binary search.
pub const RESERVED_USERNAMES: &[&str] = &[
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
];
