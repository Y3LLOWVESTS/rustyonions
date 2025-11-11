//! Storage-facing types (DTO-lite).
use serde::{Deserialize, Serialize};

/// The current head of the registry log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Head {
    /// Monotonic version (0 if empty).
    pub version: u64,
    /// Blake3 of the payload at head (dev: synthesized).
    pub payload_b3: String,
    /// RFC3339 timestamp if committed; null when empty.
    pub committed_at: Option<String>,
}
