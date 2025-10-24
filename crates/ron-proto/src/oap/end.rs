//! RO:WHAT — OAP END frame (finalization status).
//! RO:WHY  — Terminates a flow and conveys integrity/result summary.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct End {
    pub seq_end: u64,
    pub ok: bool,
    #[serde(default)]
    pub error: Option<crate::oap::error::Error>,
}
