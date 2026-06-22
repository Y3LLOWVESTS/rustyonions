//! RO:WHAT — Strict QuickChain tree-material, root-payload, and root-result DTOs for Phase 1 vectors.
//! RO:WHY — ECON/GOV: deterministic roots need reviewed, sorted, canonical JSON contracts before downstream services can display them.
//! RO:INTERACTS — hash_payload DTOs, domain separators, canonical/vector gates, future ron-ledger root projection.
//! RO:INVARIANTS — DTO/validation only; no hashing implementation; no proofs/finality; no validators; no ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — root hashes are deterministic artifacts only and grant no settlement, proof, spend, validator, or bridge authority.
//! RO:TEST — tests/quickchain_root_material.rs.

use serde::{Deserialize, Serialize};

use crate::id::ContentId;

use super::{
    domain::{
        validate_domain_separator_v1, QUICKCHAIN_ACCOUNTING_ROOT_HASH_DOMAIN_V1,
        QUICKCHAIN_HOLD_ROOT_HASH_DOMAIN_V1, QUICKCHAIN_RECEIPT_ROOT_HASH_DOMAIN_V1,
        QUICKCHAIN_REWARD_ROOT_HASH_DOMAIN_V1, QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1,
    },
    hash_payload::{
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA, QUICKCHAIN_ACTIVE_HOLD_LEAF_PAYLOAD_SCHEMA,
        QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
    },
    validate_chain_id, validate_epoch_id, validate_schema, validate_version,
    QuickChainCanonicalEncodingV1, QuickChainResult, QuickChainValidationError,
};

pub const QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA: &str = "quickchain.tree-material-item.v1";
pub const QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA: &str = "quickchain.tree-material-batch.v1";
pub const QUICKCHAIN_TREE_REDUCTION_PAIR_SCHEMA: &str = "quickchain.tree-reduction-pair.v1";
pub const QUICKCHAIN_TREE_REDUCTION_PLAN_SCHEMA: &str = "quickchain.tree-reduction-plan.v1";
pub const QUICKCHAIN_TREE_LEAF_NODE_SCHEMA: &str = "quickchain.tree-leaf-node.v1";
pub const QUICKCHAIN_TREE_BRANCH_NODE_SCHEMA: &str = "quickchain.tree-branch-node.v1";
pub const QUICKCHAIN_TREE_ROOT_PAYLOAD_SCHEMA: &str = "quickchain.tree-root-payload.v1";
pub const QUICKCHAIN_TREE_ROOT_SCHEMA: &str = "quickchain.tree-root.v1";

pub const QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1: &str =
    "bytewise_ascending_sort_key_hex_v1";
pub const QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1: &str =
    "adjacent_pairs_with_odd_carry_v1";
pub const QUICKCHAIN_TREE_ROOT_ALGORITHM_SORTED_BINARY_MERKLE_MAP_BLAKE3_JSON_V1: &str =
    "sorted_binary_merkle_map_blake3_json_v1";

pub const MAX_QUICKCHAIN_TREE_MATERIAL_SORT_KEY_HEX_BYTES: usize = 2048;
pub const MAX_QUICKCHAIN_TREE_MATERIAL_PAYLOAD_SCHEMA_BYTES: usize = 128;
pub const MAX_QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTES: usize = 128;
pub const MAX_QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_BYTES: usize = 128;
pub const MAX_QUICKCHAIN_TREE_ROOT_ALGORITHM_BYTES: usize = 128;
pub const MAX_QUICKCHAIN_TREE_MATERIAL_ITEMS: usize = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainTreeMaterialKindV1 {
    State,
    Holds,
    Receipts,
    Accounting,
    Rewards,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainTreeMaterialItemV1 {
    pub schema: String,
    pub version: u16,
    pub tree: QuickChainTreeMaterialKindV1,
    pub sort_key_hex: String,
    pub payload_schema: String,
    pub payload_hash: ContentId,
}

impl QuickChainTreeMaterialItemV1 {
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainTreeMaterialItemV1.schema",
            &self.schema,
            QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA,
        )?;
        validate_version("QuickChainTreeMaterialItemV1.version", self.version)?;
        validate_sort_key_hex(
            "QuickChainTreeMaterialItemV1.sort_key_hex",
            &self.sort_key_hex,
        )?;
        validate_payload_schema_token(&self.payload_schema)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainTreeMaterialBatchV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub tree: QuickChainTreeMaterialKindV1,
    pub canonical_encoding: QuickChainCanonicalEncodingV1,
    pub item_sort_rule: String,
    #[serde(default)]
    pub items: Vec<QuickChainTreeMaterialItemV1>,
}

