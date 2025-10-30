//! RO:WHAT — Policy model (DTOs): `PolicyBundle`, Rule, Conditions, Actions, Obligations.
//!
//! RO:WHY  — DTO hygiene: `#[serde(deny_unknown_fields)]` so policies are explicit and auditable.
//!
//! RO:INTERACTS — parse loaders, engine eval, explain trace
//!
//! RO:INVARIANTS — deny-by-default; stable enums; versioned bundle

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyBundle {
    pub version: u32,
    /// Optional metadata bag (stringly typed, for governance/notes).
    #[serde(default)]
    pub meta: BTreeMap<String, String>,
    /// Global defaults. If `default_action` is omitted -> deny-by-default.
    #[serde(default)]
    pub defaults: Defaults,
    /// Rules are evaluated in order; first match wins (unless `strategy` overrides).
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Defaults {
    #[serde(default)]
    pub default_action: Option<Action>,
    /// Max request body the engine expects callers to allow before evaluation (bytes).
    #[serde(default)]
    pub max_body_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub when: RuleCondition,
    pub action: Action,
    #[serde(default)]
    pub obligations: Vec<Obligation>,
    /// Optional human-readable reason to surface if this rule triggers.
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleCondition {
    /// Tenant or "*" for any.
    #[serde(default)]
    pub tenant: Option<String>,
    /// Method verb (e.g., "GET", "PUT") or "*" for any.
    #[serde(default)]
    pub method: Option<String>,
    /// Region/Geo (e.g., "US", "EU", "US-CA", "US-FL") or "*" for any.
    #[serde(default)]
    pub region: Option<String>,
    /// If present, deny if `body_bytes` exceeds this.
    #[serde(default)]
    pub max_body_bytes: Option<u64>,
    /// Arbitrary tags (all must be present in context if specified).
    #[serde(default)]
    pub require_tags_all: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Obligation {
    /// Name (e.g., "add-header", "mask-field", "log-audit")
    pub kind: String,
    /// Arbitrary parameters.
    #[serde(default)]
    pub params: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    #[default]
    Deny,
}
