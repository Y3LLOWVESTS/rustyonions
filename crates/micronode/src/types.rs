//! RO:WHAT — Shared DTOs for tiny endpoints (/version, dev echo).
//! RO:WHY  — Keep handler files small and composable.
//! RO:INVARIANTS — DTO hygiene: #[serde(deny_unknown_fields)].

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    pub name: &'static str,
    pub version: &'static str,
    pub built_at_unix: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Echo {
    pub message: String,
}