impl QuickChainTreeMaterialBatchV1 {
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainTreeMaterialBatchV1.schema",
            &self.schema,
            QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA,
        )?;
        validate_version("QuickChainTreeMaterialBatchV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;

        if self.canonical_encoding != QuickChainCanonicalEncodingV1::JsonV1 {
            return Err(QuickChainValidationError::InvalidField {
                field: "canonical_encoding",
                reason: "Phase 1 tree material currently allows only json-v1",
            });
        }

        validate_sort_rule(&self.item_sort_rule)?;

        if self.items.len() > MAX_QUICKCHAIN_TREE_MATERIAL_ITEMS {
            return Err(QuickChainValidationError::TooManyItems {
                field: "items",
                max: MAX_QUICKCHAIN_TREE_MATERIAL_ITEMS,
                actual: self.items.len(),
            });
        }

        let mut previous_sort_key_hex: Option<&str> = None;

        for item in &self.items {
            item.validate()?;

            if item.tree != self.tree {
                return Err(QuickChainValidationError::InvalidField {
                    field: "items.tree",
                    reason: "all material items must match the batch tree",
                });
            }

            validate_tree_payload_schema_pair(self.tree, &item.payload_schema)?;

            if let Some(previous) = previous_sort_key_hex {
                if previous >= item.sort_key_hex.as_str() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "items.sort_key_hex",
                        reason: "material item sort keys must be strictly ascending and unique",
                    });
                }
            }

            previous_sort_key_hex = Some(item.sort_key_hex.as_str());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainTreeReductionPairV1 {
    pub schema: String,
    pub version: u16,
    pub tree: QuickChainTreeMaterialKindV1,
    pub layer_index: u32,
    pub pair_index: u32,
    pub left_sort_key_hex: String,
    pub left_payload_hash: ContentId,
    #[serde(default)]
    pub right_sort_key_hex: Option<String>,
    #[serde(default)]
    pub right_payload_hash: Option<ContentId>,
}

impl QuickChainTreeReductionPairV1 {
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainTreeReductionPairV1.schema",
            &self.schema,
            QUICKCHAIN_TREE_REDUCTION_PAIR_SCHEMA,
        )?;
        validate_version("QuickChainTreeReductionPairV1.version", self.version)?;

        if self.layer_index != 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "layer_index",
                reason: "Phase 1 reduction plans describe only material-layer pairs",
            });
        }

        validate_sort_key_hex(
            "QuickChainTreeReductionPairV1.left_sort_key_hex",
            &self.left_sort_key_hex,
        )?;

        match (&self.right_sort_key_hex, &self.right_payload_hash) {
            (Some(right_sort_key_hex), Some(_)) => {
                validate_sort_key_hex(
                    "QuickChainTreeReductionPairV1.right_sort_key_hex",
                    right_sort_key_hex,
                )?;

                if self.left_sort_key_hex.as_str() >= right_sort_key_hex.as_str() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "right_sort_key_hex",
                        reason: "right sort key must be strictly greater than left sort key",
                    });
                }
            }
            (None, None) => {}
            _ => {
                return Err(QuickChainValidationError::InvalidField {
                    field: "right_pair_member",
                    reason:
                        "right sort key and right payload hash must both be present or both absent",
                });
            }
        }

        Ok(())
    }

    #[must_use]
    pub const fn has_right_member(&self) -> bool {
        self.right_sort_key_hex.is_some() && self.right_payload_hash.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainTreeReductionPlanV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub tree: QuickChainTreeMaterialKindV1,
    pub canonical_encoding: QuickChainCanonicalEncodingV1,
    pub item_sort_rule: String,
    pub pair_rule: String,
    pub source_items_count: u64,
    #[serde(default)]
    pub pairs: Vec<QuickChainTreeReductionPairV1>,
}

