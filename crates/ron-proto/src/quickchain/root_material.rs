//! RO:WHAT — Strict QuickChain tree-material DTOs for Phase 1 root-vector preparation.
//! RO:WHY — ECON/GOV: future roots need reviewed, sorted input manifests before any root producer exists.
//! RO:INTERACTS — hash_payload DTOs, canonical/vector gates, future ron-ledger material projection.
//! RO:INVARIANTS — DTO-only; no hashing; no Merkle construction; no roots; no proof verification; no ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — payload hashes are opaque reviewed inputs and grant no settlement, proof, or spend authority.
//! RO:TEST — tests/quickchain_root_material.rs.

use serde::{Deserialize, Serialize};

use crate::id::ContentId;

use super::{
    hash_payload::{
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA, QUICKCHAIN_ACTIVE_HOLD_LEAF_PAYLOAD_SCHEMA,
        QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
    },
    validate_chain_id, validate_epoch_id, validate_schema, validate_version,
    QuickChainCanonicalEncodingV1, QuickChainResult, QuickChainValidationError,
};

pub const QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA: &str = "quickchain.tree-material-item.v1";
pub const QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA: &str = "quickchain.tree-material-batch.v1";

pub const QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1: &str =
    "bytewise_ascending_sort_key_hex_v1";

pub const MAX_QUICKCHAIN_TREE_MATERIAL_SORT_KEY_HEX_BYTES: usize = 2048;
pub const MAX_QUICKCHAIN_TREE_MATERIAL_PAYLOAD_SCHEMA_BYTES: usize = 128;
pub const MAX_QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTES: usize = 128;
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
/// This keeps downstream pre-root projection code from naming or owning the
/// encoding enum directly while still producing a fully validated ron-proto DTO.
#[must_use]
pub const fn quickchain_tree_material_json_v1_encoding() -> QuickChainCanonicalEncodingV1 {
    QuickChainCanonicalEncodingV1::JsonV1
}
