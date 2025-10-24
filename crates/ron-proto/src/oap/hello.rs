//! RO:WHAT — OAP HELLO frame (protocol/version negotiation).
//! RO:WHY  — Establishes version/features; first contact envelope.
//! RO:INVARIANTS — Strict fields; unknown fields rejected. Use owned `String`
//!                 so JSON deserialization doesn't require a `'static` lifetime.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Hello {
    pub protocol: String,       // was &'static str; use String for serde-compat
    pub version: u32,           // mirrors PROTO_VERSION
    #[serde(default)]
    pub features: Vec<String>,  // future growth; strings must be stable tokens
}

impl Default for Hello {
    fn default() -> Self {
        Self {
            protocol: "OAP/1".to_string(),
            version: crate::version::PROTO_VERSION,
            features: Vec::new(),
        }
    }
}
