//! RO:WHAT — Read-only Phase 3 Round 2 validator lifecycle hardening evaluators for QuickChain artifacts.
//! RO:WHY — ECON/GOV: ron-ledger may deterministically inspect lifecycle artifacts without giving validators balance authority.
//! RO:INTERACTS — ron-proto Phase 3 validator lifecycle DTOs, validator sets, and replay evidence.
//! RO:INVARIANTS — read-only; deterministic; no clocks; no IO; no wallet mutation; no staking/slashing; no balance authority.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — decisions are diagnostic and grant no spend, paid unlock, finality, bridge, staking, slashing, or settlement authority.
//! RO:TEST — tests/quickchain_phase3_validator_lifecycle.rs.

use ron_proto::quickchain::{
    QuickChainValidationError, QuickChainValidatorEquivocationEvidenceV1,
    QuickChainValidatorLifecycleDecisionStatusV1, QuickChainValidatorLifecycleDecisionV1,
    QuickChainValidatorLifecycleOperationV1, QuickChainValidatorLifecycleRejectionCodeV1,
    QuickChainValidatorLifecycleStatusV1, QuickChainValidatorRevocationV1,
    QuickChainValidatorRotationV1, QuickChainValidatorSetV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_VALIDATOR_LIFECYCLE_DECISION_SCHEMA,
};

/// Evaluate a validator key/capability rotation artifact against one passport-gated set.
///
/// This is a read-only deterministic check. It does not mutate the validator set,
/// rotate keys in a registry, grant authority, stake, slash, bridge, or settle.
pub fn evaluate_validator_rotation_read_only(
    validator_set: &QuickChainValidatorSetV1,
    rotation: &QuickChainValidatorRotationV1,
) -> Result<QuickChainValidatorLifecycleDecisionV1, QuickChainValidationError> {
    validator_set.validate()?;
    rotation.validate()?;

    let rejection_code = rotation_rejection_code(validator_set, rotation);

    lifecycle_decision(
        &rotation.chain_id,
        &rotation.epoch_id,
        rotation.validator_set_hash.clone(),
        &rotation.validator_id,
        QuickChainValidatorLifecycleOperationV1::RotateValidator,
        rejection_code,
    )
}

/// Evaluate a validator revocation artifact against one passport-gated set.
///
/// This is a read-only deterministic check. It does not mutate the validator set,
/// revoke a live passport, grant authority, stake, slash, bridge, or settle.
pub fn evaluate_validator_revocation_read_only(
    validator_set: &QuickChainValidatorSetV1,
    revocation: &QuickChainValidatorRevocationV1,
) -> Result<QuickChainValidatorLifecycleDecisionV1, QuickChainValidationError> {
    validator_set.validate()?;
    revocation.validate()?;

    let rejection_code = revocation_rejection_code(validator_set, revocation);

    lifecycle_decision(
        &revocation.chain_id,
        &revocation.epoch_id,
        revocation.validator_set_hash.clone(),
        &revocation.validator_id,
        QuickChainValidatorLifecycleOperationV1::RevokeValidator,
        rejection_code,
    )
}

/// Evaluate split-brain / double-attestation evidence against one passport-gated set.
///
/// This records only whether evidence shape and membership are coherent. It does
/// not slash, punish, finalize, bridge, or mutate balances.
pub fn evaluate_validator_equivocation_evidence_read_only(
    validator_set: &QuickChainValidatorSetV1,
    evidence: &QuickChainValidatorEquivocationEvidenceV1,
) -> Result<QuickChainValidatorLifecycleDecisionV1, QuickChainValidationError> {
    validator_set.validate()?;
    evidence.validate()?;

    let rejection_code = equivocation_rejection_code(validator_set, evidence);

    lifecycle_decision(
        &evidence.chain_id,
        &evidence.epoch_id,
        evidence.validator_set_hash.clone(),
        &evidence.validator_id,
        QuickChainValidatorLifecycleOperationV1::RecordEquivocationEvidence,
        rejection_code,
    )
}

fn rotation_rejection_code(
    validator_set: &QuickChainValidatorSetV1,
    rotation: &QuickChainValidatorRotationV1,
) -> Option<QuickChainValidatorLifecycleRejectionCodeV1> {
    if let Some(code) = common_set_rejection_code(
        validator_set,
        &rotation.chain_id,
        &rotation.epoch_id,
        &rotation.validator_set_hash,
    ) {
        return Some(code);
    }

    let Some(member) = validator_set
        .members
        .iter()
        .find(|member| member.validator_id == rotation.validator_id)
    else {
        return Some(QuickChainValidatorLifecycleRejectionCodeV1::UnknownValidator);
    };

    if let Some(code) = member_active_rejection_code(member.lifecycle_status) {
        return Some(code);
    }

    if member.passport_subject != rotation.passport_subject {
        return Some(QuickChainValidatorLifecycleRejectionCodeV1::PassportSubjectMismatch);
    }

    if member.key_id != rotation.old_key_id {
        return Some(QuickChainValidatorLifecycleRejectionCodeV1::KeyMismatch);
    }

    if member.capability_id != rotation.old_capability_id {
        return Some(QuickChainValidatorLifecycleRejectionCodeV1::CapabilityMismatch);
    }

    None
}