impl QuickChainTreeReductionPlanV1 {
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainTreeReductionPlanV1.schema",
            &self.schema,
            QUICKCHAIN_TREE_REDUCTION_PLAN_SCHEMA,
        )?;
        validate_version("QuickChainTreeReductionPlanV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;

        if self.canonical_encoding != QuickChainCanonicalEncodingV1::JsonV1 {
            return Err(QuickChainValidationError::InvalidField {
                field: "canonical_encoding",
                reason: "Phase 1 tree reduction plans currently allow only json-v1",
            });
        }

        validate_sort_rule(&self.item_sort_rule)?;
        validate_pair_rule(&self.pair_rule)?;
        validate_source_items_count(self.source_items_count)?;

        let expected_pairs = expected_pair_count(self.source_items_count);
        if self.pairs.len() != expected_pairs {
            return Err(QuickChainValidationError::InvalidField {
                field: "pairs",
                reason: "pair count must equal ceil(source_items_count / 2)",
            });
        }

        let mut counted_source_items = 0_u64;
        let mut previous_last_sort_key_hex: Option<&str> = None;

        for (index, pair) in self.pairs.iter().enumerate() {
            pair.validate()?;

            if pair.tree != self.tree {
                return Err(QuickChainValidationError::InvalidField {
                    field: "pairs.tree",
                    reason: "all reduction pairs must match the plan tree",
                });
            }

            if pair.pair_index as usize != index {
                return Err(QuickChainValidationError::InvalidField {
                    field: "pair_index",
                    reason: "pair indexes must be zero-based and strictly contiguous",
                });
            }

            if let Some(previous) = previous_last_sort_key_hex {
                if previous >= pair.left_sort_key_hex.as_str() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "pairs.left_sort_key_hex",
                        reason: "pair sort keys must remain strictly ascending across pairs",
                    });
                }
            }

            counted_source_items += 1;

            if pair.has_right_member() {
                counted_source_items += 1;
                previous_last_sort_key_hex = pair.right_sort_key_hex.as_deref();
            } else {
                if index + 1 != self.pairs.len() || self.source_items_count % 2 == 0 {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "right_pair_member",
                        reason: "absent right member is allowed only as the final odd carry",
                    });
                }

                previous_last_sort_key_hex = Some(pair.left_sort_key_hex.as_str());
            }
        }

        if counted_source_items != self.source_items_count {
            return Err(QuickChainValidationError::InvalidField {
                field: "source_items_count",
                reason: "source item count must match the members represented by pairs",
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainTreeLeafNodeV1 {
    pub schema: String,
    pub version: u16,
    pub tree: QuickChainTreeMaterialKindV1,
    pub sort_key_hex: String,
    pub payload_schema: String,
    pub payload_hash: ContentId,
}

impl QuickChainTreeLeafNodeV1 {
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainTreeLeafNodeV1.schema",
            &self.schema,
            QUICKCHAIN_TREE_LEAF_NODE_SCHEMA,
        )?;
        validate_version("QuickChainTreeLeafNodeV1.version", self.version)?;
        validate_sort_key_hex("QuickChainTreeLeafNodeV1.sort_key_hex", &self.sort_key_hex)?;
        validate_payload_schema_token(&self.payload_schema)?;
        validate_tree_payload_schema_pair(self.tree, &self.payload_schema)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainTreeBranchNodeV1 {
    pub schema: String,
    pub version: u16,
    pub tree: QuickChainTreeMaterialKindV1,
    pub layer_index: u32,
    pub pair_index: u32,
    pub left_node_hash: ContentId,
    pub right_node_hash: ContentId,
}

impl QuickChainTreeBranchNodeV1 {
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainTreeBranchNodeV1.schema",
            &self.schema,
            QUICKCHAIN_TREE_BRANCH_NODE_SCHEMA,
        )?;
        validate_version("QuickChainTreeBranchNodeV1.version", self.version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainTreeRootPayloadV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub tree: QuickChainTreeMaterialKindV1,
    pub canonical_encoding: QuickChainCanonicalEncodingV1,
    pub item_sort_rule: String,
    pub pair_rule: String,
    pub hash_domain: String,
    pub source_items_count: u64,
    pub tree_height: u32,
    #[serde(default)]
    pub root_node_hash: Option<ContentId>,
}

impl QuickChainTreeRootPayloadV1 {
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainTreeRootPayloadV1.schema",
            &self.schema,
            QUICKCHAIN_TREE_ROOT_PAYLOAD_SCHEMA,
        )?;
        validate_version("QuickChainTreeRootPayloadV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_root_common(
            self.tree,
            self.canonical_encoding,
            &self.item_sort_rule,
            &self.pair_rule,
            &self.hash_domain,
            self.source_items_count,
        )?;

        if self.source_items_count == 0 && self.root_node_hash.is_some() {
            return Err(QuickChainValidationError::InvalidField {
                field: "root_node_hash",
                reason: "empty tree root payload must use explicit null root_node_hash",
            });
        }

        if self.source_items_count > 0 && self.root_node_hash.is_none() {
            return Err(QuickChainValidationError::InvalidField {
                field: "root_node_hash",
                reason: "non-empty tree root payload must carry the final node hash",
            });
        }

        if self.source_items_count <= 1 && self.tree_height != 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "tree_height",
                reason: "empty or single-item trees must have height 0",
            });
        }

        if self.source_items_count > 1 && self.tree_height == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "tree_height",
                reason: "multi-item trees must have at least one reduction layer",
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainTreeRootV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub tree: QuickChainTreeMaterialKindV1,
    pub canonical_encoding: QuickChainCanonicalEncodingV1,
    pub item_sort_rule: String,
    pub pair_rule: String,
    pub hash_domain: String,
    pub algorithm: String,
    pub source_items_count: u64,
    pub tree_height: u32,
    pub root_hash: ContentId,
}

