//! RO:WHAT — Pure QuickChain material sorting, reduction planning, and deterministic local root projection.
//! RO:WHY — ECON/RES: Phase 1 requires replayable roots from explicit sorted material without DB order, clocks, or services.
//! RO:INTERACTS — ron-proto tree-material/root DTOs and canonical JSON helpers.
//! RO:INVARIANTS — BTreeMap ordering; BLAKE3 over domain||0x00||canonical JSON; no IO, clocks, validators, finality, or mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none; available only through quickchain-preflight.
//! RO:SECURITY — produced roots are local deterministic artifacts and do not grant settlement, spend, validator, bridge, or finality authority.
//! RO:TEST — tests/quickchain_phase1_tree_material.rs.

use std::collections::BTreeMap;

use ron_proto::{
    quickchain::{
        quickchain_tree_material_json_v1_encoding, quickchain_tree_root_domain_for_tree,
        to_canonical_json_vec, QuickChainTreeBranchNodeV1, QuickChainTreeInclusionProofStepV1,
        QuickChainTreeInclusionProofV1, QuickChainTreeLeafNodeV1, QuickChainTreeMaterialBatchV1,
        QuickChainTreeMaterialItemV1, QuickChainTreeMaterialKindV1,
        QuickChainTreeProofSiblingPositionV1, QuickChainTreeReductionPairV1,
        QuickChainTreeReductionPlanV1, QuickChainTreeRootPayloadV1, QuickChainTreeRootV1,
        QUICKCHAIN_DTO_VERSION, QUICKCHAIN_TREE_BRANCH_NODE_SCHEMA,
        QUICKCHAIN_TREE_INCLUSION_PROOF_SCHEMA, QUICKCHAIN_TREE_INCLUSION_PROOF_STEP_SCHEMA,
        QUICKCHAIN_TREE_LEAF_NODE_SCHEMA, QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA,
        QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA,
        QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1,
        QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1,
        QUICKCHAIN_TREE_REDUCTION_PAIR_SCHEMA, QUICKCHAIN_TREE_REDUCTION_PLAN_SCHEMA,
        QUICKCHAIN_TREE_ROOT_PAYLOAD_SCHEMA, QUICKCHAIN_TREE_ROOT_SCHEMA,
    },
    ContentId,
};
use serde::Serialize;
use thiserror::Error;

/// One caller-supplied material item before deterministic ordering.
///
/// This value is intentionally not receipt authority, a checkpoint, finality, or
/// a live chain-state mutation. It binds an already-reviewed sort key, payload
/// schema, and payload commitment so ron-ledger can assemble the exact input
/// order expected by the ron-proto tree-material DTO boundary.
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

    /// The assembled ron-proto tree-material/root DTO failed strict validation.
    #[error("invalid QuickChain tree material/root DTO: {reason}")]
    InvalidBatch {
        /// Bounded validation reason from the ron-proto DTO boundary.
        reason: String,
    },

    /// Canonical JSON bytes could not be produced for a root-vector payload.
    #[error("invalid QuickChain canonical bytes: {reason}")]
    InvalidCanonicalBytes {
        /// Serialization failure reason.
        reason: String,
    },

    /// A computed BLAKE3 digest failed strict b3 ContentId parsing.
    #[error("invalid QuickChain computed b3 hash: {reason}")]
    InvalidComputedHash {
        /// Strict ContentId parse failure reason.
        reason: String,
    },
    /// The requested material sort key was not present in the validated material batch.
    #[error("QuickChain tree material sort key not found: {sort_key_hex}")]
    MaterialSortKeyNotFound {
        /// Missing lowercase hex sort key.
        sort_key_hex: String,
    },

    /// The supplied proof/root pair failed deterministic verification.
    #[error("QuickChain tree proof verification failed: {reason}")]
    ProofVerificationFailed {
        /// Bounded verification reason.
        reason: &'static str,
    },

    /// The assembled ron-proto tree proof DTO failed strict validation.
    #[error("invalid QuickChain tree proof DTO: {reason}")]
    InvalidProof {
        /// Bounded validation reason from the ron-proto DTO boundary.
        reason: String,
    },
}

/// Sort explicit material inputs and assemble a strict ron-proto tree-material batch.
///
/// This function does not read from storage, clocks, wall time, database cursors,
/// caches, or services. Caller order is erased through BTreeMap sorting and
/// duplicate sort keys reject before any batch is returned.
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

    validate_batch(&batch)?;

    Ok(batch)
}

