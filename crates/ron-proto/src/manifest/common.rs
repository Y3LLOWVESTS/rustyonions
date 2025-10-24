//! RO:WHAT — Shared enums/consts across manifest versions.
//! RO:WHY  — Keep cross-version evolution clean and explicit.
//! RO:INVARIANTS — Deterministic defaults; serde rejects unknown fields where used.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MediaKind {
    Blob,
    Manifest,
}

impl Default for MediaKind {
    fn default() -> Self {
        MediaKind::Blob
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EntryRef {
    pub id: crate::id::ContentId,
    pub size: u64,
    /// Default to `blob` when omitted for backward compatibility.
    #[serde(default)]
    pub kind: MediaKind,
}