impl QuickChainTreeRootV1 {
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainTreeRootV1.schema",
            &self.schema,
            QUICKCHAIN_TREE_ROOT_SCHEMA,
        )?;
        validate_version("QuickChainTreeRootV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_root_common(
            self.tree,
            self.canonical_encoding,
            &self.item_sort_rule,
            &self.pair_rule,
            &self.hash_domain,
            self.source_items_count,
        )?;
        validate_root_algorithm(&self.algorithm)?;

        if self.source_items_count <= 1 && self.tree_height != 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "tree_height",
                reason: "empty or single-item trees must have height 0",
            });
        }

        if self.source_items_count > 1 && self.tree_height == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "tree_height",
                reason: "multi-item trees must have at least one reduction layer",
            });
        }

        Ok(())
    }
}

#[must_use]
pub const fn quickchain_tree_root_domain_for_tree(
    tree: QuickChainTreeMaterialKindV1,
) -> &'static str {
    match tree {
        QuickChainTreeMaterialKindV1::State => QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1,
        QuickChainTreeMaterialKindV1::Holds => QUICKCHAIN_HOLD_ROOT_HASH_DOMAIN_V1,
        QuickChainTreeMaterialKindV1::Receipts => QUICKCHAIN_RECEIPT_ROOT_HASH_DOMAIN_V1,
        QuickChainTreeMaterialKindV1::Accounting => QUICKCHAIN_ACCOUNTING_ROOT_HASH_DOMAIN_V1,
        QuickChainTreeMaterialKindV1::Rewards => QUICKCHAIN_REWARD_ROOT_HASH_DOMAIN_V1,
    }
}

