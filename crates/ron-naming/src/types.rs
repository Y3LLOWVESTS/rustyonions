//! RO:WHAT — Fundamental naming/addressing DTOs used across RON.
//! RO:WHY  — Keep schemas/validation centralized; services (svc-index) consume these types.
//! RO:INTERACTS — address, normalize, version, wire::{json,cbor}
//! RO:INVARIANTS — DTO hygiene with #[serde(deny_unknown_fields)]; content id prefix "b3:" only.
//! RO:METRICS — none here (types-only).
//! RO:SECURITY — No secrets; no I/O.
//! RO:TEST — see tests/address_hygiene.rs and dto_wire_vectors.rs.

use serde::{Deserialize, Serialize};

/// Fully Qualified Domain Name (ASCII, normalized, no trailing dot).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Fqdn(pub String);

impl Fqdn {
    /// Returns `true` if this FQDN looks syntactically valid (cheap checks).
    pub fn is_valid(&self) -> bool {
        // Minimal hygiene: 1..=253 bytes, labels 1..=63, allowed chars (a-z0-9-), no leading/trailing hyphen.
        let s = self.0.as_str();
        if s.is_empty() || s.len() > 253 || s.starts_with('.') || s.ends_with('.') {
            return false;
        }
        for label in s.split('.') {
            if label.is_empty() || label.len() > 63 {
                return false;
            }
            if label.starts_with('-') || label.ends_with('-') {
                return false;
            }
            if !label
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                return false;
            }
        }
        true
    }
}

/// Content ID — canonical BLAKE3-256 address, always `"b3:<hex>"`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentId(pub String);

impl ContentId {
    /// Validate the `b3:<hex>` shape (lowercase, 64 hex chars).
    pub fn validate(&self) -> bool {
        let s = self.0.as_str();
        if !s.starts_with("b3:") {
            return false;
        }
        let hex = &s[3..];
        hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    }
}

/// Example DTO representing a name→manifest mapping (types-only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameRecord {
    /// Normalized ASCII FQDN.
    pub name: Fqdn,
    /// Optional semantic version of the record (e.g., for app packages).
    pub version: Option<crate::version::NameVersion>,
    /// Addressed manifest/content.
    pub content: ContentId,
}
