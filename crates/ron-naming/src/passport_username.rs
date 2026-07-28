//! RO:WHAT — Canonical Native Passport username and handle DTOs.
//! RO:WHY — P3 Identity & Keys + P7 SDK/Interop. Keeps `@username` syntax in ron-naming instead of duplicating handle rules in services.
//! RO:INTERACTS — svc-passport native username reuse, existing public profile routes, future client SDK handle parsing.
//! RO:INVARIANTS — usernames are optional handles, not legal names, not secret material, not wallet/ledger authority, and not Passport IDs.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects reserved/confusing names, whitespace, uppercase-at-rest, non-ASCII, and authority-looking labels.
//! RO:TEST — tests/native_passport_phase2b_username_handles.rs.

use std::{fmt, str::FromStr};

/// Phase label for canonical Native Passport username/handle parsing.
pub const NATIVE_PASSPORT_PHASE2B_LABEL: &str = "NATIVE_PASSPORT_PHASE2B_RON_NAMING_USERNAME_REUSE";

/// Canonical handle prefix.
pub const HANDLE_PREFIX: &str = "@";

/// Minimum username length.
pub const USERNAME_MIN_LEN: usize = 3;

/// Maximum username length.
pub const USERNAME_MAX_LEN: usize = 32;

/// Reserved labels that cannot be claimed as optional public handles.
pub const RESERVED_USERNAME_LABELS: &[&str] = &[
    "admin",
    "administrator",
    "api",
    "auth",
    "capability",
    "crab",
    "crablink",
    "device",
    "gateway",
    "help",
    "ledger",
    "login",
    "me",
    "null",
    "operator",
    "passport",
    "profile",
    "root",
    "roc",
    "rox",
    "security",
    "service",
    "site",
    "staff",
    "support",
    "system",
    "treasury",
    "undefined",
    "wallet",
];

/// Canonical username parse errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassportUsernameParseError {
    /// Username was empty after trimming and optional `@` removal.
    Empty,
    /// Username was too short.
    TooShort {
        /// Actual character count.
        actual: usize,
        /// Minimum character count.
        min: usize,
    },
    /// Username was too long.
    TooLong {
        /// Actual character count.
        actual: usize,
        /// Maximum character count.
        max: usize,
    },
    /// Username contained an unsupported character.
    InvalidCharacter {
        /// Rejected character.
        ch: char,
    },
    /// Username did not start or end with an ASCII lowercase letter or digit.
    InvalidBoundary,
    /// Username contained a confusing separator pattern.
    ConfusingSeparator,
    /// Username is reserved.
    Reserved,
}

impl fmt::Display for PassportUsernameParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("username must not be empty"),
            Self::TooShort { actual, min } => {
                write!(f, "username is too short: got {actual}, minimum {min}")
            }
            Self::TooLong { actual, max } => {
                write!(f, "username is too long: got {actual}, maximum {max}")
            }
            Self::InvalidCharacter { ch } => {
                write!(f, "username contains invalid character {ch:?}")
            }
            Self::InvalidBoundary => {
                f.write_str("username must start and end with a lowercase ASCII letter or digit")
            }
            Self::ConfusingSeparator => {
                f.write_str("username contains a confusing separator pattern")
            }
            Self::Reserved => f.write_str("username is reserved"),
        }
    }
}

impl std::error::Error for PassportUsernameParseError {}

fn strip_optional_handle_prefix(value: &str) -> &str {
    value
        .trim()
        .strip_prefix(HANDLE_PREFIX)
        .unwrap_or(value.trim())
}

fn is_ascii_lower_or_digit(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_digit()
}

fn validate_canonical_username(value: &str) -> Result<(), PassportUsernameParseError> {
    if value.is_empty() {
        return Err(PassportUsernameParseError::Empty);
    }

    let len = value.chars().count();

    if len < USERNAME_MIN_LEN {
        return Err(PassportUsernameParseError::TooShort {
            actual: len,
            min: USERNAME_MIN_LEN,
        });
    }

    if len > USERNAME_MAX_LEN {
        return Err(PassportUsernameParseError::TooLong {
            actual: len,
            max: USERNAME_MAX_LEN,
        });
    }

    for ch in value.chars() {
        if !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_') {
            return Err(PassportUsernameParseError::InvalidCharacter { ch });
        }
    }

    let first = value.chars().next().expect("non-empty checked above");
    let last = value.chars().last().expect("non-empty checked above");

    if !is_ascii_lower_or_digit(first) || !is_ascii_lower_or_digit(last) {
        return Err(PassportUsernameParseError::InvalidBoundary);
    }

    if value.contains("__") {
        return Err(PassportUsernameParseError::ConfusingSeparator);
    }

    if RESERVED_USERNAME_LABELS.contains(&value) {
        return Err(PassportUsernameParseError::Reserved);
    }

    Ok(())
}

/// Canonical optional Native Passport username without `@`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UsernameV1(String);

impl UsernameV1 {
    /// Parse and canonicalize a username or handle.
    ///
    /// This accepts either `name` or `@name`, lowercases ASCII input, and stores
    /// only the username body. It intentionally does not accept legal names,
    /// whitespace, raw Passport IDs, wallet labels, or authority-looking labels.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, PassportUsernameParseError> {
        let raw = strip_optional_handle_prefix(value.as_ref());
        let canonical = raw.to_ascii_lowercase();

        validate_canonical_username(&canonical)?;

        Ok(Self(canonical))
    }

    /// Borrow the canonical username body without `@`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return the canonical handle with `@`.
    #[must_use]
    pub fn handle(&self) -> String {
        format!("{HANDLE_PREFIX}{}", self.0)
    }

    /// Consume and return the username body.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for UsernameV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for UsernameV1 {
    type Err = PassportUsernameParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Canonical optional Native Passport handle with `@`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandleV1(String);

impl HandleV1 {
    /// Parse and canonicalize a username or handle into `@username` form.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, PassportUsernameParseError> {
        let username = UsernameV1::parse(value)?;
        Ok(Self(username.handle()))
    }

    /// Borrow the canonical handle string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Borrow the username body without `@`.
    #[must_use]
    pub fn username(&self) -> &str {
        self.0
            .strip_prefix(HANDLE_PREFIX)
            .expect("HandleV1 always stores @ prefix")
    }

    /// Consume and return the handle string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for HandleV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for HandleV1 {
    type Err = PassportUsernameParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Return true when a string parses as a canonical optional Native Passport username.
#[must_use]
pub fn is_username_v1(value: &str) -> bool {
    UsernameV1::parse(value).is_ok()
}

/// Return true when a string parses as a canonical optional Native Passport handle.
#[must_use]
pub fn is_handle_v1(value: &str) -> bool {
    HandleV1::parse(value).is_ok()
}