fn validate_root_common(
    tree: QuickChainTreeMaterialKindV1,
    canonical_encoding: QuickChainCanonicalEncodingV1,
    item_sort_rule: &str,
    pair_rule: &str,
    hash_domain: &str,
    source_items_count: u64,
) -> QuickChainResult<()> {
    if canonical_encoding != QuickChainCanonicalEncodingV1::JsonV1 {
        return Err(QuickChainValidationError::InvalidField {
            field: "canonical_encoding",
            reason: "Phase 1 tree roots currently allow only json-v1",
        });
    }

    validate_sort_rule(item_sort_rule)?;
    validate_pair_rule(pair_rule)?;
    validate_source_items_count(source_items_count)?;
    validate_domain_separator_v1(hash_domain)?;

    if hash_domain != quickchain_tree_root_domain_for_tree(tree) {
        return Err(QuickChainValidationError::InvalidField {
            field: "hash_domain",
            reason: "hash domain must match the tree kind",
        });
    }

    Ok(())
}

fn validate_root_algorithm(value: &str) -> QuickChainResult<()> {
    validate_payload_schema_like_token(
        "algorithm",
        value,
        MAX_QUICKCHAIN_TREE_ROOT_ALGORITHM_BYTES,
    )?;

    if value != QUICKCHAIN_TREE_ROOT_ALGORITHM_SORTED_BINARY_MERKLE_MAP_BLAKE3_JSON_V1 {
        return Err(QuickChainValidationError::InvalidField {
            field: "algorithm",
            reason: "unsupported QuickChain tree-root algorithm",
        });
    }

    Ok(())
}

fn validate_sort_rule(value: &str) -> QuickChainResult<()> {
    validate_payload_schema_like_token(
        "item_sort_rule",
        value,
        MAX_QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTES,
    )?;

    if value != QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1 {
        return Err(QuickChainValidationError::InvalidField {
            field: "item_sort_rule",
            reason: "unsupported tree-material sort rule",
        });
    }

    Ok(())
}

fn validate_pair_rule(value: &str) -> QuickChainResult<()> {
    validate_payload_schema_like_token(
        "pair_rule",
        value,
        MAX_QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_BYTES,
    )?;

    if value != QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1 {
        return Err(QuickChainValidationError::InvalidField {
            field: "pair_rule",
            reason: "unsupported tree-reduction pair rule",
        });
    }

    Ok(())
}

fn validate_source_items_count(value: u64) -> QuickChainResult<()> {
    if value > MAX_QUICKCHAIN_TREE_MATERIAL_ITEMS as u64 {
        return Err(QuickChainValidationError::TooManyItems {
            field: "source_items_count",
            max: MAX_QUICKCHAIN_TREE_MATERIAL_ITEMS,
            actual: match usize::try_from(value) {
                Ok(actual) => actual,
                Err(_) => usize::MAX,
            },
        });
    }

    Ok(())
}

fn expected_pair_count(source_items_count: u64) -> usize {
    source_items_count.div_ceil(2) as usize
}

fn validate_tree_payload_schema_pair(
    tree: QuickChainTreeMaterialKindV1,
    payload_schema: &str,
) -> QuickChainResult<()> {
    let expected = match tree {
        QuickChainTreeMaterialKindV1::State => Some(QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA),
        QuickChainTreeMaterialKindV1::Holds => Some(QUICKCHAIN_ACTIVE_HOLD_LEAF_PAYLOAD_SCHEMA),
        QuickChainTreeMaterialKindV1::Receipts => Some(QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA),
        QuickChainTreeMaterialKindV1::Accounting | QuickChainTreeMaterialKindV1::Rewards => None,
    };

    if let Some(expected) = expected {
        if payload_schema != expected {
            return Err(QuickChainValidationError::InvalidField {
                field: "payload_schema",
                reason: "payload schema does not match tree material kind",
            });
        }
    }

    Ok(())
}

fn validate_payload_schema_token(value: &str) -> QuickChainResult<()> {
    validate_payload_schema_like_token(
        "payload_schema",
        value,
        MAX_QUICKCHAIN_TREE_MATERIAL_PAYLOAD_SCHEMA_BYTES,
    )?;

    if !value.starts_with("quickchain.") || !value.ends_with(".v1") || value.contains("..") {
        return Err(QuickChainValidationError::InvalidField {
            field: "payload_schema",
            reason: "must be a versioned quickchain.*.v1 schema token",
        });
    }

    Ok(())
}

