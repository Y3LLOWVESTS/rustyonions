//! RO:WHAT — Canonical Native Passport ID DTOs and strict parsers.
//! RO:WHY — P7 SDK/Interop + P3 Identity & Keys; one cross-crate ID model prevents duplicate service parsers.
//! RO:INTERACTS — svc-passport native DTO layer, future ron-auth proof verification, client SDKs.
//! RO:INVARIANTS — parse/serde only; no hashing, key derivation, signing, vault access, wallet mutation, or ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects legacy labels, uppercase hex, short digests, wrong algorithms, and non-production prefixes.
//! RO:TEST — tests/native_passport_phase2a_proto_canonical_ids.rs.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Canonical Passport ID prefix for Native Passport V1 main Ed25519 IDs.
pub const PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX: &str = "passport:v1:main:ed25519:b3:";

/// Canonical Device ID prefix for Native Passport V1 Ed25519 device IDs.
pub const DEVICE_ID_V1_ED25519_B3_PREFIX: &str = "device:v1:ed25519:b3:";

/// Canonical Challenge ID prefix for Native Passport V1 challenge IDs.
pub const CHALLENGE_ID_V1_B3_PREFIX: &str = "challenge:v1:b3:";

/// Required lowercase BLAKE3-256 digest length in hex characters.
pub const B3_DIGEST_HEX_LEN: usize = 64;

/// Required Ed25519 public-key length in lowercase hex characters.
pub const ED25519_PUBLIC_KEY_HEX_LEN: usize = 64;

/// Native Passport ID version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NativePassportIdVersion {
    /// Version 1.
    V1,
}

/// Native Passport ID kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NativePassportIdKind {
    /// Main Passport identity.
    Main,
}

/// Native Passport key algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NativePassportIdAlgorithm {
    /// Ed25519 public-key binding.
    Ed25519,
}

/// Native Passport digest algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NativePassportDigestAlgorithm {
    /// BLAKE3-256 digest encoded as lowercase hex.
    Blake3,
}

/// Canonical device class for Native Passport DTOs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DeviceClassV1 {
    /// TV read-only delegated device.
    TvReadOnly,
    /// Desktop read-only device.
    DesktopReadOnly,
    /// Mobile read-only device.
    MobileReadOnly,
}

/// Strict parse errors for Native Passport canonical ID DTOs.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NativePassportIdParseError {
    /// Required prefix was not present.
    #[error("missing or invalid prefix for {field}")]
    InvalidPrefix {
        /// Field name.
        field: &'static str,
    },
    /// Digest/public-key length was wrong.
    #[error("invalid lowercase hex length for {field}: expected {expected_len}, got {actual_len}")]
    InvalidHexLength {
        /// Field name.
        field: &'static str,
        /// Expected hex length.
        expected_len: usize,
        /// Actual hex length.
        actual_len: usize,
    },
    /// Hex contained uppercase or non-hex bytes.
    #[error("invalid lowercase hex alphabet for {field}")]
    InvalidLowerHex {
        /// Field name.
        field: &'static str,
    },
}

