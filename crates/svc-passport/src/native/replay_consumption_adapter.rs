//! RO:WHAT — Native Passport injected replay/challenge-consumption adapter seam.
//! RO:WHY — P3 Identity & Keys. Allows a future durable state adapter to apply replay and challenge-consumption evidence only after Phase 11A reviewed the exact verified envelope binding.
//! RO:INTERACTS — Phase 11A replay/challenge-consumption contract decisions and future durable replay/challenge state adapters.
//! RO:INVARIANTS — validates the Phase 11A decision, replay key, idempotency key, challenge/proof/request binding, signer/verifier labels, authority, algorithm, signed payload, and timing before calling an injected state adapter. svc-passport native still does not own durable storage, routes, runtime I/O, wallet/ledger mutation, or secret/key material.
//! RO:TEST — tests/native_passport_phase11b_replay_and_challenge_consumption_adapter.rs.

use super::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, NativePassportProofAuthority, NativePassportProofKind,
    NativeProofSignatureAlgorithm, NativeProofSignedPayloadHex, NativeProofSigningOperationKind,
    NativeReplayChallengeConsumptionContractDecisionV1, PassportIdV1,
    PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_PROOF_CONTRACT_DOMAIN,
    PHASE8B_PROOF_CONTRACT_VERSION, PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
    PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN, PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
};

pub const NATIVE_PASSPORT_PHASE11B_LABEL: &str =
    "NATIVE_PASSPORT_PHASE11B_REPLAY_AND_CHALLENGE_CONSUMPTION_ADAPTER";

pub const PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_DOMAIN: &str =
    "native-passport/replay-challenge-consumption-adapter/v1";

pub const PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_VERSION: u16 = 1;

