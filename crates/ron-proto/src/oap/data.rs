//! RO:WHAT — OAP DATA frame with object address and payload chunk.
//! RO:WHY  — Carries addressed bytes; readers verify BLAKE3 elsewhere.
//! RO:INVARIANTS — obj is "b3:<hex>"; bytes length must respect max_frame (host-enforced).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub obj: crate::id::ContentId,
    pub seq: u64,
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}
