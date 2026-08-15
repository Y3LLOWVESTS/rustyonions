//! RO:WHAT — Builds deterministic unsigned QuickChain committee checkpoint candidates from reviewed roots and commitments.
//! RO:WHY — ECON/RES: Phase 19 needs one reproducible checkpoint hash before validator signing or finality can begin.
//! RO:INTERACTS — ron-proto committee checkpoint DTO/canonical JSON, deterministic state roots, typed ledger-sequence receipt roots.
//! RO:INVARIANTS — canonical JSON; checkpoint domain unchanged; typed receipt provenance; no IO, clocks, signing, quorum, finality, wallet, or ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none; available only through quickchain-preflight.
//! RO:SECURITY — candidate hashes are pre-signature artifacts and grant no spend, settlement, validator, bridge, or finality authority.
//! RO:TEST — tests/final_beta_phase19_checkpoint_candidate.rs.

use ron_proto::{
    quickchain::{
        to_canonical_json_vec, QuickChainCanonicalEncodingV1,
        QuickChainCommitteeCheckpointPayloadV1, QuickChainConservationV1,
        QuickChainReceiptRootSchemeV1, QuickChainSettlementModeV1, QuickChainStateRootSchemeV1,
        QuickChainSupplyDeltaV1, QuickChainTreeMaterialKindV1, QuickChainTreeRootV1,
        QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1, QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA,
        QUICKCHAIN_DTO_VERSION,
    },
    ContentId,
};
use thiserror::Error;

use super::QuickChainLedgerSequenceReceiptRoot;

/// Explicit deterministic inputs required to assemble one unsigned committee
/// checkpoint candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainCommitteeCheckpointCandidateContext {
    /// QuickChain network identifier committed by this candidate.
    pub chain_id: String,
    /// Non-zero checkpoint height.
    pub height: u64,
    /// Epoch whose transition is committed by this candidate.
    pub epoch_id: String,
    /// Execution specification version used for deterministic replay.
    pub execution_spec_version: String,
    /// Hash of the checkpoint immediately preceding this candidate.
    pub previous_checkpoint_hash: ContentId,
    /// Reviewed state root before the candidate transition.
    pub previous_state_root: QuickChainTreeRootV1,
    /// Reviewed state root after the candidate transition.
    pub new_state_root: QuickChainTreeRootV1,
    /// Typed ledger-sequence receipt root for the candidate epoch.
    pub receipt_root: QuickChainLedgerSequenceReceiptRoot,
    /// Deterministic accounting snapshot commitment.
    pub accounting_snapshot_root: ContentId,
    /// Deterministic reward-manifest commitment.
    pub reward_manifest_root: ContentId,
    /// Data-availability commitment required by the committee payload.
    pub data_availability_root: ContentId,
    /// Reviewed policy commitment governing this transition.
    pub policy_hash: ContentId,
    /// Validator-set commitment for the candidate committee.
    pub validator_set_hash: ContentId,
    /// Chain-parameter commitment used by deterministic validation.
    pub chain_params_hash: ContentId,
    /// Supply delta committed by the candidate payload.
    pub supply_delta: QuickChainSupplyDeltaV1,
    /// Conservation proof summary committed by the candidate payload.
    pub conservation: QuickChainConservationV1,
    /// Epoch transition start timestamp in milliseconds.
    pub started_at_ms: u64,
    /// Epoch transition end timestamp in milliseconds.
    pub ended_at_ms: u64,
    /// Candidate production timestamp in milliseconds.
    pub produced_at_ms: u64,
}

/// Pre-signature checkpoint artifact containing only the validated payload,
/// canonical payload bytes, and deterministic checkpoint hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainCommitteeCheckpointCandidate {
    payload: QuickChainCommitteeCheckpointPayloadV1,
    canonical_payload_bytes: Vec<u8>,
    checkpoint_hash: ContentId,
}

impl QuickChainCommitteeCheckpointCandidate {
    /// Borrow the validated unsigned committee checkpoint payload.
    #[must_use]
    pub fn payload(&self) -> &QuickChainCommitteeCheckpointPayloadV1 {
        &self.payload
    }

