//! RO:WHAT — Mailbox Recv DTO (deliver to consumer).
//! RO:WHY  — Symmetric with Send; pure data only.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Recv {
    pub msg_id: String,
    pub from: String,
    pub kind: String,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}
