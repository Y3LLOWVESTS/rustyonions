//! RO:WHAT — Naming/manifest reference types (pure data).
//! RO:WHY  — Provide typed handles for index/gateway layers; resolution lives elsewhere.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NameRef {
    pub name: String,                           // e.g., "user:stevan/avatar"
    pub expected: Option<crate::id::ContentId>, // optional pin for strong reads
}