/// Build the first deterministic adjacent-pair plan from a validated material batch.
///
/// This describes which sorted payload commitments are paired by the Phase 1
/// reducer. Root production is handled separately by `compute_tree_root_from_batch`.
pub fn build_tree_reduction_plan_from_batch(
    batch: &QuickChainTreeMaterialBatchV1,
) -> Result<QuickChainTreeReductionPlanV1, QuickChainTreeMaterialProjectionError> {
    validate_batch(batch)?;

    let mut pairs = Vec::with_capacity(batch.items.len().div_ceil(2));

    for (pair_index, chunk) in batch.items.chunks(2).enumerate() {
        let left = &chunk[0];
        let right = chunk.get(1);

        pairs.push(QuickChainTreeReductionPairV1 {
            schema: QUICKCHAIN_TREE_REDUCTION_PAIR_SCHEMA.to_string(),
            version: QUICKCHAIN_DTO_VERSION,
            tree: batch.tree,
            layer_index: 0,
            pair_index: pair_index as u32,
            left_sort_key_hex: left.sort_key_hex.clone(),
            left_payload_hash: left.payload_hash.clone(),
            right_sort_key_hex: right.map(|item| item.sort_key_hex.clone()),
            right_payload_hash: right.map(|item| item.payload_hash.clone()),
        });
    }

    let plan = QuickChainTreeReductionPlanV1 {
        schema: QUICKCHAIN_TREE_REDUCTION_PLAN_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: batch.chain_id.clone(),
        epoch_id: batch.epoch_id.clone(),
        tree: batch.tree,
        canonical_encoding: quickchain_tree_material_json_v1_encoding(),
        item_sort_rule: QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1.to_string(),
        pair_rule: QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1.to_string(),
        source_items_count: batch.items.len() as u64,
        pairs,
    };

    plan.validate().map_err(
        |error| QuickChainTreeMaterialProjectionError::InvalidBatch {
            reason: error.to_string(),
        },
    )?;

    Ok(plan)
}

/// Compute a local deterministic Phase 1 tree root from a validated material batch.
///
/// Hash rule, for every leaf/branch/final-root payload, is exactly:
///
/// ```text
/// domain_separator_bytes || 0x00 || canonical_payload_bytes
/// ```
///
/// The domain is selected by `QuickChainTreeMaterialKindV1` through ron-proto's
/// audited v1 root-domain constants. The reducer is a sorted binary Merkle map:
/// leaf nodes commit `sort_key_hex + payload_schema + payload_hash`, branch nodes
/// commit two child node hashes, and an odd final item is carried forward to the
/// next layer. The final root is domain-separated again over a root payload that
/// binds chain, epoch, tree kind, sort/reduction rules, source count, height, and
/// final node hash. Empty roots use explicit `root_node_hash: null`.
pub fn compute_tree_root_from_batch(
    batch: &QuickChainTreeMaterialBatchV1,
) -> Result<QuickChainTreeRootV1, QuickChainTreeMaterialProjectionError> {
    validate_batch(batch)?;

    let hash_domain = quickchain_tree_root_domain_for_tree(batch.tree);
    let mut layer = Vec::with_capacity(batch.items.len());

    for item in &batch.items {
        let leaf = QuickChainTreeLeafNodeV1 {
            schema: QUICKCHAIN_TREE_LEAF_NODE_SCHEMA.to_string(),
            version: QUICKCHAIN_DTO_VERSION,
            tree: batch.tree,
            sort_key_hex: item.sort_key_hex.clone(),
            payload_schema: item.payload_schema.clone(),
            payload_hash: item.payload_hash.clone(),
        };

        leaf.validate().map_err(
            |error| QuickChainTreeMaterialProjectionError::InvalidBatch {
                reason: error.to_string(),
            },
        )?;

        layer.push(hash_canonical_payload(hash_domain, &leaf)?);
    }

    let mut tree_height = 0_u32;

    while layer.len() > 1 {
        let mut next_layer = Vec::with_capacity(layer.len().div_ceil(2));

        for (pair_index, chunk) in layer.chunks(2).enumerate() {
            if chunk.len() == 1 {
                next_layer.push(chunk[0].clone());
                continue;
            }

            let branch = QuickChainTreeBranchNodeV1 {
                schema: QUICKCHAIN_TREE_BRANCH_NODE_SCHEMA.to_string(),
                version: QUICKCHAIN_DTO_VERSION,
                tree: batch.tree,
                layer_index: tree_height,
                pair_index: pair_index as u32,
                left_node_hash: chunk[0].clone(),
                right_node_hash: chunk[1].clone(),
            };

            branch.validate().map_err(|error| {
                QuickChainTreeMaterialProjectionError::InvalidBatch {
                    reason: error.to_string(),
                }
            })?;

            next_layer.push(hash_canonical_payload(hash_domain, &branch)?);
        }

        layer = next_layer;
        tree_height += 1;
    }

    let root_node_hash = layer.into_iter().next();
    let root_payload = QuickChainTreeRootPayloadV1 {
        schema: QUICKCHAIN_TREE_ROOT_PAYLOAD_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: batch.chain_id.clone(),
        epoch_id: batch.epoch_id.clone(),
        tree: batch.tree,
        canonical_encoding: quickchain_tree_material_json_v1_encoding(),
        item_sort_rule: QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1.to_string(),
        pair_rule: QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1.to_string(),
        hash_domain: hash_domain.to_string(),
        source_items_count: batch.items.len() as u64,
        tree_height,
        root_node_hash,
    };

    root_payload.validate().map_err(|error| {
        QuickChainTreeMaterialProjectionError::InvalidBatch {
            reason: error.to_string(),
        }
    })?;

    let root_hash = hash_canonical_payload(hash_domain, &root_payload)?;
    let root = QuickChainTreeRootV1 {
        schema: QUICKCHAIN_TREE_ROOT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: batch.chain_id.clone(),
        epoch_id: batch.epoch_id.clone(),
        tree: batch.tree,
        canonical_encoding: quickchain_tree_material_json_v1_encoding(),
        item_sort_rule: QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1.to_string(),
        pair_rule: QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1.to_string(),
        hash_domain: hash_domain.to_string(),
        algorithm: "sorted_binary_merkle_map_blake3_json_v1".to_string(),
        source_items_count: batch.items.len() as u64,
        tree_height,
        root_hash,
    };

    root.validate().map_err(
        |error| QuickChainTreeMaterialProjectionError::InvalidBatch {
            reason: error.to_string(),
        },
    )?;

    Ok(root)
}

