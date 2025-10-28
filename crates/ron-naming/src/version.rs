//! RO:WHAT — Name version wrapper atop semver for optional versioned records.
//! RO:WHY  — Keep version grammar stable and decoupled from services.
//! RO:INVARIANTS — Strict semver parse; serialized as plain string.
//! RO:TEST — unit tests in this module.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Semantic Version wrapper (e.g., "1.2.3").
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NameVersion(pub semver::Version);

impl Serialize for NameVersion {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for NameVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        semver::Version::parse(&s)
            .map(NameVersion)
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

/// Parser error.
#[derive(thiserror::Error, Debug)]
#[error("invalid version: {0}")]
pub struct VersionParseError(pub String);

impl fmt::Display for NameVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Parse a version string into `NameVersion`.
pub fn parse_version(s: &str) -> Result<NameVersion, VersionParseError> {
    semver::Version::parse(s)
        .map(NameVersion)
        .map_err(|e| VersionParseError(e.to_string()))
}
