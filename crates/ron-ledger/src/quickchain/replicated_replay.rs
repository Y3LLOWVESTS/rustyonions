//! RO:WHAT — Read-only QuickChain Phase 2 replay-bundle verification adapter.
//! RO:WHY — ECON/GOV: replicated verification must recompute Phase 1 artifacts without service IO or authority.
//! RO:INTERACTS — tree_material_projection, ron-proto verifier replay DTOs.
//! RO:INVARIANTS — read-only; deterministic; no clocks; no storage/network; no committee signing; no finality; no settlement mutation.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — replay results are diagnostic artifacts and grant no spend, receipt, bridge, or settlement authority.
//! RO:TEST — tests/quickchain_phase2_replicated_replay.rs.

use ron_proto::quickchain::{
    QuickChainCommitteeAttestationSetV1, QuickChainCommitteeDisagreementCodeV1,
    QuickChainCommitteeReadinessResultV1, QuickChainCommitteeReadinessStatusV1,
    QuickChainTreeMaterialKindV1, QuickChainTreeRootV1, QuickChainVerifierCheckStatusV1,
    QuickChainVerifierProofCheckV1, QuickChainVerifierReplayBundleV1,
    QuickChainVerifierReplayResultV1, QuickChainVerifierReplayStatusV1,
    QuickChainVerifierRootCheckV1, QUICKCHAIN_COMMITTEE_READINESS_RESULT_SCHEMA,
    QUICKCHAIN_DTO_VERSION, QUICKCHAIN_VERIFIER_PROOF_CHECK_SCHEMA,
    QUICKCHAIN_VERIFIER_REPLAY_ALGORITHM_READ_ONLY_ROOT_PROOF_V1,
    QUICKCHAIN_VERIFIER_REPLAY_RESULT_SCHEMA, QUICKCHAIN_VERIFIER_ROOT_CHECK_SCHEMA,
};

use super::{
    compute_tree_root_from_batch, verify_tree_inclusion_proof,
    QuickChainTreeMaterialProjectionError,
};

/// Replay one verifier bundle without IO, clocks, service calls, signing, quorum,
/// finality, or settlement mutation.
///
/// The function recomputes every expected root from the bundle's material batches
/// and checks every included proof against the corresponding expected root. A
/// mismatch returns a successful replay-result DTO with mismatch status; malformed
/// bundle/result DTOs return deterministic errors.
pub fn verify_replay_bundle_read_only(
    bundle: &QuickChainVerifierReplayBundleV1,
) -> Result<QuickChainVerifierReplayResultV1, QuickChainTreeMaterialProjectionError> {
    bundle.validate().map_err(
        |error| QuickChainTreeMaterialProjectionError::InvalidBatch {
            reason: format!("invalid verifier replay bundle: {error}"),
        },
    )?;

    let mut root_checks = Vec::with_capacity(bundle.expected_roots.len());
    let mut proof_checks = Vec::with_capacity(bundle.inclusion_proofs.len());
    let mut any_mismatch = false;

    for material in &bundle.material_batches {
        let expected_root = expected_root_for_tree(&bundle.expected_roots, material.tree)?;
        let recomputed_root = compute_tree_root_from_batch(material)?;

        let status = if expected_root.root_hash == recomputed_root.root_hash {
            QuickChainVerifierCheckStatusV1::Verified
        } else {
            any_mismatch = true;
            QuickChainVerifierCheckStatusV1::Mismatch
        };

        let detail = if status == QuickChainVerifierCheckStatusV1::Verified {
            None
        } else {
            Some("root hash mismatch".to_string())
        };

        root_checks.push(QuickChainVerifierRootCheckV1 {
            schema: QUICKCHAIN_VERIFIER_ROOT_CHECK_SCHEMA.to_string(),
            version: QUICKCHAIN_DTO_VERSION,
            chain_id: bundle.chain_id.clone(),
            epoch_id: bundle.epoch_id.clone(),
            tree: material.tree,
            expected_root_hash: expected_root.root_hash.clone(),
            recomputed_root_hash: recomputed_root.root_hash,
            status,
            detail,
        });
    }

    for proof in &bundle.inclusion_proofs {
        let expected_root = expected_root_for_tree(&bundle.expected_roots, proof.tree)?;

        let status = match verify_tree_inclusion_proof(expected_root, proof) {
            Ok(()) => QuickChainVerifierCheckStatusV1::Verified,
            Err(_) => {
                any_mismatch = true;
                QuickChainVerifierCheckStatusV1::Mismatch
            }
        };

        let detail = if status == QuickChainVerifierCheckStatusV1::Verified {
            None
        } else {
            Some("proof verification failed".to_string())
        };

        proof_checks.push(QuickChainVerifierProofCheckV1 {
            schema: QUICKCHAIN_VERIFIER_PROOF_CHECK_SCHEMA.to_string(),
            version: QUICKCHAIN_DTO_VERSION,
            chain_id: bundle.chain_id.clone(),
            epoch_id: bundle.epoch_id.clone(),
            tree: proof.tree,
            leaf_sort_key_hex: proof.leaf.sort_key_hex.clone(),
            root_hash: proof.root_hash.clone(),
            status,
            detail,
        });
    }

    let result = QuickChainVerifierReplayResultV1 {
        schema: QUICKCHAIN_VERIFIER_REPLAY_RESULT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: bundle.chain_id.clone(),
        epoch_id: bundle.epoch_id.clone(),
        replay_algorithm: QUICKCHAIN_VERIFIER_REPLAY_ALGORITHM_READ_ONLY_ROOT_PROOF_V1.to_string(),
        status: if any_mismatch {
            QuickChainVerifierReplayStatusV1::Mismatch
        } else {
            QuickChainVerifierReplayStatusV1::Verified
        },
        material_batches_count: bundle.material_batches.len() as u64,
        expected_roots_count: bundle.expected_roots.len() as u64,
        inclusion_proofs_count: bundle.inclusion_proofs.len() as u64,
        root_checks,
        proof_checks,
    };

    result.validate().map_err(
        |error| QuickChainTreeMaterialProjectionError::InvalidBatch {
            reason: format!("invalid verifier replay result: {error}"),
        },
    )?;

    Ok(result)
}

