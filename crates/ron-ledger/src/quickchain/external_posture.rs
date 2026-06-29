//! RO:WHAT — Read-only QuickChain Phase 5 Round 3 chosen external posture verification helpers.
//! RO:WHY — ECON/GOV: ledger can observe the chosen anchor-only posture without treating external evidence as ROC truth.
//! RO:INTERACTS — ron-proto external posture DTOs and Phase 5 anchor evidence references.
//! RO:INVARIANTS — no IO, clocks, services, balance mutation, receipt creation, paid unlock, bridge behavior, or external truth.
//! RO:METRICS — none.
//! RO:CONFIG — none; available only through quickchain-preflight.
//! RO:SECURITY — evidence/status verification grants no spend, unlock, finality, or settlement authority.
//! RO:TEST — tests/quickchain_phase5_external_posture.rs.

use ron_proto::{
    QuickChainExternalIntegrationPostureV1, QuickChainExternalPostureDecisionV1,
    QuickChainExternalPostureVerificationV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_EXTERNAL_POSTURE_VERIFICATION_SCHEMA,
};
use thiserror::Error;

/// Failure while verifying the chosen external posture as read-only evidence.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QuickChainExternalPostureBoundaryError {
    /// External posture decision DTO failed strict validation before verification.
    #[error("invalid QuickChain external posture decision: {reason}")]
    InvalidDecision {
        /// Bounded validation reason.
        reason: String,
    },

    /// External posture verification DTO failed strict validation after assembly.
    #[error("invalid QuickChain external posture verification: {reason}")]
    InvalidVerification {
        /// Bounded validation reason.
        reason: String,
    },
}

/// Verify a Phase 5 Round 3 external posture decision as read-only evidence.
///
/// This helper deliberately does not read external systems, write artifacts,
/// create receipts, unlock paid content, mutate balances, or convert anchor
/// evidence into ROC truth. It only validates the strict DTO and mirrors a
/// bounded evidence-only verification artifact.
pub fn verify_external_posture_decision_read_only(
    decision: &QuickChainExternalPostureDecisionV1,
) -> Result<QuickChainExternalPostureVerificationV1, QuickChainExternalPostureBoundaryError> {
    decision.validate().map_err(|error| {
        QuickChainExternalPostureBoundaryError::InvalidDecision {
            reason: error.to_string(),
        }
    })?;

    let verification = QuickChainExternalPostureVerificationV1 {
        schema: QUICKCHAIN_EXTERNAL_POSTURE_VERIFICATION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: decision.chain_id.clone(),
        epoch_id: decision.epoch_id.clone(),
        posture_id: decision.posture_id.clone(),
        chosen_posture: QuickChainExternalIntegrationPostureV1::AnchorOnly,
        anchor_commitment_hash: decision.anchor_commitment_hash.clone(),
        observed_evidence_ref: decision.evidence_ref.clone(),
        single_path_selected: true,
        verified_evidence_only: true,
        wallet_ledger_truth_canonical: true,
        balance_mutation_detected: false,
        receipt_authority_detected: false,
        paid_unlock_detected: false,
        external_settlement_detected: false,
        bridge_detected: false,
        rox_solana_runtime_detected: false,
        exchange_facing_detected: false,
        liquidity_detected: false,
        staking_detected: false,
    };

    verification.validate().map_err(|error| {
        QuickChainExternalPostureBoundaryError::InvalidVerification {
            reason: error.to_string(),
        }
    })?;

    Ok(verification)
}