pub const PHASE11B_FORBIDDEN_CONSUMPTION_ADAPTER_AUTHORITY_FLAGS: &[&str] = &[
    "secret_key_access",
    "key_loading",
    "key_derivation_runtime",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_unseal",
    "capability_consumption",
    "capability_issuance",
    "capability_refresh",
    "capability_revocation",
    "runtime_io",
    "route_added",
    "storage_mutation_inside_svc_passport_native",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReplayChallengeConsumptionAdapterDraftV1 {
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
    pub allows_injected_local_state_adapter: bool,
    pub requests_secret_key_access: bool,
    pub requests_key_loading: bool,
    pub requests_key_derivation_runtime: bool,
    pub requests_vault_unlock: bool,
    pub includes_vault_runtime: bool,
    pub requests_platform_sealer_unseal: bool,
    pub requests_capability_consumption: bool,
    pub requests_capability_issuance: bool,
    pub requests_capability_lifecycle_runtime: bool,
    pub requests_runtime_io: bool,
    pub adds_routes: bool,
    pub requests_storage_mutation_inside_native: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReplayChallengeConsumptionAdapterRequestV1 {
    pub operation_kind: NativeProofSigningOperationKind,
    pub expected_authority: NativePassportProofAuthority,
    pub challenge_id: ChallengeIdV1,
    pub passport_id: Option<PassportIdV1>,
    pub device_id: Option<DeviceIdV1>,
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
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReplayChallengeConsumptionAdapterEvidenceV1 {
    pub accepted: bool,
    pub state_adapter_label: &'static str,
    pub challenge_id: ChallengeIdV1,
    pub replay_key_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub replay_recorded: bool,
    pub challenge_marked_consumed: bool,
    pub capability_state_changed: bool,
    pub secret_material_exposed: bool,
    pub runtime_io_performed: bool,
    pub wallet_or_ledger_mutated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReplayChallengeConsumptionAdapterEnvelopeV1 {
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
    pub state_adapter_label: &'static str,
    pub replay_recorded_by_adapter: bool,
    pub challenge_consumed_by_adapter: bool,
    pub capability_state_changed: bool,
    pub runtime_io_performed: bool,
    pub wallet_or_ledger_mutated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeReplayChallengeConsumptionAdapterReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    ConsumptionDecisionInvalid,
    ConsumptionDecisionAlreadyApplied,
    InjectedStateAdapterNotAllowed,
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
    ReplayKeyMismatch,
    IdempotencyKeyMismatch,
    ConsumptionTimeMismatch,
    UnsafeAdapterAuthorityFlag,
    StateAdapterRejected,
    StateAdapterEvidenceMismatch,
    StateAdapterEvidenceUnsafe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReplayChallengeConsumptionAdapterPosture {
    pub phase_label: &'static str,
    pub replay_challenge_consumption_adapter_added: bool,
    pub injected_local_state_adapter_added: bool,
    pub replay_recording_evidence_added: bool,
    pub challenge_consumption_evidence_added: bool,
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
    pub storage_mutation_inside_native_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub runtime_authority_changed: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub trait NativeLocalReplayChallengeConsumptionAdapter {
    fn apply_consumption_state(
        &self,
        request: &NativeReplayChallengeConsumptionAdapterRequestV1,
    ) -> Result<
        NativeReplayChallengeConsumptionAdapterEvidenceV1,
        NativeReplayChallengeConsumptionAdapterReviewError,
    >;
}

pub fn native_replay_challenge_consumption_adapter_posture(
) -> NativeReplayChallengeConsumptionAdapterPosture {
    NativeReplayChallengeConsumptionAdapterPosture {
        phase_label: NATIVE_PASSPORT_PHASE11B_LABEL,
        replay_challenge_consumption_adapter_added: true,
        injected_local_state_adapter_added: true,
        replay_recording_evidence_added: true,
        challenge_consumption_evidence_added: true,
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
        storage_mutation_inside_native_added: false,
        wallet_or_ledger_mutation_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE11B_FORBIDDEN_CONSUMPTION_ADAPTER_AUTHORITY_FLAGS,
    }
}

pub fn execute_native_replay_challenge_consumption_adapter<
    S: NativeLocalReplayChallengeConsumptionAdapter,
>(
    decision: &NativeReplayChallengeConsumptionContractDecisionV1,
    draft: NativeReplayChallengeConsumptionAdapterDraftV1,
    adapter: &S,
) -> Result<
    NativeReplayChallengeConsumptionAdapterEnvelopeV1,
    NativeReplayChallengeConsumptionAdapterReviewError,
> {
    validate_decision(decision)?;
    validate_adapter_draft(decision, &draft)?;

    let request = NativeReplayChallengeConsumptionAdapterRequestV1 {
        operation_kind: draft.operation_kind,
        expected_authority: draft.expected_authority,
        challenge_id: draft.challenge_id.clone(),
        passport_id: draft.passport_id.clone(),
        device_id: draft.device_id.clone(),
        proof_kind: draft.proof_kind,
        transcript_hash: draft.transcript_hash.clone(),
        signer_public_key_label: draft.signer_public_key_label,
        expected_signature_algorithm: draft.expected_signature_algorithm,
        signed_payload_hex: draft.signed_payload_hex.clone(),
        verification_evidence_label: draft.verification_evidence_label,
        verifier_public_key_label: draft.verifier_public_key_label,
        replay_key_hash: draft.replay_key_hash.clone(),
        idempotency_key_hash: draft.idempotency_key_hash.clone(),
        consumption_requested_at_ms: draft.consumption_requested_at_ms,
        contract_only: true,
    };

    let evidence = adapter.apply_consumption_state(&request)?;
    validate_evidence(&request, &evidence)?;

    Ok(NativeReplayChallengeConsumptionAdapterEnvelopeV1 {
        contract_domain: PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_DOMAIN,
        contract_version: PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_VERSION,
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
        state_adapter_label: evidence.state_adapter_label,
        replay_recorded_by_adapter: true,
        challenge_consumed_by_adapter: true,
        capability_state_changed: false,
        runtime_io_performed: false,
        wallet_or_ledger_mutated: false,
    })
}

fn validate_decision(
    decision: &NativeReplayChallengeConsumptionContractDecisionV1,
) -> Result<(), NativeReplayChallengeConsumptionAdapterReviewError> {
    if !decision.contract_only {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::ConsumptionDecisionInvalid);
    }

    if decision.replay_store_mutated
        || decision.challenge_consumed
        || decision.capability_state_changed
        || decision.runtime_io_performed
        || decision.wallet_or_ledger_mutated
    {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::ConsumptionDecisionAlreadyApplied,
        );
    }

    Ok(())
}

fn validate_adapter_draft(
    decision: &NativeReplayChallengeConsumptionContractDecisionV1,
    draft: &NativeReplayChallengeConsumptionAdapterDraftV1,
) -> Result<(), NativeReplayChallengeConsumptionAdapterReviewError> {
    if draft.contract_domain != PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_DOMAIN {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_VERSION {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::ContractVersionMismatch);
    }

    if !draft.allows_injected_local_state_adapter {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::InjectedStateAdapterNotAllowed,
        );
    }

    if draft.operation_kind != decision.operation_kind {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::OperationMismatch);
    }

    if draft.expected_authority != decision.reviewed_authority {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::AuthorityMismatch);
    }

    if draft.challenge_id != decision.challenge_id {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::ChallengeIdMismatch);
    }

    if draft.passport_id != decision.passport_id {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::PassportBindingMismatch);
    }

    if draft.device_id != decision.device_id {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::DeviceBindingMismatch);
    }

    if draft.proof_contract_domain != decision.proof_contract_domain
        || draft.proof_contract_domain != PHASE8B_PROOF_CONTRACT_DOMAIN
    {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::ProofContractDomainMismatch,
        );
    }

    if draft.proof_contract_version != decision.proof_contract_version
        || draft.proof_contract_version != PHASE8B_PROOF_CONTRACT_VERSION
    {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::ProofContractVersionMismatch,
        );
    }

    if draft.request_contract_domain != decision.request_contract_domain {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::RequestProofContractDomainMismatch,
        );
    }

    if draft.request_contract_version != decision.request_contract_version {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::RequestProofContractVersionMismatch,
        );
    }

    if let Some(domain) = draft.request_contract_domain {
        if domain != PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN {
            return Err(
                NativeReplayChallengeConsumptionAdapterReviewError::RequestProofContractDomainMismatch,
            );
        }
    }

    if let Some(version) = draft.request_contract_version {
        if version != PHASE8C_PROOF_REQUEST_CONTRACT_VERSION {
            return Err(
                NativeReplayChallengeConsumptionAdapterReviewError::RequestProofContractVersionMismatch,
            );
        }
    }

    if draft.proof_kind != decision.proof_kind {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::ProofKindMismatch);
    }

    if draft.transcript_hash != decision.transcript_hash {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::TranscriptHashMismatch);
    }

    if draft.signer_public_key_label.is_empty() {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::MissingSignerPublicKeyLabel,
        );
    }

    if draft.signer_public_key_label != decision.signer_public_key_label {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::SignerPublicKeyLabelMismatch,
        );
    }

    validate_signature_algorithm(draft.expected_authority, draft.expected_signature_algorithm)?;

    if draft.expected_signature_algorithm != decision.expected_signature_algorithm {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::SignatureAlgorithmMismatch);
    }

    if draft.signed_payload_hex != decision.signed_payload_hex {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::SignedPayloadMismatch);
    }

    if draft.verification_evidence_label.is_empty() {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::MissingVerificationEvidenceLabel,
        );
    }

    if draft.verification_evidence_label != decision.verification_evidence_label {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::VerificationEvidenceLabelMismatch,
        );
    }

    if draft.verifier_public_key_label.is_empty() {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::MissingVerifierPublicKeyLabel,
        );
    }

    if draft.verifier_public_key_label != decision.verifier_public_key_label {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::VerifierPublicKeyLabelMismatch,
        );
    }

    if draft.replay_key_hash != decision.replay_key_hash {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::ReplayKeyMismatch);
    }

    if draft.idempotency_key_hash != decision.idempotency_key_hash {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::IdempotencyKeyMismatch);
    }

    if draft.consumption_requested_at_ms != decision.consumption_requested_at_ms {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::ConsumptionTimeMismatch);
    }

    validate_no_unsafe_flags(draft)
}