fn validate_batch(
    batch: &QuickChainTreeMaterialBatchV1,
) -> Result<(), QuickChainTreeMaterialProjectionError> {
    batch.validate().map_err(
        |error| QuickChainTreeMaterialProjectionError::InvalidBatch {
            reason: error.to_string(),
        },
    )
}

fn hash_canonical_payload<T>(
    hash_domain: &str,
    payload: &T,
) -> Result<ContentId, QuickChainTreeMaterialProjectionError>
where
    T: Serialize,
{
    let canonical_payload_bytes = to_canonical_json_vec(payload).map_err(|error| {
        QuickChainTreeMaterialProjectionError::InvalidCanonicalBytes {
            reason: error.to_string(),
        }
    })?;

    let mut framed_bytes =
        Vec::with_capacity(hash_domain.len() + 1 + canonical_payload_bytes.len());
    framed_bytes.extend_from_slice(hash_domain.as_bytes());
    framed_bytes.push(0x00);
    framed_bytes.extend_from_slice(&canonical_payload_bytes);

    let digest = blake3::hash(&framed_bytes).to_hex().to_string();
    format!("b3:{digest}")
        .parse::<ContentId>()
        .map_err(
            |error| QuickChainTreeMaterialProjectionError::InvalidComputedHash {
                reason: error.to_string(),
            },
        )
}

