//! RO:WHAT — `ContentId` newtype ("b3:<hex>") with strict parser/serde.
//! RO:WHY  — Enforce I-1 addressing (BLAKE3) and deterministic casing across SDKs.
//! RO:INTERACTS — oap::Data/End, manifest entries, naming refs.
//! RO:INVARIANTS — hex length=64, lowercase, prefix "b3:"; serde rejects unknown/invalid forms.
//! RO:TEST — proptest for random valid/invalid strings; vector parity tests.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

pub const CONTENT_ID_PREFIX: &str = "b3:";
pub const CONTENT_ID_HEX_LEN: usize = 64;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentId(String);

impl ContentId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parse with strict validation.
    pub fn parse(s: &str) -> Result<Self, crate::id::ParseContentIdError> {
        crate::id::validate_b3_str(s)?;
        Ok(Self(s.to_string()))
    }
}

impl fmt::Debug for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Avoid dumping long hex in logs: short preview
        write!(
            f,
            "ContentId({}…)",
            &self.0[..std::cmp::min(self.0.len(), 8)]
        )
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ContentId {
    type Err = crate::id::ParseContentIdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Serialize for ContentId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ContentId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        crate::id::validate_b3_str(&s).map_err(serde::de::Error::custom)?;
        Ok(Self(s))
    }
}