fn is_lower_hex_len(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_lower_hex(
    field: &'static str,
    value: &str,
    expected_len: usize,
) -> Result<(), NativePassportIdParseError> {
    if value.len() != expected_len {
        return Err(NativePassportIdParseError::InvalidHexLength {
            field,
            expected_len,
            actual_len: value.len(),
        });
    }

    if !is_lower_hex_len(value, expected_len) {
        return Err(NativePassportIdParseError::InvalidLowerHex { field });
    }

    Ok(())
}

fn parse_prefixed(
    field: &'static str,
    value: impl Into<String>,
    prefix: &'static str,
    digest_len: usize,
) -> Result<String, NativePassportIdParseError> {
    let value = value.into();
    let digest = value
        .strip_prefix(prefix)
        .ok_or(NativePassportIdParseError::InvalidPrefix { field })?;

    validate_lower_hex(field, digest, digest_len)?;
    Ok(value)
}

/// Return true when a value is a canonical Native Passport V1 main Ed25519 Passport ID.
#[must_use]
pub fn is_passport_id_v1_main_ed25519_b3(value: &str) -> bool {
    PassportIdV1::parse(value).is_ok()
}

/// Return true when a value is a canonical Native Passport V1 Ed25519 Device ID.
#[must_use]
pub fn is_device_id_v1_ed25519_b3(value: &str) -> bool {
    DeviceIdV1::parse(value).is_ok()
}

/// Return true when a value is a canonical Native Passport V1 Challenge ID.
#[must_use]
pub fn is_challenge_id_v1_b3(value: &str) -> bool {
    ChallengeIdV1::parse(value).is_ok()
}

/// Canonical Native Passport V1 Passport ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct PassportIdV1(String);

impl PassportIdV1 {
    /// Parse a canonical Passport ID.
    pub fn parse(value: impl Into<String>) -> Result<Self, NativePassportIdParseError> {
        parse_prefixed(
            "passport_id",
            value,
            PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX,
            B3_DIGEST_HEX_LEN,
        )
        .map(Self)
    }

    /// Borrow the canonical string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the canonical string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for PassportIdV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PassportIdV1 {
    type Err = NativePassportIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl<'de> Deserialize<'de> for PassportIdV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// Canonical Native Passport V1 Device ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct DeviceIdV1(String);

impl DeviceIdV1 {
    /// Parse a canonical Device ID.
    pub fn parse(value: impl Into<String>) -> Result<Self, NativePassportIdParseError> {
        parse_prefixed(
            "device_id",
            value,
            DEVICE_ID_V1_ED25519_B3_PREFIX,
            B3_DIGEST_HEX_LEN,
        )
        .map(Self)
    }

    /// Borrow the canonical string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the canonical string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for DeviceIdV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DeviceIdV1 {
    type Err = NativePassportIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl<'de> Deserialize<'de> for DeviceIdV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// Canonical Native Passport V1 Challenge ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ChallengeIdV1(String);

impl ChallengeIdV1 {
    /// Parse a canonical Challenge ID.
    pub fn parse(value: impl Into<String>) -> Result<Self, NativePassportIdParseError> {
        parse_prefixed(
            "challenge_id",
            value,
            CHALLENGE_ID_V1_B3_PREFIX,
            B3_DIGEST_HEX_LEN,
        )
        .map(Self)
    }

    /// Borrow the canonical string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the canonical string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for ChallengeIdV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ChallengeIdV1 {
    type Err = NativePassportIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl<'de> Deserialize<'de> for ChallengeIdV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// Strict lowercase BLAKE3-256 digest hex DTO.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct B3DigestHex(String);

impl B3DigestHex {
    /// Parse a lowercase BLAKE3-256 digest hex string.
    pub fn parse(
        field: &'static str,
        value: impl Into<String>,
    ) -> Result<Self, NativePassportIdParseError> {
        let value = value.into();
        validate_lower_hex(field, &value, B3_DIGEST_HEX_LEN)?;
        Ok(Self(value))
    }

    /// Borrow the digest string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the digest string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for B3DigestHex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for B3DigestHex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse("b3_digest_hex", value).map_err(serde::de::Error::custom)
    }
}

/// Strict Ed25519 public key lowercase hex DTO.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Ed25519PublicKeyHex(String);

impl Ed25519PublicKeyHex {
    /// Parse an Ed25519 public-key lowercase hex string.
    pub fn parse(value: impl Into<String>) -> Result<Self, NativePassportIdParseError> {
        let value = value.into();
        validate_lower_hex("ed25519_public_key_hex", &value, ED25519_PUBLIC_KEY_HEX_LEN)?;
        Ok(Self(value))
    }

    /// Borrow the public-key hex string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the public-key hex string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for Ed25519PublicKeyHex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Ed25519PublicKeyHex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// Legacy subject string kept separate from production PassportIdV1.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LegacyPassportSubject(String);

impl LegacyPassportSubject {
    /// Store a legacy subject label without granting production PassportIdV1 authority.
    pub fn parse(value: impl Into<String>) -> Result<Self, NativePassportIdParseError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(NativePassportIdParseError::InvalidPrefix {
                field: "legacy_subject",
            });
        }
        Ok(Self(value))
    }

    /// Borrow the legacy subject string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
