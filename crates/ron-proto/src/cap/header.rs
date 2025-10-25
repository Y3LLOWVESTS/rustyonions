//! RO:WHAT — `CapTokenHdr` with typed claims (no signatures here).
//! RO:WHY  — Stable header schema for capability enforcement.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapTokenHdr {
    pub subject: String, // e.g., user or service id
    pub scope: String,   // e.g., "read", "write-once"
    pub issued_at: u64,  // seconds
    pub expires_at: u64, // seconds (short TTL recommended)
    #[serde(default)]
    pub caveats: Vec<crate::cap::Caveat>,
}
