//! RO:WHAT — Content-address parser for sealed rewarder inputs.
//! RO:WHY — Pillar 12; Concerns: SEC/GOV. Reward runs must bind to canonical `b3:<hex>` input handles.
//! RO:INTERACTS — http DTOs, run_key generation, manifest commitments.
//! RO:INVARIANTS — only full BLAKE3-256 hex CIDs accepted; lowercase canonical output.
//! RO:METRICS — invalid CIDs are counted by callers as bad_request.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects truncated/malformed content IDs.
//! RO:TEST — inline cid parser tests.

use serde::{Deserialize, Serialize};

use crate::{Result, RewarderError};

/// Canonical BLAKE3 content id string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentCid(String);

impl ContentCid {
    /// Parse and canonicalize a `b3:<64 lowercase or uppercase hex>` CID.
    pub fn parse(input: impl AsRef<str>) -> Result<Self> {
        let raw = input.as_ref().trim();
        let hex = raw
            .strip_prefix("b3:")
            .ok_or_else(|| RewarderError::BadRequest("inputs_cid must start with b3:".into()))?;
        if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(RewarderError::BadRequest(
                "inputs_cid must be b3:<64 hex chars>".into(),
            ));
        }
        Ok(Self(format!("b3:{}", hex.to_ascii_lowercase())))
    }

    /// Borrow the canonical string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ContentCid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_full_b3_hex() {
        let cid = ContentCid::parse(format!("b3:{}", "A".repeat(64))).unwrap();
        assert_eq!(cid.as_str(), format!("b3:{}", "a".repeat(64)));
    }
}
