//! RO:WHAT — OAP ERROR envelope (wire-level error details).
//! RO:WHY  — Stable, typed error codes that SDKs can rely on for metrics and control flow.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Error {
    pub code: crate::error::Kind,
    pub message: String,
    #[serde(default)]
    pub detail: Option<String>,
}
