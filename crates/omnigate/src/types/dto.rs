//! RO:WHAT — Public response DTOs the API returns.
//! RO:WHY  — Tests serialize/deserialize these; keep stable wire shape.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionResponse {
    pub version: String,
    /// Optional short git hash if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PingResponse {
    pub ok: bool,
}
