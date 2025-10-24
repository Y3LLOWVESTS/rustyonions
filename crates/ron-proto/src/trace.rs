//! RO:WHAT — Tiny helpers for correlation/trace fields in DTOs.
//! RO:WHY  — Allow hosts to carry correlation IDs without depending on tracing crates.

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(deny_unknown_fields)]
pub struct CorrId {
    pub id: u64,
}