fn revocation_rejection_code(
    validator_set: &QuickChainValidatorSetV1,
    revocation: &QuickChainValidatorRevocationV1,
) -> Option<QuickChainValidatorLifecycleRejectionCodeV1> {
    if let Some(code) = common_set_rejection_code(
        validator_set,
        &revocation.chain_id,
        &revocation.epoch_id,
        &revocation.validator_set_hash,
    ) {
        return Some(code);
    }

    let Some(member) = validator_set
        .members
        .iter()
        .find(|member| member.validator_id == revocation.validator_id)
    else {
        return Some(QuickChainValidatorLifecycleRejectionCodeV1::UnknownValidator);
    };

    if let Some(code) = member_active_rejection_code(member.lifecycle_status) {
        return Some(code);
    }

    if member.passport_subject != revocation.passport_subject {
        return Some(QuickChainValidatorLifecycleRejectionCodeV1::PassportSubjectMismatch);
    }

    if member.key_id != revocation.key_id {
        return Some(QuickChainValidatorLifecycleRejectionCodeV1::KeyMismatch);
    }

    if member.capability_id != revocation.capability_id {
        return Some(QuickChainValidatorLifecycleRejectionCodeV1::CapabilityMismatch);
    }

    None
}

fn equivocation_rejection_code(
    validator_set: &QuickChainValidatorSetV1,
    evidence: &QuickChainValidatorEquivocationEvidenceV1,
) -> Option<QuickChainValidatorLifecycleRejectionCodeV1> {
    if let Some(code) = common_set_rejection_code(
        validator_set,
        &evidence.chain_id,
        &evidence.epoch_id,
        &evidence.validator_set_hash,
    ) {
        return Some(code);
    }

    let Some(member) = validator_set
        .members
        .iter()
        .find(|member| member.validator_id == evidence.validator_id)
    else {
        return Some(QuickChainValidatorLifecycleRejectionCodeV1::UnknownValidator);
    };

    if let Some(code) = member_active_rejection_code(member.lifecycle_status) {
        return Some(code);
    }

    if member.key_id != evidence.key_id {
        return Some(QuickChainValidatorLifecycleRejectionCodeV1::KeyMismatch);
    }

    None
}

fn common_set_rejection_code(
    validator_set: &QuickChainValidatorSetV1,
    chain_id: &str,
    epoch_id: &str,
    validator_set_hash: &ron_proto::ContentId,
) -> Option<QuickChainValidatorLifecycleRejectionCodeV1> {
    if validator_set.validator_set_hash != *validator_set_hash {
        return Some(QuickChainValidatorLifecycleRejectionCodeV1::ValidatorSetHashMismatch);
    }

    if validator_set.chain_id != chain_id || validator_set.epoch_id != epoch_id {
        return Some(QuickChainValidatorLifecycleRejectionCodeV1::ChainEpochMismatch);
    }

    if validator_set.bond_required {
        return Some(QuickChainValidatorLifecycleRejectionCodeV1::BondedEconomicsForbidden);
    }

    None
}

fn member_active_rejection_code(
    status: QuickChainValidatorLifecycleStatusV1,
) -> Option<QuickChainValidatorLifecycleRejectionCodeV1> {
    match status {
        QuickChainValidatorLifecycleStatusV1::Active => None,
        QuickChainValidatorLifecycleStatusV1::Revoked => {
            Some(QuickChainValidatorLifecycleRejectionCodeV1::RevokedValidator)
        }
        QuickChainValidatorLifecycleStatusV1::Expired => {
            Some(QuickChainValidatorLifecycleRejectionCodeV1::ExpiredValidator)
        }
        QuickChainValidatorLifecycleStatusV1::Pending => {
            Some(QuickChainValidatorLifecycleRejectionCodeV1::ValidatorNotActive)
        }
        _ => Some(QuickChainValidatorLifecycleRejectionCodeV1::ValidatorNotActive),
    }
}

fn lifecycle_decision(
    chain_id: &str,
    epoch_id: &str,
    validator_set_hash: ron_proto::ContentId,
    validator_id: &str,
    operation: QuickChainValidatorLifecycleOperationV1,
    rejection_code: Option<QuickChainValidatorLifecycleRejectionCodeV1>,
) -> Result<QuickChainValidatorLifecycleDecisionV1, QuickChainValidationError> {
    let status = if rejection_code.is_some() {
        QuickChainValidatorLifecycleDecisionStatusV1::Rejected
    } else {
        QuickChainValidatorLifecycleDecisionStatusV1::Accepted
    };

    let decision = QuickChainValidatorLifecycleDecisionV1 {
        schema: QUICKCHAIN_VALIDATOR_LIFECYCLE_DECISION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: chain_id.to_string(),
        epoch_id: epoch_id.to_string(),
        validator_set_hash,
        validator_id: validator_id.to_string(),
        operation,
        status,
        rejection_code,
    };

    decision.validate()?;
    Ok(decision)
}
