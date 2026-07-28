//! RO:WHAT — Native Passport replay and challenge-consumption contract.
//! RO:WHY — P3 Identity & Keys. Adds a contract-only review that binds accepted verification envelopes to replay keys and future challenge-consumption decisions.
//! RO:INTERACTS — Phase 10B proof-verification adapter envelopes and future replay/challenge state adapters.
//! RO:INVARIANTS — requires accepted verification evidence, exact binding to challenge/proof/request labels, unseen replay posture, unconsumed challenge posture, replay key hash, idempotency key hash, signer label, authority, algorithm, transcript hash, and timing. This phase does not mutate replay stores, consume challenges, consume/issue capabilities, add routes, perform runtime I/O, mutate storage, touch wallet/ledger state, or access secrets.
//! RO:TEST — tests/native_passport_phase11a_replay_and_challenge_consumption_contract.rs.

use super::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, NativePassportProofAuthority, NativePassportProofKind,
    NativeProofSignatureAlgorithm, NativeProofSignedPayloadHex, NativeProofSigningOperationKind,
    NativeProofVerificationAdapterEnvelopeV1, PassportIdV1,
    PHASE10B_PROOF_VERIFICATION_ADAPTER_DOMAIN, PHASE10B_PROOF_VERIFICATION_ADAPTER_VERSION,
    PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_PROOF_CONTRACT_DOMAIN,
    PHASE8B_PROOF_CONTRACT_VERSION, PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
    PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN, PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
};

pub const NATIVE_PASSPORT_PHASE11A_LABEL: &str =
    "NATIVE_PASSPORT_PHASE11A_REPLAY_AND_CHALLENGE_CONSUMPTION_CONTRACT";

pub const PHASE11A_REPLAY_CHALLENGE_CONSUMPTION_CONTRACT_DOMAIN: &str =
    "native-passport/replay-challenge-consumption-contract/v1";

pub const PHASE11A_REPLAY_CHALLENGE_CONSUMPTION_CONTRACT_VERSION: u16 = 1;

