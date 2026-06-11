//! RO:WHAT — Canonical empty-tree payload DTO for QuickChain Phase 0 byte vectors.
//! RO:WHY — ECON/RES: future empty roots need explicit typed bytes rather than magic constants.
//! RO:INTERACTS — canonical JSON helpers, test-vector metadata, future root code outside ron-proto.
//! RO:INVARIANTS — DTO-only; no hashing; no tree construction; no roots; no ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — this payload does not claim or produce an empty root.
//! RO:TEST — tests/quickchain_empty_tree.rs.

use serde::{Deserialize, Serialize};

use super::{validate_schema, validate_version, QuickChainResult};

/// Schema tag for canonical empty-tree payloads.
pub const QUICKCHAIN_EMPTY_TREE_SCHEMA: &str = "quickchain.empty-tree.v1";

/// Tree category represented by a canonical empty-tree payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainEmptyTreeKindV1 {
    /// Global account/state tree.
    State,

    /// Per-account or global hold-state collection.
    Holds,

    /// Epoch receipt collection.
    Receipts,

    /// Sealed accounting collection.
    Accounting,

    /// Sealed reward-manifest collection.
    Rewards,
}

/// Typed canonical payload for a future empty-tree commitment.
///
/// This DTO prevents future empty roots from being undocumented constants.
/// It does not compute a hash or choose a Merkle construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainEmptyTreeV1 {
    pub schema: String,
    pub version: u16,
    pub tree: QuickChainEmptyTreeKindV1,
}

impl QuickChainEmptyTreeV1 {
    /// Validate DTO shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainEmptyTreeV1.schema",
            &self.schema,
            QUICKCHAIN_EMPTY_TREE_SCHEMA,
        )?;
        validate_version("QuickChainEmptyTreeV1.version", self.version)
    }
}
