//! RO:WHAT — Common DHT types: canonical B3 CIDs and Crab node identities.
//! RO:WHY  — Phase 7 IP privacy: DHT provider records advertise crab://node identities,
//!   not transport-specific routes, raw IPs, or user residential sockets.
//! RO:INTERACTS — rpc::http, provider::Store, pipeline::lookup, peer::NodeId.
//! RO:INVARIANTS — BLAKE3-256 CIDs only; crab://node/<64-lowercase-hex> only;
//!   no relay://, onion://, service://, tcp://, http://, socket, LAN, or IP literals.
//! RO:SECURITY — Rejects malformed IDs before they enter the provider store.
//! RO:TEST — tests/crab_node_identity.rs, tests/provider_roundtrip.rs, tests/api_smoke.rs.

use crate::peer::NodeId;
use std::fmt;
use std::str::FromStr;

/// Canonical content address: "b3:<64-lowercase-hex>" (BLAKE3-256).
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
        const PREFIX: &str = "b3:";

        if !s.starts_with(PREFIX) {
            return Err("bad-prefix");
        }

        let hex = &s[PREFIX.len()..];

        if hex.len() != 64 {
            return Err("bad-length");
        }

        if !hex.as_bytes().iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return Err("bad-hex");
        }

        Ok(B3Cid(s.to_string()))
    }
}

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

pub const CRAB_NODE_URI_PREFIX: &str = "crab://node/";

/// Canonical CrabLink/RustyOnions node identity.
///
/// Internal networking code should pass this typed identity around. The
/// `crab://node/<node-id>` string is only the API/CLI/UI representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CrabNodeId([u8; 32]);

impl CrabNodeId {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_node_id(self) -> NodeId {
        NodeId::from_bytes(self.0)
    }

    pub fn to_node_hex(self) -> String {
        hex::encode(self.0)
    }

    pub fn to_uri(self) -> String {
        format!("{CRAB_NODE_URI_PREFIX}{}", self.to_node_hex())
    }

    pub fn from_node_hex(hex: &str) -> Result<Self, CrabNodeIdError> {
        let bytes = parse_lower_hex_32(hex)?;
        Ok(Self(bytes))
    }

    pub fn from_uri(uri: &str) -> Result<Self, CrabNodeIdError> {
        if uri.is_empty() {
            return Err(CrabNodeIdError::Empty);
        }

        if uri.contains(char::is_whitespace) {
            return Err(CrabNodeIdError::Whitespace);
        }

        let Some(hex) = uri.strip_prefix(CRAB_NODE_URI_PREFIX) else {
            return Err(CrabNodeIdError::MissingCrabNodePrefix);
        };

        Self::from_node_hex(hex)
    }
}

impl fmt::Display for CrabNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_uri())
    }
}

impl FromStr for CrabNodeId {
    type Err = CrabNodeIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_uri(s)
    }
}

impl<'de> serde::Deserialize<'de> for CrabNodeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_uri(&s).map_err(serde::de::Error::custom)
    }
}

impl serde::Serialize for CrabNodeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_uri())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabNodeIdError {
    Empty,
    Whitespace,
    MissingCrabNodePrefix,
    BadLength,
    BadHex,
}

impl CrabNodeIdError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Whitespace => "whitespace",
            Self::MissingCrabNodePrefix => "missing_crab_node_prefix",
            Self::BadLength => "bad_length",
            Self::BadHex => "bad_hex",
        }
    }
}

impl fmt::Display for CrabNodeIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for CrabNodeIdError {}

/// Backward-compatible checker used by existing HTTP call sites.
pub fn validate_node_uri(s: &str) -> bool {
    CrabNodeId::from_uri(s).is_ok()
}

fn parse_lower_hex_32(hex: &str) -> Result<[u8; 32], CrabNodeIdError> {
    if hex.len() != 64 {
        return Err(CrabNodeIdError::BadLength);
    }

    let mut out = [0u8; 32];

    for (idx, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
        let hi = lower_hex_value(pair[0]).ok_or(CrabNodeIdError::BadHex)?;
        let lo = lower_hex_value(pair[1]).ok_or(CrabNodeIdError::BadHex)?;
        out[idx] = (hi << 4) | lo;
    }

    Ok(out)
}

fn lower_hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + (b - b'a')),
        _ => None,
    }
}