fn validate_payload_schema_like_token(
    field: &'static str,
    value: &str,
    max: usize,
) -> QuickChainResult<()> {
    if value.trim().is_empty() {
        return Err(QuickChainValidationError::EmptyField { field });
    }

    let actual = value.len();
    if actual > max {
        return Err(QuickChainValidationError::FieldTooLong { field, max, actual });
    }

    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
    }) {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason:
                "must contain only lowercase ASCII letters, digits, dots, underscores, or hyphens",
        });
    }

    Ok(())
}

fn validate_sort_key_hex(field: &'static str, value: &str) -> QuickChainResult<()> {
    if value.is_empty() {
        return Err(QuickChainValidationError::EmptyField { field });
    }

    let actual = value.len();
    if actual > MAX_QUICKCHAIN_TREE_MATERIAL_SORT_KEY_HEX_BYTES {
        return Err(QuickChainValidationError::FieldTooLong {
            field,
            max: MAX_QUICKCHAIN_TREE_MATERIAL_SORT_KEY_HEX_BYTES,
            actual,
        });
    }

    if actual % 2 != 0 {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "hex sort key must have an even number of characters",
        });
    }

    if !value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "hex sort key must be lowercase hexadecimal",
        });
    }

    Ok(())
}

/// Return the only Phase-1 tree-material encoding currently allowed by the DTO boundary.
///
/// This keeps downstream projection code from naming or owning the encoding enum directly
/// while still producing a fully validated ron-proto DTO.
#[must_use]
pub const fn quickchain_tree_material_json_v1_encoding() -> QuickChainCanonicalEncodingV1 {
    QuickChainCanonicalEncodingV1::JsonV1
}

/// Schema tag for a single QuickChain tree inclusion-proof step.
pub const QUICKCHAIN_TREE_INCLUSION_PROOF_STEP_SCHEMA: &str =
    "quickchain.tree-inclusion-proof-step.v1";

/// Schema tag for a QuickChain tree inclusion proof.
pub const QUICKCHAIN_TREE_INCLUSION_PROOF_SCHEMA: &str = "quickchain.tree-inclusion-proof.v1";

/// Maximum proof steps allowed by the Phase 1 material cap.
pub const MAX_QUICKCHAIN_TREE_PROOF_STEPS: usize = 256;

/// Position of a sibling node relative to the current proof path node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainTreeProofSiblingPositionV1 {
    /// Sibling hash is the left child; current path hash is the right child.
    Left,
    /// Sibling hash is the right child; current path hash is the left child.
    Right,
}

/// One sibling commitment in a deterministic QuickChain tree inclusion proof.
///
/// This DTO carries commitments only. It does not verify hashes, assign finality,
/// authorize settlement, or represent validator consensus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainTreeInclusionProofStepV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Tree kind this proof step belongs to.
    pub tree: QuickChainTreeMaterialKindV1,
    /// Zero-based reduction layer index.
    pub layer_index: u32,
    /// Sibling position relative to the current proof path node.
    pub sibling_position: QuickChainTreeProofSiblingPositionV1,
    /// Sibling node commitment.
    pub sibling_node_hash: ContentId,
}

impl QuickChainTreeInclusionProofStepV1 {
    /// Validate the proof-step DTO shape.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainTreeInclusionProofStepV1.schema",
            &self.schema,
            QUICKCHAIN_TREE_INCLUSION_PROOF_STEP_SCHEMA,
        )?;
        validate_version("QuickChainTreeInclusionProofStepV1.version", self.version)
    }
}

