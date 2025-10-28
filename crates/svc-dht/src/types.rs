//! RO:WHAT — Common types (B3Cid validator, NodeUri placeholder).
//! RO:WHY  — DX/SEC hardening: validate CIDs early; avoid junk through the pipeline.
//! RO:INTERACTS — rpc::http, provider::Store, pipeline::lookup.
//! RO:INVARIANTS — BLAKE3-256 only; lowercase hex; "b3:<64-hex>"; no locks across .await.
//! RO:SECURITY — Rejects malformed IDs with 400; prevents cache poisoning.
//! RO:TEST — unit in tests/provider_roundtrip.rs and rpc/http tests.

use std::fmt;
use std::str::FromStr;

/// Canonical content address: "b3:<64-lowercase-hex>" (BLAKE3-256)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct B3Cid(String);

impl B3Cid {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for B3Cid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for B3Cid {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Strict: exactly "b3:" + 64 lowercase hex chars.
        const PREFIX: &str = "b3:";
        if !s.starts_with(PREFIX) {
            return Err("bad-prefix");
        }
        let hex = &s[PREFIX.len()..];
        if hex.len() != 64 {
            return Err("bad-length");
        }
        if !hex.as_bytes().iter().all(|b| matches!(b, b'0'..=b'9'|b'a'..=b'f')) {
            return Err("bad-hex");
        }
        Ok(B3Cid(s.to_string()))
    }
}

// Serde glue so DTOs can use B3Cid directly.
impl<'de> serde::Deserialize<'de> for B3Cid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}
impl serde::Serialize for B3Cid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

/// Very light Node URI checker (MVP): "<scheme>://<id>"
/// We only require non-empty and forbid whitespace; detailed validation left for transport layer.
pub fn validate_node_uri(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    if s.contains(char::is_whitespace) {
        return false;
    }
    s.contains("://")
}