/// Build a deterministic inclusion proof for one sorted material item.
///
/// The caller selects the target by lowercase hex sort key. The proof includes
/// only sibling commitments required to recompute the root, while odd carries
/// are represented implicitly by `source_items_count`, `tree_height`, and
/// `leaf_index`.
pub fn build_tree_inclusion_proof_from_batch(
    batch: &QuickChainTreeMaterialBatchV1,
    sort_key_hex: &str,
) -> Result<QuickChainTreeInclusionProofV1, QuickChainTreeMaterialProjectionError> {
    validate_batch(batch)?;

    let leaf_index = batch
        .items
        .iter()
        .position(|item| item.sort_key_hex == sort_key_hex)
        .ok_or_else(
            || QuickChainTreeMaterialProjectionError::MaterialSortKeyNotFound {
                sort_key_hex: sort_key_hex.to_string(),
            },
        )?;

    let hash_domain = quickchain_tree_root_domain_for_tree(batch.tree);
    let mut layer = Vec::with_capacity(batch.items.len());

    for item in &batch.items {
        let leaf = leaf_node_from_item(batch.tree, item);

        leaf.validate().map_err(
            |error| QuickChainTreeMaterialProjectionError::InvalidBatch {
                reason: error.to_string(),
            },
        )?;

        layer.push(hash_canonical_payload(hash_domain, &leaf)?);
    }

    let proof_leaf = leaf_node_from_item(batch.tree, &batch.items[leaf_index]);
    let mut proof_steps = Vec::new();
    let mut target_index = leaf_index;
    let mut tree_height = 0_u32;

    while layer.len() > 1 {
        if !(layer.len() % 2 == 1 && target_index == layer.len() - 1) {
            let sibling_index = if target_index % 2 == 0 {
                target_index + 1
            } else {
                target_index - 1
            };

            let sibling_position = if sibling_index < target_index {
                QuickChainTreeProofSiblingPositionV1::Left
            } else {
                QuickChainTreeProofSiblingPositionV1::Right
            };

            proof_steps.push(QuickChainTreeInclusionProofStepV1 {
                schema: QUICKCHAIN_TREE_INCLUSION_PROOF_STEP_SCHEMA.to_string(),
                version: QUICKCHAIN_DTO_VERSION,
                tree: batch.tree,
                layer_index: tree_height,
                sibling_position,
                sibling_node_hash: layer[sibling_index].clone(),
            });
        }

        let mut next_layer = Vec::with_capacity(layer.len().div_ceil(2));

        for (pair_index, chunk) in layer.chunks(2).enumerate() {
            if chunk.len() == 1 {
                next_layer.push(chunk[0].clone());
                continue;
            }

            let branch = QuickChainTreeBranchNodeV1 {
                schema: QUICKCHAIN_TREE_BRANCH_NODE_SCHEMA.to_string(),
                version: QUICKCHAIN_DTO_VERSION,
                tree: batch.tree,
                layer_index: tree_height,
                pair_index: pair_index as u32,
                left_node_hash: chunk[0].clone(),
                right_node_hash: chunk[1].clone(),
            };

            branch.validate().map_err(|error| {
                QuickChainTreeMaterialProjectionError::InvalidBatch {
                    reason: error.to_string(),
                }
            })?;

            next_layer.push(hash_canonical_payload(hash_domain, &branch)?);
        }

        layer = next_layer;
        target_index /= 2;
        tree_height += 1;
    }

    let root = compute_tree_root_from_batch(batch)?;
    let proof = QuickChainTreeInclusionProofV1 {
        schema: QUICKCHAIN_TREE_INCLUSION_PROOF_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: batch.chain_id.clone(),
        epoch_id: batch.epoch_id.clone(),
        tree: batch.tree,
        canonical_encoding: quickchain_tree_material_json_v1_encoding(),
        item_sort_rule: QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1.to_string(),
        pair_rule: QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1.to_string(),
        hash_domain: hash_domain.to_string(),
        algorithm: "sorted_binary_merkle_map_blake3_json_v1".to_string(),
        source_items_count: batch.items.len() as u64,
        tree_height,
        root_hash: root.root_hash,
        leaf_index: leaf_index as u64,
        leaf: proof_leaf,
        steps: proof_steps,
    };

    proof.validate().map_err(
        |error| QuickChainTreeMaterialProjectionError::InvalidProof {
            reason: error.to_string(),
        },
    )?;

    Ok(proof)
}