fn validate_signature_algorithm(
    authority: NativePassportProofAuthority,
    algorithm: NativeProofSignatureAlgorithm,
) -> Result<(), NativeReplayChallengeConsumptionAdapterReviewError> {
    let expected = match authority {
        NativePassportProofAuthority::RootAuthority => PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
        NativePassportProofAuthority::DeviceAuthority => PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
    };

    if algorithm != expected {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::SignatureAlgorithmMismatch);
    }

    Ok(())
}

fn validate_no_unsafe_flags(
    draft: &NativeReplayChallengeConsumptionAdapterDraftV1,
) -> Result<(), NativeReplayChallengeConsumptionAdapterReviewError> {
    if draft.requests_secret_key_access
        || draft.requests_key_loading
        || draft.requests_key_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.requests_platform_sealer_unseal
        || draft.requests_capability_consumption
        || draft.requests_capability_issuance
        || draft.requests_capability_lifecycle_runtime
        || draft.requests_runtime_io
        || draft.adds_routes
        || draft.requests_storage_mutation_inside_native
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::UnsafeAdapterAuthorityFlag);
    }

    Ok(())
}

fn validate_evidence(
    request: &NativeReplayChallengeConsumptionAdapterRequestV1,
    evidence: &NativeReplayChallengeConsumptionAdapterEvidenceV1,
) -> Result<(), NativeReplayChallengeConsumptionAdapterReviewError> {
    if !evidence.accepted {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::StateAdapterRejected);
    }

    if evidence.state_adapter_label.is_empty() {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::StateAdapterEvidenceMismatch,
        );
    }

    if evidence.challenge_id != request.challenge_id
        || evidence.replay_key_hash != request.replay_key_hash
        || evidence.idempotency_key_hash != request.idempotency_key_hash
    {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::StateAdapterEvidenceMismatch,
        );
    }

    if !evidence.replay_recorded || !evidence.challenge_marked_consumed {
        return Err(
            NativeReplayChallengeConsumptionAdapterReviewError::StateAdapterEvidenceMismatch,
        );
    }

    if evidence.capability_state_changed
        || evidence.secret_material_exposed
        || evidence.runtime_io_performed
        || evidence.wallet_or_ledger_mutated
    {
        return Err(NativeReplayChallengeConsumptionAdapterReviewError::StateAdapterEvidenceUnsafe);
    }

    Ok(())
}