/// Evaluate a bounded committee attestation set for Phase 2 Round 2 readiness.
///
/// This performs deterministic DTO consistency/readiness evaluation only. It
/// does not verify signatures cryptographically, does not create finality, does
/// not choose forks, does not prune, and does not mutate ledger or wallet truth.
pub fn evaluate_committee_readiness_from_attestations(
    set: &QuickChainCommitteeAttestationSetV1,
) -> Result<QuickChainCommitteeReadinessResultV1, QuickChainTreeMaterialProjectionError> {
    set.validate().map_err(
        |error| QuickChainTreeMaterialProjectionError::InvalidBatch {
            reason: format!("invalid committee attestation set: {error}"),
        },
    )?;

    let accepted_attestations_count = u16::try_from(set.attestations.len()).map_err(|_| {
        QuickChainTreeMaterialProjectionError::InvalidBatch {
            reason: "attestation count exceeds u16 readiness DTO bounds".to_string(),
        }
    })?;

    let committee_members_count = u16::try_from(set.committee_members.len()).map_err(|_| {
        QuickChainTreeMaterialProjectionError::InvalidBatch {
            reason: "committee member count exceeds u16 readiness DTO bounds".to_string(),
        }
    })?;

    let (status, disagreement_code) = if accepted_attestations_count >= set.required_attestations {
        (QuickChainCommitteeReadinessStatusV1::Ready, None)
    } else {
        (
            QuickChainCommitteeReadinessStatusV1::NotReady,
            Some(QuickChainCommitteeDisagreementCodeV1::InsufficientAttestations),
        )
    };

    let result = QuickChainCommitteeReadinessResultV1 {
        schema: QUICKCHAIN_COMMITTEE_READINESS_RESULT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: set.chain_id.clone(),
        epoch_id: set.epoch_id.clone(),
        committee_algorithm: set.committee_algorithm.clone(),
        replay_result_hash: set.replay_result_hash.clone(),
        replay_status: set.replay_status,
        required_attestations: set.required_attestations,
        committee_members_count,
        accepted_attestations_count,
        status,
        disagreement_code,
    };

    result.validate().map_err(
        |error| QuickChainTreeMaterialProjectionError::InvalidBatch {
            reason: format!("invalid committee readiness result: {error}"),
        },
    )?;

    Ok(result)
}

fn expected_root_for_tree(
    expected_roots: &[QuickChainTreeRootV1],
    tree: QuickChainTreeMaterialKindV1,
) -> Result<&QuickChainTreeRootV1, QuickChainTreeMaterialProjectionError> {
    expected_roots
        .iter()
        .find(|root| root.tree == tree)
        .ok_or_else(|| QuickChainTreeMaterialProjectionError::InvalidBatch {
            reason: "missing expected root for replay material tree".to_string(),
        })
}
