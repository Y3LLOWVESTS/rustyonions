//! RO:WHAT — Read-only Phase 3 passport-gated attestation authorization for QuickChain verifier artifacts.
//! RO:WHY — ECON/GOV: ron-ledger may evaluate deterministic eligibility but must not give validators balance authority.
//! RO:INTERACTS — ron-proto Phase 3 validator-set/capability DTOs and Phase 2 verifier attestations.
//! RO:INVARIANTS — read-only; deterministic; no clocks; no IO; no wallet mutation; no staking/slashing; no balance authority.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — authorization result is diagnostic and grants no spend, paid unlock, finality, bridge, or settlement authority.
//! RO:TEST — tests/quickchain_phase3_validator_gate.rs.

use ron_proto::quickchain::{
    QuickChainValidationError, QuickChainValidatorAuthorizationRejectionCodeV1,
    QuickChainValidatorAuthorizationRequestV1, QuickChainValidatorAuthorizationResultV1,
    QuickChainValidatorAuthorizationStatusV1, QuickChainValidatorLifecycleStatusV1,
    QUICKCHAIN_DTO_VERSION, QUICKCHAIN_VALIDATOR_AUTHORIZATION_RESULT_SCHEMA,
    QUICKCHAIN_VALIDATOR_CAPABILITY_SCOPE_VERIFY_V1,
};

/// Evaluate one Phase 3 passport-gated verifier attestation request.
///
/// This function performs deterministic shape and membership checks only. It
/// does not verify cryptographic signatures, read passports from a service,
/// mutate balances, produce finality, slash, stake, bridge, or settle.
pub fn evaluate_passport_gated_attestation_authorization(
    request: &QuickChainValidatorAuthorizationRequestV1,
) -> Result<QuickChainValidatorAuthorizationResultV1, QuickChainValidationError> {
    request.validate()?;

    let code = authorization_rejection_code(request);

    let status = if code.is_none() {
        QuickChainValidatorAuthorizationStatusV1::Authorized
    } else {
        QuickChainValidatorAuthorizationStatusV1::Rejected
    };

    let result = QuickChainValidatorAuthorizationResultV1 {
        schema: QUICKCHAIN_VALIDATOR_AUTHORIZATION_RESULT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: request.chain_id.clone(),
        epoch_id: request.epoch_id.clone(),
        validator_set_hash: request.validator_set_hash.clone(),
        validator_id: request.capability.validator_id.clone(),
        passport_subject: request.capability.passport_subject.clone(),
        key_id: request.capability.key_id.clone(),
        replay_result_hash: request.replay_result_hash.clone(),
        replay_status: request.replay_status,
        status,
        rejection_code: code,
    };

    result.validate()?;
    Ok(result)
}

fn authorization_rejection_code(
    request: &QuickChainValidatorAuthorizationRequestV1,
) -> Option<QuickChainValidatorAuthorizationRejectionCodeV1> {
    let set = &request.validator_set;
    let capability = &request.capability;
    let attestation = &request.attestation;

    if !set.passport_required {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::PassportRequired);
    }

    if set.bond_required {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::BondedEconomicsForbidden);
    }

    if set.validator_set_hash != request.validator_set_hash {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::ValidatorSetHashMismatch);
    }

    if capability.capability_scope != QUICKCHAIN_VALIDATOR_CAPABILITY_SCOPE_VERIFY_V1 {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::CapabilityScopeMismatch);
    }

    if attestation.replay_result_hash != request.replay_result_hash {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::ReplayResultHashMismatch);
    }

    if attestation.replay_status != request.replay_status {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::ReplayStatusMismatch);
    }

    let Some(member) = set
        .members
        .iter()
        .find(|member| member.validator_id == capability.validator_id)
    else {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::UnauthorizedIdentity);
    };

    if member.chain_id != request.chain_id
        || member.epoch_id != request.epoch_id
        || capability.chain_id != request.chain_id
        || capability.epoch_id != request.epoch_id
        || attestation.chain_id != request.chain_id
        || attestation.epoch_id != request.epoch_id
    {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::ChainEpochMismatch);
    }

    match member.lifecycle_status {
        QuickChainValidatorLifecycleStatusV1::Active => {}
        QuickChainValidatorLifecycleStatusV1::Revoked => {
            return Some(QuickChainValidatorAuthorizationRejectionCodeV1::RevokedValidator);
        }
        QuickChainValidatorLifecycleStatusV1::Expired => {
            return Some(QuickChainValidatorAuthorizationRejectionCodeV1::ExpiredCapability);
        }
        QuickChainValidatorLifecycleStatusV1::Pending => {
            return Some(QuickChainValidatorAuthorizationRejectionCodeV1::ValidatorNotActive);
        }

        _ => {
            return Some(QuickChainValidatorAuthorizationRejectionCodeV1::ValidatorNotActive);
        }
    }

    if request.evaluation_time_ms < member.not_before_ms
        || request.evaluation_time_ms < capability.not_before_ms
    {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::CapabilityNotYetValid);
    }

    if request.evaluation_time_ms >= member.expires_at_ms
        || request.evaluation_time_ms >= capability.expires_at_ms
    {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::ExpiredCapability);
    }

    if let Some(revoked_at_ms) = capability.revoked_at_ms {
        if revoked_at_ms <= request.evaluation_time_ms {
            return Some(QuickChainValidatorAuthorizationRejectionCodeV1::RevokedValidator);
        }
    }

    if member.passport_subject != capability.passport_subject {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::PassportSubjectMismatch);
    }

    if member.registry_entry_id != capability.registry_entry_id {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::RegistryEntryMismatch);
    }

    if member.capability_id != capability.capability_id {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::CapabilityMismatch);
    }

    if member.key_id != capability.key_id
        || attestation.key_id != capability.key_id
        || member.signature_algorithm != capability.signature_algorithm
        || attestation.signature_algorithm != capability.signature_algorithm
    {
        return Some(QuickChainValidatorAuthorizationRejectionCodeV1::KeyMismatch);
    }

    None
}