    /// Borrow the exact canonical bytes committed by the checkpoint hash.
    #[must_use]
    pub fn canonical_payload_bytes(&self) -> &[u8] {
        &self.canonical_payload_bytes
    }

    /// Borrow the deterministic unsigned checkpoint candidate hash.
    #[must_use]
    pub fn checkpoint_hash(&self) -> &ContentId {
        &self.checkpoint_hash
    }
}

/// Deterministic checkpoint-candidate assembly failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum QuickChainCommitteeCheckpointCandidateError {
    #[error("invalid {field}: {reason}")]
    /// A supplied reviewed root failed its own validation rules.
    InvalidRoot {
        /// Candidate input field containing the invalid root.
        field: &'static str,
        /// Validation failure returned by the root artifact.
        reason: String,
    },

    #[error("{field} must be a {expected} root")]
    /// A supplied tree root represents the wrong material kind.
    WrongRootKind {
        /// Candidate input field containing the mismatched root.
        field: &'static str,
        /// Required tree material kind for that field.
        expected: &'static str,
    },

    #[error("{field} chain_id must match candidate chain_id")]
    /// A supplied root is bound to a different chain.
    RootChainMismatch {
        /// Candidate input field whose chain binding mismatched.
        field: &'static str,
    },

    #[error("{field} epoch_id must match candidate epoch_id")]
    /// A supplied current-epoch root is bound to a different epoch.
    RootEpochMismatch {
        /// Candidate input field whose epoch binding mismatched.
        field: &'static str,
    },

    #[error("invalid committee checkpoint payload: {reason}")]
    /// The assembled committee checkpoint payload failed validation.
    InvalidPayload {
        /// Payload validation failure.
        reason: String,
    },

    #[error("committee checkpoint payload canonicalization failed: {reason}")]
    /// Canonical JSON generation failed for the validated payload.
    CanonicalizationFailed {
        /// Canonicalization failure.
        reason: String,
    },

    #[error("computed committee checkpoint hash is invalid: {reason}")]
    /// The locally computed BLAKE3 checkpoint hash could not form a valid content ID.
    InvalidComputedHash {
        /// Content-ID construction failure.
        reason: String,
    },
}

/// Assemble one deterministic unsigned committee checkpoint candidate.
///
/// Schema, DTO version, canonical encoding, root schemes, and settlement mode
/// are fixed to the reviewed V1 values rather than caller-controlled switches.
///
/// The current receipt root must carry the typed ledger-sequence provenance
/// produced by `compute_ledger_sequence_receipt_root`. The new state root and
/// receipt root must bind the same chain and epoch as the candidate.
///
/// This function performs no IO, clock reads, validator signing, quorum
/// decision, checkpoint finalization, wallet mutation, or ledger mutation.
pub fn build_committee_checkpoint_candidate(
    context: QuickChainCommitteeCheckpointCandidateContext,
) -> Result<QuickChainCommitteeCheckpointCandidate, QuickChainCommitteeCheckpointCandidateError> {
    validate_state_root(
        "previous_state_root",
        &context.previous_state_root,
        &context.chain_id,
        None,
    )?;

    validate_state_root(
        "new_state_root",
        &context.new_state_root,
        &context.chain_id,
        Some(&context.epoch_id),
    )?;

    validate_receipt_root(&context.receipt_root, &context.chain_id, &context.epoch_id)?;

    let payload = QuickChainCommitteeCheckpointPayloadV1 {
        schema: QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: context.chain_id,
        height: context.height,
        epoch_id: context.epoch_id,
        execution_spec_version: context.execution_spec_version,
        previous_checkpoint_hash: context.previous_checkpoint_hash,
        previous_state_root: context.previous_state_root.root_hash,
        new_state_root: context.new_state_root.root_hash,
        receipt_root: context.receipt_root.into_root().root_hash,
        accounting_snapshot_root: context.accounting_snapshot_root,
        reward_manifest_root: context.reward_manifest_root,
        data_availability_root: context.data_availability_root,
        policy_hash: context.policy_hash,
        validator_set_hash: context.validator_set_hash,
        chain_params_hash: context.chain_params_hash,
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        state_root_scheme: QuickChainStateRootSchemeV1::SortedMerkleMapV1,
        receipt_root_scheme: QuickChainReceiptRootSchemeV1::LedgerSequenceMerkleV1,
        supply_delta: context.supply_delta,
        conservation: context.conservation,
        settlement_mode: QuickChainSettlementModeV1::LocalRoot,
        started_at_ms: context.started_at_ms,
        ended_at_ms: context.ended_at_ms,
        produced_at_ms: context.produced_at_ms,
    };

    payload.validate().map_err(|error| {
        QuickChainCommitteeCheckpointCandidateError::InvalidPayload {
            reason: error.to_string(),
        }
    })?;

    let canonical_payload_bytes = to_canonical_json_vec(&payload).map_err(|error| {
        QuickChainCommitteeCheckpointCandidateError::CanonicalizationFailed {
            reason: error.to_string(),
        }
    })?;

    let checkpoint_hash = hash_checkpoint_candidate(&canonical_payload_bytes)?;

    Ok(QuickChainCommitteeCheckpointCandidate {
        payload,
        canonical_payload_bytes,
        checkpoint_hash,
    })
}