pub const PHASE11A_FORBIDDEN_CONSUMPTION_AUTHORITY_FLAGS: &[&str] = &[
    "replay_store_mutation",
    "challenge_consumption_execution",
    "capability_consumption",
    "capability_issuance",
    "capability_refresh",
    "capability_revocation",
    "secret_key_access",
    "key_loading",
    "key_derivation_runtime",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_unseal",
    "runtime_io",
    "route_added",
    "storage_mutation",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReplayChallengeConsumptionContractDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub operation_kind: NativeProofSigningOperationKind,
    pub expected_authority: NativePassportProofAuthority,
    pub challenge_id: ChallengeIdV1,
    pub passport_id: Option<PassportIdV1>,
    pub device_id: Option<DeviceIdV1>,
    pub proof_contract_domain: &'static str,
    pub proof_contract_version: u16,
    pub request_contract_domain: Option<&'static str>,
    pub request_contract_version: Option<u16>,
    pub proof_kind: NativePassportProofKind,
    pub transcript_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub expected_signature_algorithm: NativeProofSignatureAlgorithm,
    pub signed_payload_hex: NativeProofSignedPayloadHex,
    pub verification_evidence_label: &'static str,
    pub verifier_public_key_label: &'static str,
    pub replay_key_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub consumption_requested_at_ms: u64,
    pub expects_verified_payload_accepted: bool,
    pub expects_replay_state_unseen: bool,
    pub expects_challenge_unconsumed: bool,
    pub contract_only: bool,
    pub requests_replay_store_mutation: bool,
    pub requests_challenge_consumption_execution: bool,
    pub requests_capability_consumption: bool,
    pub requests_capability_issuance: bool,
    pub requests_capability_lifecycle_runtime: bool,
    pub requests_secret_key_access: bool,
    pub requests_key_loading: bool,
    pub requests_key_derivation_runtime: bool,
    pub requests_vault_unlock: bool,
    pub includes_vault_runtime: bool,
    pub requests_platform_sealer_unseal: bool,
    pub requests_runtime_io: bool,
    pub adds_routes: bool,
    pub requests_storage_mutation: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReplayChallengeConsumptionContractDecisionV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub operation_kind: NativeProofSigningOperationKind,
    pub reviewed_authority: NativePassportProofAuthority,
    pub challenge_id: ChallengeIdV1,
    pub passport_id: Option<PassportIdV1>,
    pub device_id: Option<DeviceIdV1>,
    pub proof_contract_domain: &'static str,
    pub proof_contract_version: u16,
    pub request_contract_domain: Option<&'static str>,
    pub request_contract_version: Option<u16>,
    pub proof_kind: NativePassportProofKind,
    pub transcript_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub expected_signature_algorithm: NativeProofSignatureAlgorithm,
    pub signed_payload_hex: NativeProofSignedPayloadHex,
    pub verification_evidence_label: &'static str,
    pub verifier_public_key_label: &'static str,
    pub replay_key_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub consumption_requested_at_ms: u64,
    pub replay_state_reviewed_unseen: bool,
    pub challenge_reviewed_unconsumed: bool,
    pub replay_store_mutated: bool,
    pub challenge_consumed: bool,
    pub capability_state_changed: bool,
    pub runtime_io_performed: bool,
    pub wallet_or_ledger_mutated: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeReplayChallengeConsumptionContractReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    VerificationEnvelopeDomainMismatch,
    VerificationEnvelopeVersionMismatch,
    VerificationEnvelopeNotAccepted,
    VerificationEnvelopeAlreadyMutatedState,
    OperationMismatch,
    AuthorityMismatch,
    ChallengeIdMismatch,
    PassportBindingMismatch,
    DeviceBindingMismatch,
    ProofContractDomainMismatch,
    ProofContractVersionMismatch,
    RequestProofContractDomainMismatch,
    RequestProofContractVersionMismatch,
    ProofKindMismatch,
    TranscriptHashMismatch,
    MissingSignerPublicKeyLabel,
    SignerPublicKeyLabelMismatch,
    SignatureAlgorithmMismatch,
    SignedPayloadMismatch,
    MissingVerificationEvidenceLabel,
    VerificationEvidenceLabelMismatch,
    MissingVerifierPublicKeyLabel,
    VerifierPublicKeyLabelMismatch,
    ReplayStateAlreadySeen,
    ChallengeAlreadyConsumed,
    InvalidConsumptionTime,
    ConsumptionContractNotContractOnly,
    UnsafeConsumptionAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReplayChallengeConsumptionContractPosture {
    pub phase_label: &'static str,
    pub replay_challenge_consumption_contract_added: bool,
    pub replay_key_binding_added: bool,
    pub challenge_consumption_review_added: bool,
    pub replay_store_mutation_added: bool,
    pub challenge_consumption_execution_added: bool,
    pub capability_consumption_added: bool,
    pub capability_issuance_added: bool,
    pub capability_lifecycle_runtime_added: bool,
    pub secret_key_access_added: bool,
    pub key_loading_added: bool,
    pub key_derivation_runtime_added: bool,
    pub vault_unlock_added: bool,
    pub vault_runtime_added: bool,
    pub platform_sealer_unseal_added: bool,
    pub runtime_io_added: bool,
    pub routes_added: bool,
    pub storage_mutation_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub runtime_authority_changed: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_replay_challenge_consumption_contract_posture(
) -> NativeReplayChallengeConsumptionContractPosture {
    NativeReplayChallengeConsumptionContractPosture {
        phase_label: NATIVE_PASSPORT_PHASE11A_LABEL,
        replay_challenge_consumption_contract_added: true,
        replay_key_binding_added: true,
        challenge_consumption_review_added: true,
        replay_store_mutation_added: false,
        challenge_consumption_execution_added: false,
        capability_consumption_added: false,
        capability_issuance_added: false,
        capability_lifecycle_runtime_added: false,
        secret_key_access_added: false,
        key_loading_added: false,
        key_derivation_runtime_added: false,
        vault_unlock_added: false,
        vault_runtime_added: false,
        platform_sealer_unseal_added: false,
        runtime_io_added: false,
        routes_added: false,
        storage_mutation_added: false,
        wallet_or_ledger_mutation_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE11A_FORBIDDEN_CONSUMPTION_AUTHORITY_FLAGS,
    }
}

pub fn review_native_replay_challenge_consumption_contract(
    envelope: &NativeProofVerificationAdapterEnvelopeV1,
    draft: NativeReplayChallengeConsumptionContractDraftV1,
) -> Result<
    NativeReplayChallengeConsumptionContractDecisionV1,
    NativeReplayChallengeConsumptionContractReviewError,
> {
    validate_verification_envelope(envelope)?;
    validate_draft_against_envelope(envelope, &draft)?;
    validate_no_unsafe_flags(&draft)?;

    Ok(NativeReplayChallengeConsumptionContractDecisionV1 {
        contract_domain: PHASE11A_REPLAY_CHALLENGE_CONSUMPTION_CONTRACT_DOMAIN,
        contract_version: PHASE11A_REPLAY_CHALLENGE_CONSUMPTION_CONTRACT_VERSION,
        operation_kind: draft.operation_kind,
        reviewed_authority: draft.expected_authority,
        challenge_id: draft.challenge_id,
        passport_id: draft.passport_id,
        device_id: draft.device_id,
        proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
        proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
        request_contract_domain: draft.request_contract_domain,
        request_contract_version: draft.request_contract_version,
        proof_kind: draft.proof_kind,
        transcript_hash: draft.transcript_hash,
        signer_public_key_label: draft.signer_public_key_label,
        expected_signature_algorithm: draft.expected_signature_algorithm,
        signed_payload_hex: draft.signed_payload_hex,
        verification_evidence_label: draft.verification_evidence_label,
        verifier_public_key_label: draft.verifier_public_key_label,
        replay_key_hash: draft.replay_key_hash,
        idempotency_key_hash: draft.idempotency_key_hash,
        consumption_requested_at_ms: draft.consumption_requested_at_ms,
        replay_state_reviewed_unseen: true,
        challenge_reviewed_unconsumed: true,
        replay_store_mutated: false,
        challenge_consumed: false,
        capability_state_changed: false,
        runtime_io_performed: false,
        wallet_or_ledger_mutated: false,
        contract_only: true,
    })
}

fn validate_verification_envelope(
    envelope: &NativeProofVerificationAdapterEnvelopeV1,
) -> Result<(), NativeReplayChallengeConsumptionContractReviewError> {
    if envelope.contract_domain != PHASE10B_PROOF_VERIFICATION_ADAPTER_DOMAIN {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::VerificationEnvelopeDomainMismatch,
        );
    }

    if envelope.contract_version != PHASE10B_PROOF_VERIFICATION_ADAPTER_VERSION {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::VerificationEnvelopeVersionMismatch,
        );
    }

    if !envelope.signed_payload_accepted {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::VerificationEnvelopeNotAccepted,
        );
    }

    if envelope.replay_state_mutated
        || envelope.challenge_consumed
        || envelope.capability_state_changed
        || envelope.runtime_io_performed
        || envelope.wallet_or_ledger_mutated
    {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::VerificationEnvelopeAlreadyMutatedState,
        );
    }

    Ok(())
}

