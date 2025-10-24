//! RO:WHAT — OAP START frame (session/stream parameters).
//! RO:WHY  — Sets negotiated limits before DATA frames flow.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Start {
    pub seq_start: u64,
    pub max_frame_bytes: u32, // must be <= 1MiB; validated by hosts
    #[serde(default)]
    pub meta: Option<String>, // reserved for growth (e.g., codec hints)
}
