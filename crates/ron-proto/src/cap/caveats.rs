//! RO:WHAT — Enumerated caveats/flags attached to capability tokens.
//! RO:WHY  — Additive growth via non_exhaustive enum + reserved fields.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Caveat {
    IpAllowlist { cidrs: Vec<String> },
    WriteOnce,
    ContentPrefix { prefix: String }, // e.g., restrict to a subtree/name
}
pub type CaveatKind = Caveat;