fn validate_draft_against_envelope(
    envelope: &NativeProofVerificationAdapterEnvelopeV1,
    draft: &NativeReplayChallengeConsumptionContractDraftV1,
) -> Result<(), NativeReplayChallengeConsumptionContractReviewError> {
    if draft.contract_domain != PHASE11A_REPLAY_CHALLENGE_CONSUMPTION_CONTRACT_DOMAIN {
        return Err(NativeReplayChallengeConsumptionContractReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE11A_REPLAY_CHALLENGE_CONSUMPTION_CONTRACT_VERSION {
        return Err(NativeReplayChallengeConsumptionContractReviewError::ContractVersionMismatch);
    }

    if !draft.contract_only {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::ConsumptionContractNotContractOnly,
        );
    }

    if !draft.expects_verified_payload_accepted {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::VerificationEnvelopeNotAccepted,
        );
    }

    if !draft.expects_replay_state_unseen {
        return Err(NativeReplayChallengeConsumptionContractReviewError::ReplayStateAlreadySeen);
    }

    if !draft.expects_challenge_unconsumed {
        return Err(NativeReplayChallengeConsumptionContractReviewError::ChallengeAlreadyConsumed);
    }

    if draft.operation_kind != envelope.operation_kind {
        return Err(NativeReplayChallengeConsumptionContractReviewError::OperationMismatch);
    }

    if draft.expected_authority != envelope.reviewed_authority {
        return Err(NativeReplayChallengeConsumptionContractReviewError::AuthorityMismatch);
    }

    if draft.challenge_id != envelope.challenge_id {
        return Err(NativeReplayChallengeConsumptionContractReviewError::ChallengeIdMismatch);
    }

    if draft.passport_id != envelope.passport_id {
        return Err(NativeReplayChallengeConsumptionContractReviewError::PassportBindingMismatch);
    }

    if draft.device_id != envelope.device_id {
        return Err(NativeReplayChallengeConsumptionContractReviewError::DeviceBindingMismatch);
    }

    if draft.proof_contract_domain != envelope.proof_contract_domain
        || draft.proof_contract_domain != PHASE8B_PROOF_CONTRACT_DOMAIN
    {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::ProofContractDomainMismatch,
        );
    }

    if draft.proof_contract_version != envelope.proof_contract_version
        || draft.proof_contract_version != PHASE8B_PROOF_CONTRACT_VERSION
    {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::ProofContractVersionMismatch,
        );
    }

    if draft.request_contract_domain != envelope.request_contract_domain {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::RequestProofContractDomainMismatch,
        );
    }

    if draft.request_contract_version != envelope.request_contract_version {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::RequestProofContractVersionMismatch,
        );
    }

    if let Some(domain) = draft.request_contract_domain {
        if domain != PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN {
            return Err(
                NativeReplayChallengeConsumptionContractReviewError::RequestProofContractDomainMismatch,
            );
        }
    }

    if let Some(version) = draft.request_contract_version {
        if version != PHASE8C_PROOF_REQUEST_CONTRACT_VERSION {
            return Err(
                NativeReplayChallengeConsumptionContractReviewError::RequestProofContractVersionMismatch,
            );
        }
    }

    if draft.proof_kind != envelope.proof_kind {
        return Err(NativeReplayChallengeConsumptionContractReviewError::ProofKindMismatch);
    }

    if draft.transcript_hash != envelope.transcript_hash {
        return Err(NativeReplayChallengeConsumptionContractReviewError::TranscriptHashMismatch);
    }

    if draft.signer_public_key_label.is_empty() {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::MissingSignerPublicKeyLabel,
        );
    }

    if draft.signer_public_key_label != envelope.signer_public_key_label {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::SignerPublicKeyLabelMismatch,
        );
    }

    validate_signature_algorithm(draft.expected_authority, draft.expected_signature_algorithm)?;

    if draft.expected_signature_algorithm != envelope.expected_signature_algorithm {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::SignatureAlgorithmMismatch,
        );
    }

    if draft.signed_payload_hex != envelope.signed_payload_hex {
        return Err(NativeReplayChallengeConsumptionContractReviewError::SignedPayloadMismatch);
    }

    if draft.verification_evidence_label.is_empty() {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::MissingVerificationEvidenceLabel,
        );
    }

    if draft.verification_evidence_label != envelope.evidence_label {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::VerificationEvidenceLabelMismatch,
        );
    }

    if draft.verifier_public_key_label.is_empty() {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::MissingVerifierPublicKeyLabel,
        );
    }

    if draft.verifier_public_key_label != envelope.verifier_public_key_label {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::VerifierPublicKeyLabelMismatch,
        );
    }

    if draft.consumption_requested_at_ms < envelope.verification_requested_at_ms {
        return Err(NativeReplayChallengeConsumptionContractReviewError::InvalidConsumptionTime);
    }

    Ok(())
}

fn validate_signature_algorithm(
    authority: NativePassportProofAuthority,
    algorithm: NativeProofSignatureAlgorithm,
) -> Result<(), NativeReplayChallengeConsumptionContractReviewError> {
    let expected = match authority {
        NativePassportProofAuthority::RootAuthority => PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
        NativePassportProofAuthority::DeviceAuthority => PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
    };

    if algorithm != expected {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::SignatureAlgorithmMismatch,
        );
    }

    Ok(())
}

fn validate_no_unsafe_flags(
    draft: &NativeReplayChallengeConsumptionContractDraftV1,
) -> Result<(), NativeReplayChallengeConsumptionContractReviewError> {
    if draft.requests_replay_store_mutation
        || draft.requests_challenge_consumption_execution
        || draft.requests_capability_consumption
        || draft.requests_capability_issuance
        || draft.requests_capability_lifecycle_runtime
        || draft.requests_secret_key_access
        || draft.requests_key_loading
        || draft.requests_key_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.requests_platform_sealer_unseal
        || draft.requests_runtime_io
        || draft.adds_routes
        || draft.requests_storage_mutation
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(
            NativeReplayChallengeConsumptionContractReviewError::UnsafeConsumptionAuthorityFlag,
        );
    }

    Ok(())
}
