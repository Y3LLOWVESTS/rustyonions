//! RO:WHAT — Mailbox Send DTO with idempotency headers.
//! RO:WHY  — Enable at-least-once delivery patterns without logic here.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Send {
    pub msg_id: String,
    pub to: String,
    pub kind: String,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
    /// Host-side idempotency key for dedupe
    #[serde(default)]
    pub idempotency_key: Option<String>,
}