/// Deterministic inclusion proof for one material leaf under one tree root.
///
/// This is a proof *format* only. Verification belongs to the ledger/verifier
/// implementation that recomputes the same canonical leaf, branch, and root
/// payload hashes. The proof does not grant spend authority, finality, validator
/// authority, bridge authority, or external settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainTreeInclusionProofV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Chain identity.
    pub chain_id: String,
    /// Epoch identity.
    pub epoch_id: String,
    /// Tree kind.
    pub tree: QuickChainTreeMaterialKindV1,
    /// Canonical encoding used for proof hash payloads.
    pub canonical_encoding: QuickChainCanonicalEncodingV1,
    /// Material item sort rule.
    pub item_sort_rule: String,
    /// Tree reduction pair rule.
    pub pair_rule: String,
    /// Hash domain for this tree kind.
    pub hash_domain: String,
    /// Root algorithm token.
    pub algorithm: String,
    /// Number of source material items committed by the root.
    pub source_items_count: u64,
    /// Reduction height of the committed tree.
    pub tree_height: u32,
    /// Root hash this proof targets.
    pub root_hash: ContentId,
    /// Zero-based leaf index after material sorting.
    pub leaf_index: u64,
    /// Leaf payload committed by this proof.
    pub leaf: QuickChainTreeLeafNodeV1,
    /// Sibling commitments needed to reconstruct the root.
    #[serde(default)]
    pub steps: Vec<QuickChainTreeInclusionProofStepV1>,
}

impl QuickChainTreeInclusionProofV1 {
    /// Validate the inclusion-proof DTO shape.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainTreeInclusionProofV1.schema",
            &self.schema,
            QUICKCHAIN_TREE_INCLUSION_PROOF_SCHEMA,
        )?;
        validate_version("QuickChainTreeInclusionProofV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_root_common(
            self.tree,
            self.canonical_encoding,
            &self.item_sort_rule,
            &self.pair_rule,
            &self.hash_domain,
            self.source_items_count,
        )?;
        validate_root_algorithm(&self.algorithm)?;

        if self.source_items_count == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "source_items_count",
                reason: "inclusion proofs require a non-empty tree",
            });
        }

        if self.leaf_index >= self.source_items_count {
            return Err(QuickChainValidationError::InvalidField {
                field: "leaf_index",
                reason: "leaf index must be within the source material count",
            });
        }

        if self.tree_height > MAX_QUICKCHAIN_TREE_PROOF_STEPS as u32 {
            return Err(QuickChainValidationError::TooManyItems {
                field: "tree_height",
                max: MAX_QUICKCHAIN_TREE_PROOF_STEPS,
                actual: self.tree_height as usize,
            });
        }

        if self.steps.len() > MAX_QUICKCHAIN_TREE_PROOF_STEPS {
            return Err(QuickChainValidationError::TooManyItems {
                field: "steps",
                max: MAX_QUICKCHAIN_TREE_PROOF_STEPS,
                actual: self.steps.len(),
            });
        }

        if self.steps.len() > self.tree_height as usize {
            return Err(QuickChainValidationError::InvalidField {
                field: "steps",
                reason: "proof cannot contain more steps than the tree height",
            });
        }

        self.leaf.validate()?;

        if self.leaf.tree != self.tree {
            return Err(QuickChainValidationError::InvalidField {
                field: "leaf.tree",
                reason: "proof leaf tree must match proof tree",
            });
        }

        let mut previous_layer_index: Option<u32> = None;

        for step in &self.steps {
            step.validate()?;

            if step.tree != self.tree {
                return Err(QuickChainValidationError::InvalidField {
                    field: "steps.tree",
                    reason: "all proof steps must match the proof tree",
                });
            }

            if step.layer_index >= self.tree_height {
                return Err(QuickChainValidationError::InvalidField {
                    field: "steps.layer_index",
                    reason: "proof step layer must be below tree height",
                });
            }

            if let Some(previous) = previous_layer_index {
                if step.layer_index <= previous {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "steps.layer_index",
                        reason: "proof step layers must be strictly ascending",
                    });
                }
            }

            previous_layer_index = Some(step.layer_index);
        }

        if self.source_items_count <= 1 && self.tree_height != 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "tree_height",
                reason: "single-item inclusion proofs must have height 0",
            });
        }

        if self.source_items_count > 1 && self.tree_height == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "tree_height",
                reason: "multi-item inclusion proofs must have non-zero height",
            });
        }

        Ok(())
    }
}
