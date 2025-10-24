//! RO:WHAT — `ManifestV1` DTO (explicit version field).
//! RO:WHY  — Deterministic, growth-tolerant manifest for CAS graphs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ManifestV1 {
    pub version: u32, // = 1
    pub root: crate::id::ContentId,
    /// Ordered mapping (deterministic) from name → object reference
    pub entries: BTreeMap<String, crate::manifest::EntryRef>,
    #[serde(default)]
    pub meta: BTreeMap<String, String>,
}

impl Default for ManifestV1 {
    fn default() -> Self {
        Self {
            version: 1,
            root: "b3:0000000000000000000000000000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            entries: BTreeMap::new(),
            meta: BTreeMap::new(),
        }
    }
}