fn validate_state_root(
    field: &'static str,
    root: &QuickChainTreeRootV1,
    chain_id: &str,
    epoch_id: Option<&str>,
) -> Result<(), QuickChainCommitteeCheckpointCandidateError> {
    root.validate().map_err(
        |error| QuickChainCommitteeCheckpointCandidateError::InvalidRoot {
            field,
            reason: error.to_string(),
        },
    )?;

    if root.tree != QuickChainTreeMaterialKindV1::State {
        return Err(QuickChainCommitteeCheckpointCandidateError::WrongRootKind {
            field,
            expected: "state",
        });
    }

    if root.chain_id != chain_id {
        return Err(QuickChainCommitteeCheckpointCandidateError::RootChainMismatch { field });
    }

    if let Some(expected_epoch_id) = epoch_id {
        if root.epoch_id != expected_epoch_id {
            return Err(QuickChainCommitteeCheckpointCandidateError::RootEpochMismatch { field });
        }
    }

    Ok(())
}

fn validate_receipt_root(
    receipt_root: &QuickChainLedgerSequenceReceiptRoot,
    chain_id: &str,
    epoch_id: &str,
) -> Result<(), QuickChainCommitteeCheckpointCandidateError> {
    let root = receipt_root.root();

    root.validate().map_err(
        |error| QuickChainCommitteeCheckpointCandidateError::InvalidRoot {
            field: "receipt_root",
            reason: error.to_string(),
        },
    )?;

    if root.tree != QuickChainTreeMaterialKindV1::Receipts {
        return Err(QuickChainCommitteeCheckpointCandidateError::WrongRootKind {
            field: "receipt_root",
            expected: "receipts",
        });
    }

    if root.chain_id != chain_id {
        return Err(
            QuickChainCommitteeCheckpointCandidateError::RootChainMismatch {
                field: "receipt_root",
            },
        );
    }

    if root.epoch_id != epoch_id {
        return Err(
            QuickChainCommitteeCheckpointCandidateError::RootEpochMismatch {
                field: "receipt_root",
            },
        );
    }

    Ok(())
}

fn hash_checkpoint_candidate(
    canonical_payload_bytes: &[u8],
) -> Result<ContentId, QuickChainCommitteeCheckpointCandidateError> {
    let mut framed = Vec::with_capacity(
        QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1.len() + 1 + canonical_payload_bytes.len(),
    );

    framed.extend_from_slice(QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1.as_bytes());
    framed.push(0x00);
    framed.extend_from_slice(canonical_payload_bytes);

    let digest = blake3::hash(&framed).to_hex().to_string();

    format!("b3:{digest}")
        .parse::<ContentId>()
        .map_err(
            |error| QuickChainCommitteeCheckpointCandidateError::InvalidComputedHash {
                reason: error.to_string(),
            },
        )
}