/// Verify a deterministic inclusion proof against a root DTO.
///
/// This verifier recomputes the leaf hash, each required branch hash, the final
/// root payload hash, and all root/proof metadata. It does not consult storage,
/// wall clocks, services, validators, anchors, or external settlement state.
pub fn verify_tree_inclusion_proof(
    root: &QuickChainTreeRootV1,
    proof: &QuickChainTreeInclusionProofV1,
) -> Result<(), QuickChainTreeMaterialProjectionError> {
    root.validate().map_err(
        |error| QuickChainTreeMaterialProjectionError::InvalidBatch {
            reason: error.to_string(),
        },
    )?;
    proof.validate().map_err(
        |error| QuickChainTreeMaterialProjectionError::InvalidProof {
            reason: error.to_string(),
        },
    )?;

    ensure_root_and_proof_metadata_match(root, proof)?;

    let hash_domain = proof.hash_domain.as_str();
    let mut current_hash = hash_canonical_payload(hash_domain, &proof.leaf)?;
    let mut current_index = proof.leaf_index as usize;
    let mut current_count = proof.source_items_count as usize;
    let mut steps = proof.steps.iter().peekable();

    for layer_index in 0..proof.tree_height {
        if current_count % 2 == 1 && current_index == current_count - 1 {
            current_index /= 2;
            current_count = current_count.div_ceil(2);
            continue;
        }

        let step = steps.next().ok_or(
            QuickChainTreeMaterialProjectionError::ProofVerificationFailed {
                reason: "missing proof step for non-carried path layer",
            },
        )?;

        if step.layer_index != layer_index {
            return Err(
                QuickChainTreeMaterialProjectionError::ProofVerificationFailed {
                    reason: "proof step layer does not match reconstructed path layer",
                },
            );
        }

        let (left_node_hash, right_node_hash) = match step.sibling_position {
            QuickChainTreeProofSiblingPositionV1::Left => {
                (step.sibling_node_hash.clone(), current_hash)
            }
            QuickChainTreeProofSiblingPositionV1::Right => {
                (current_hash, step.sibling_node_hash.clone())
            }
            _ => {
                return Err(
                    QuickChainTreeMaterialProjectionError::ProofVerificationFailed {
                        reason: "unsupported proof sibling position",
                    },
                );
            }
        };

        let branch = QuickChainTreeBranchNodeV1 {
            schema: QUICKCHAIN_TREE_BRANCH_NODE_SCHEMA.to_string(),
            version: QUICKCHAIN_DTO_VERSION,
            tree: proof.tree,
            layer_index,
            pair_index: (current_index / 2) as u32,
            left_node_hash,
            right_node_hash,
        };

        branch.validate().map_err(
            |error| QuickChainTreeMaterialProjectionError::InvalidProof {
                reason: error.to_string(),
            },
        )?;

        current_hash = hash_canonical_payload(hash_domain, &branch)?;
        current_index /= 2;
        current_count = current_count.div_ceil(2);
    }

    if steps.next().is_some() {
        return Err(
            QuickChainTreeMaterialProjectionError::ProofVerificationFailed {
                reason: "proof contains unused steps",
            },
        );
    }

    let root_payload = QuickChainTreeRootPayloadV1 {
        schema: QUICKCHAIN_TREE_ROOT_PAYLOAD_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: proof.chain_id.clone(),
        epoch_id: proof.epoch_id.clone(),
        tree: proof.tree,
        canonical_encoding: quickchain_tree_material_json_v1_encoding(),
        item_sort_rule: QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1.to_string(),
        pair_rule: QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1.to_string(),
        hash_domain: proof.hash_domain.clone(),
        source_items_count: proof.source_items_count,
        tree_height: proof.tree_height,
        root_node_hash: Some(current_hash),
    };

    root_payload.validate().map_err(|error| {
        QuickChainTreeMaterialProjectionError::InvalidProof {
            reason: error.to_string(),
        }
    })?;

    let recomputed_root_hash = hash_canonical_payload(hash_domain, &root_payload)?;

    if recomputed_root_hash != root.root_hash || recomputed_root_hash != proof.root_hash {
        return Err(
            QuickChainTreeMaterialProjectionError::ProofVerificationFailed {
                reason: "recomputed proof root does not match target root",
            },
        );
    }

    Ok(())
}

fn leaf_node_from_item(
    tree: QuickChainTreeMaterialKindV1,
    item: &QuickChainTreeMaterialItemV1,
) -> QuickChainTreeLeafNodeV1 {
    QuickChainTreeLeafNodeV1 {
        schema: QUICKCHAIN_TREE_LEAF_NODE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        tree,
        sort_key_hex: item.sort_key_hex.clone(),
        payload_schema: item.payload_schema.clone(),
        payload_hash: item.payload_hash.clone(),
    }
}

fn ensure_root_and_proof_metadata_match(
    root: &QuickChainTreeRootV1,
    proof: &QuickChainTreeInclusionProofV1,
) -> Result<(), QuickChainTreeMaterialProjectionError> {
    if root.chain_id != proof.chain_id
        || root.epoch_id != proof.epoch_id
        || root.tree != proof.tree
        || root.canonical_encoding != proof.canonical_encoding
        || root.item_sort_rule != proof.item_sort_rule
        || root.pair_rule != proof.pair_rule
        || root.hash_domain != proof.hash_domain
        || root.algorithm != proof.algorithm
        || root.source_items_count != proof.source_items_count
        || root.tree_height != proof.tree_height
        || root.root_hash != proof.root_hash
    {
        return Err(
            QuickChainTreeMaterialProjectionError::ProofVerificationFailed {
                reason: "root and proof metadata do not match",
            },
        );
    }

    Ok(())
}
