//! RO:WHAT — Pure sorting adapter from explicit QuickChain material inputs into ron-proto tree-material DTOs.
//! RO:WHY — ECON/RES: Phase 1 needs deterministic material order before any reducing or proof engine exists.
//! RO:INTERACTS — ron-proto tree-material DTOs and future vector gates.
//! RO:INVARIANTS — BTreeMap ordering; explicit payload commitments only; no hashing, no serialization, no clocks, no IO, no mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none; available only through quickchain-preflight.
//! RO:SECURITY — supplied payload commitments are inert inputs and do not grant settlement, receipt, or spend authority.
//! RO:TEST — tests/quickchain_phase1_tree_material.rs.

use std::collections::BTreeMap;

use ron_proto::{
    quickchain::{
        quickchain_tree_material_json_v1_encoding, QuickChainTreeMaterialBatchV1,
        QuickChainTreeMaterialItemV1, QuickChainTreeMaterialKindV1, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA, QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA,
        QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1,
    },
    ContentId,
};
use thiserror::Error;

/// One caller-supplied material item before deterministic ordering.
///
/// This value is intentionally not a leaf, branch, proof, tree, receipt, or
/// checkpoint. It only binds an already-reviewed sort key, payload schema, and
/// opaque payload commitment so ron-ledger can assemble the exact input order
/// expected by the ron-proto tree-material DTO boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainTreeMaterialProjectionItem {
    sort_key_hex: String,
    payload_schema: String,
    payload_hash: ContentId,
}

impl QuickChainTreeMaterialProjectionItem {
    /// Construct one unordered material-projection item.
    #[must_use]
    pub fn new(
        sort_key_hex: impl Into<String>,
        payload_schema: impl Into<String>,
        payload_hash: ContentId,
    ) -> Self {
        Self {
            sort_key_hex: sort_key_hex.into(),
            payload_schema: payload_schema.into(),
            payload_hash,
        }
    }

    /// Return the lowercase hex sort key supplied by the caller.
    #[must_use]
    pub fn sort_key_hex(&self) -> &str {
        &self.sort_key_hex
    }

    /// Return the versioned payload schema token supplied by the caller.
    #[must_use]
    pub fn payload_schema(&self) -> &str {
        &self.payload_schema
    }

    /// Return the opaque payload commitment supplied by the caller.
    #[must_use]
    pub const fn payload_hash(&self) -> &ContentId {
        &self.payload_hash
    }
}

/// Deterministic rejection reasons from tree-material projection.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum QuickChainTreeMaterialProjectionError {
    /// Two caller inputs carried the same sort key.
    #[error("duplicate QuickChain tree-material sort key: {sort_key_hex}")]
    DuplicateSortKey {
        /// Duplicate lowercase hex sort key.
        sort_key_hex: String,
    },

    /// The assembled ron-proto tree-material batch failed strict validation.
    #[error("invalid QuickChain tree-material batch: {reason}")]
    InvalidBatch {
        /// Bounded validation reason from the ron-proto DTO boundary.
        reason: String,
    },
}

/// Sort explicit material inputs and assemble a strict ron-proto tree-material batch.
///
/// This function deliberately does not compute bytes, commitments, reductions,
/// proofs, checkpoints, signatures, or finality. It is only a deterministic
/// ordering adapter for Phase 1 vector preparation.
pub fn build_tree_material_batch(
    chain_id: impl Into<String>,
    epoch_id: impl Into<String>,
    tree: QuickChainTreeMaterialKindV1,
    items: impl IntoIterator<Item = QuickChainTreeMaterialProjectionItem>,
) -> Result<QuickChainTreeMaterialBatchV1, QuickChainTreeMaterialProjectionError> {
    let mut sorted = BTreeMap::<String, QuickChainTreeMaterialProjectionItem>::new();

    for item in items {
        let sort_key_hex = item.sort_key_hex().to_string();

        if sorted.insert(sort_key_hex.clone(), item).is_some() {
            return Err(QuickChainTreeMaterialProjectionError::DuplicateSortKey { sort_key_hex });
        }
    }

    let items = sorted
        .into_values()
        .map(|item| QuickChainTreeMaterialItemV1 {
            schema: QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA.to_string(),
            version: QUICKCHAIN_DTO_VERSION,
            tree,
            sort_key_hex: item.sort_key_hex,
            payload_schema: item.payload_schema,
            payload_hash: item.payload_hash,
        })
        .collect();

    let batch = QuickChainTreeMaterialBatchV1 {
        schema: QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: chain_id.into(),
        epoch_id: epoch_id.into(),
        tree,
        canonical_encoding: quickchain_tree_material_json_v1_encoding(),
        item_sort_rule: QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1.to_string(),
        items,
    };

    batch.validate().map_err(
        |error| QuickChainTreeMaterialProjectionError::InvalidBatch {
            reason: error.to_string(),
        },
    )?;

    Ok(batch)
}
