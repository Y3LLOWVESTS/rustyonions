//! RO:WHAT — `MoveEntryV1` debit/credit record shape (no arithmetic here).
//! RO:WHY  — Deterministic, signed-friendly DTO for ECON services.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MoveEntryV1 {
    pub entry_id: String,
    pub account: String,
    /// Positive integer value in minor units (e.g., cents)
    pub amount_minor: u64,
    /// +1 for credit, -1 for debit (host logic enforces consistency)
    pub sign: i8,
    #[serde(default)]
    pub memo: Option<String>,
}
