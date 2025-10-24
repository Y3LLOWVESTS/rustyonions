//! RO:WHAT — Mailbox Ack DTO.
//! RO:WHY  — Allows hosts to acknowledge processing; no side-effects here.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Ack {
    pub msg_id: String,
    pub ok: bool,
    #[serde(default)]
    pub error: Option<crate::error::ProtoError>,
}
