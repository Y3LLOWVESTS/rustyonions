//! RO:WHAT — Native Passport private-beta `@username` registry adapter seam.
//! RO:WHY — P3 Identity & Keys + P7 SDK/Interop. Allows a future single-writer private-beta namespace adapter to apply a reviewed username transition while keeping native contract authority separate from durable state.
//! RO:INTERACTS — Phase 12A username registry transition decisions, ron-naming canonical UsernameV1, future namespace writer, and future svc-index projection.
//! RO:INVARIANTS — validates the Phase 12A decision, canonical handle, transition hash, idempotency key, proof transcript, Passport/device binding, owner/recipient shape, writer evidence, and projection posture before accepting adapter evidence. Native still adds no namespace routes, no durable writer implementation, no index authority, no native storage mutation, no wallet/ledger mutation, and no secret/key access.
//! RO:TEST — tests/native_passport_phase12b_username_private_beta_registry_adapter.rs.

use super::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, NativeUsernameRegistryTransitionDecisionV1,
    NativeUsernameRegistryTransitionKind, PassportIdV1, UsernameV1,
    PHASE12A_USERNAME_REGISTRY_CONTRACT_DOMAIN, PHASE12A_USERNAME_REGISTRY_CONTRACT_VERSION,
};

pub const NATIVE_PASSPORT_PHASE12B_LABEL: &str =
    "NATIVE_PASSPORT_PHASE12B_USERNAME_PRIVATE_BETA_REGISTRY_ADAPTER";

pub const PHASE12B_USERNAME_REGISTRY_ADAPTER_DOMAIN: &str =
    "native-passport/username-private-beta-registry-adapter/v1";

pub const PHASE12B_USERNAME_REGISTRY_ADAPTER_VERSION: u16 = 1;

pub const PHASE12B_FORBIDDEN_USERNAME_REGISTRY_ADAPTER_AUTHORITY_FLAGS: &[&str] = &[
    "username_as_real_or_legal_name_authority",
    "username_as_recovery_authority",
    "username_as_root_authority",
    "username_as_wallet_authority",
    "username_as_ledger_authority",
    "index_projection_authority",
    "namespace_route_added",
    "storage_mutation_inside_svc_passport_native",
    "secret_key_access",
    "key_loading",
    "key_derivation_runtime",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_unseal",
    "runtime_io",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUsernameRegistryAdapterDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub transition_kind: NativeUsernameRegistryTransitionKind,
    pub canonical_username: UsernameV1,
    pub canonical_handle: String,
    pub passport_id: PassportIdV1,
    pub device_id: Option<DeviceIdV1>,
    pub current_owner_passport_id: Option<PassportIdV1>,
    pub recipient_passport_id: Option<PassportIdV1>,
    pub challenge_id: ChallengeIdV1,
    pub operation_body_hash: B3DigestHex,
    pub transition_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub fresh_proof_transcript_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub transition_requested_at_ms: u64,
    pub transition_expires_at_ms: u64,
    pub allows_injected_private_beta_writer: bool,
    pub index_projection_only: bool,
    pub treats_index_projection_as_authority: bool,
    pub treats_username_as_real_or_legal_name_authority: bool,
    pub treats_username_as_recovery_authority: bool,
    pub treats_username_as_root_authority: bool,
    pub treats_username_as_wallet_or_ledger_authority: bool,
    pub requests_namespace_route: bool,
    pub requests_storage_mutation_inside_native: bool,
    pub requests_secret_key_access: bool,
    pub requests_key_loading: bool,
    pub requests_key_derivation_runtime: bool,
    pub requests_vault_unlock: bool,
    pub includes_vault_runtime: bool,
    pub requests_platform_sealer_unseal: bool,
    pub requests_runtime_io: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUsernameRegistryAdapterRequestV1 {
    pub transition_kind: NativeUsernameRegistryTransitionKind,
    pub canonical_username: UsernameV1,
    pub canonical_handle: String,
    pub passport_id: PassportIdV1,
    pub device_id: Option<DeviceIdV1>,
    pub current_owner_passport_id: Option<PassportIdV1>,
    pub recipient_passport_id: Option<PassportIdV1>,
    pub challenge_id: ChallengeIdV1,
    pub operation_body_hash: B3DigestHex,
    pub transition_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub fresh_proof_transcript_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub transition_requested_at_ms: u64,
    pub transition_expires_at_ms: u64,
    pub index_projection_only: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUsernameRegistryAdapterEvidenceV1 {
    pub accepted: bool,
    pub private_beta_writer_label: &'static str,
    pub transition_kind: NativeUsernameRegistryTransitionKind,
    pub canonical_username: UsernameV1,
    pub canonical_handle: String,
    pub passport_id: PassportIdV1,
    pub current_owner_passport_id: Option<PassportIdV1>,
    pub recipient_passport_id: Option<PassportIdV1>,
    pub transition_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub username_finalized: bool,
    pub index_projection_queued: bool,
    pub index_projection_authoritative: bool,
    pub secret_material_exposed: bool,
    pub runtime_io_performed: bool,
    pub wallet_or_ledger_mutated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUsernameRegistryAdapterEnvelopeV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub transition_kind: NativeUsernameRegistryTransitionKind,
    pub canonical_username: UsernameV1,
    pub canonical_handle: String,
    pub passport_id: PassportIdV1,
    pub device_id: Option<DeviceIdV1>,
    pub current_owner_passport_id: Option<PassportIdV1>,
    pub recipient_passport_id: Option<PassportIdV1>,
    pub challenge_id: ChallengeIdV1,
    pub operation_body_hash: B3DigestHex,
    pub transition_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub fresh_proof_transcript_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub private_beta_writer_label: &'static str,
    pub transition_requested_at_ms: u64,
    pub transition_expires_at_ms: u64,
    pub username_finalized_by_adapter: bool,
    pub index_projection_queued_by_adapter: bool,
    pub index_projection_authoritative: bool,
    pub storage_mutated_inside_native: bool,
    pub runtime_io_performed: bool,
    pub wallet_or_ledger_mutated: bool,
    pub secret_material_exposed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeUsernameRegistryAdapterReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    RegistryDecisionInvalid,
    RegistryDecisionAlreadyFinalized,
    InjectedPrivateBetaWriterNotAllowed,
    TransitionKindMismatch,
    CanonicalUsernameMismatch,
    CanonicalHandleMismatch,
    PassportBindingMismatch,
    DeviceBindingMismatch,
    CurrentOwnerMismatch,
    RecipientMismatch,
    ChallengeIdMismatch,
    OperationBodyHashMismatch,
    TransitionHashMismatch,
    IdempotencyKeyMismatch,
    FreshProofTranscriptHashMismatch,
    MissingSignerPublicKeyLabel,
    SignerPublicKeyLabelMismatch,
    TransitionTimeMismatch,
    IndexProjectionTreatedAsAuthority,
    UnsafeUsernameRegistryAdapterAuthorityFlag,
    WriterRejected,
    WriterEvidenceMismatch,
    WriterEvidenceUnsafe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUsernameRegistryAdapterPosture {
    pub phase_label: &'static str,
    pub username_registry_adapter_added: bool,
    pub injected_private_beta_writer_added: bool,
    pub username_finalization_evidence_added: bool,
    pub index_projection_queue_evidence_added: bool,
    pub durable_username_writer_implemented_inside_native: bool,
    pub index_projection_authority_added: bool,
    pub namespace_routes_added: bool,
    pub storage_mutation_inside_native_added: bool,
    pub username_as_real_or_legal_name_authority_added: bool,
    pub username_as_recovery_authority_added: bool,
    pub username_as_root_authority_added: bool,
    pub username_as_wallet_or_ledger_authority_added: bool,
    pub secret_key_access_added: bool,
    pub runtime_io_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub runtime_authority_changed: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub trait NativeLocalUsernameRegistryAdapter {
    fn apply_username_transition(
        &self,
        request: &NativeUsernameRegistryAdapterRequestV1,
    ) -> Result<NativeUsernameRegistryAdapterEvidenceV1, NativeUsernameRegistryAdapterReviewError>;
}

pub fn native_username_registry_adapter_posture() -> NativeUsernameRegistryAdapterPosture {
    NativeUsernameRegistryAdapterPosture {
        phase_label: NATIVE_PASSPORT_PHASE12B_LABEL,
        username_registry_adapter_added: true,
        injected_private_beta_writer_added: true,
        username_finalization_evidence_added: true,
        index_projection_queue_evidence_added: true,
        durable_username_writer_implemented_inside_native: false,
        index_projection_authority_added: false,
        namespace_routes_added: false,
        storage_mutation_inside_native_added: false,
        username_as_real_or_legal_name_authority_added: false,
        username_as_recovery_authority_added: false,
        username_as_root_authority_added: false,
        username_as_wallet_or_ledger_authority_added: false,
        secret_key_access_added: false,
        runtime_io_added: false,
        wallet_or_ledger_mutation_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE12B_FORBIDDEN_USERNAME_REGISTRY_ADAPTER_AUTHORITY_FLAGS,
    }
}

pub fn execute_native_username_registry_adapter<S: NativeLocalUsernameRegistryAdapter>(
    decision: &NativeUsernameRegistryTransitionDecisionV1,
    draft: NativeUsernameRegistryAdapterDraftV1,
    adapter: &S,
) -> Result<NativeUsernameRegistryAdapterEnvelopeV1, NativeUsernameRegistryAdapterReviewError> {
    validate_decision(decision)?;
    validate_adapter_draft(decision, &draft)?;

    let request = NativeUsernameRegistryAdapterRequestV1 {
        transition_kind: draft.transition_kind,
        canonical_username: draft.canonical_username.clone(),
        canonical_handle: draft.canonical_handle.clone(),
        passport_id: draft.passport_id.clone(),
        device_id: draft.device_id.clone(),
        current_owner_passport_id: draft.current_owner_passport_id.clone(),
        recipient_passport_id: draft.recipient_passport_id.clone(),
        challenge_id: draft.challenge_id.clone(),
        operation_body_hash: draft.operation_body_hash.clone(),
        transition_hash: draft.transition_hash.clone(),
        idempotency_key_hash: draft.idempotency_key_hash.clone(),
        fresh_proof_transcript_hash: draft.fresh_proof_transcript_hash.clone(),
        signer_public_key_label: draft.signer_public_key_label,
        transition_requested_at_ms: draft.transition_requested_at_ms,
        transition_expires_at_ms: draft.transition_expires_at_ms,
        index_projection_only: true,
        contract_only: true,
    };

    let evidence = adapter.apply_username_transition(&request)?;
    validate_evidence(&request, &evidence)?;

    Ok(NativeUsernameRegistryAdapterEnvelopeV1 {
        contract_domain: PHASE12B_USERNAME_REGISTRY_ADAPTER_DOMAIN,
        contract_version: PHASE12B_USERNAME_REGISTRY_ADAPTER_VERSION,
        transition_kind: draft.transition_kind,
        canonical_username: draft.canonical_username,
        canonical_handle: draft.canonical_handle,
        passport_id: draft.passport_id,
        device_id: draft.device_id,
        current_owner_passport_id: draft.current_owner_passport_id,
        recipient_passport_id: draft.recipient_passport_id,
        challenge_id: draft.challenge_id,
        operation_body_hash: draft.operation_body_hash,
        transition_hash: draft.transition_hash,
        idempotency_key_hash: draft.idempotency_key_hash,
        fresh_proof_transcript_hash: draft.fresh_proof_transcript_hash,
        signer_public_key_label: draft.signer_public_key_label,
        private_beta_writer_label: evidence.private_beta_writer_label,
        transition_requested_at_ms: draft.transition_requested_at_ms,
        transition_expires_at_ms: draft.transition_expires_at_ms,
        username_finalized_by_adapter: true,
        index_projection_queued_by_adapter: true,
        index_projection_authoritative: false,
        storage_mutated_inside_native: false,
        runtime_io_performed: false,
        wallet_or_ledger_mutated: false,
        secret_material_exposed: false,
    })
}

fn validate_decision(
    decision: &NativeUsernameRegistryTransitionDecisionV1,
) -> Result<(), NativeUsernameRegistryAdapterReviewError> {
    if decision.contract_domain != PHASE12A_USERNAME_REGISTRY_CONTRACT_DOMAIN
        || decision.contract_version != PHASE12A_USERNAME_REGISTRY_CONTRACT_VERSION
        || !decision.single_writer_private_beta_reviewed
        || !decision.index_projection_only
        || !decision.contract_only
    {
        return Err(NativeUsernameRegistryAdapterReviewError::RegistryDecisionInvalid);
    }

    if decision.username_finalized
        || decision.durable_writer_called
        || decision.index_projection_mutated
        || decision.storage_mutated_inside_native
        || decision.wallet_or_ledger_mutated
        || decision.secret_material_exposed
    {
        return Err(NativeUsernameRegistryAdapterReviewError::RegistryDecisionAlreadyFinalized);
    }

    if decision.signer_public_key_label.is_empty() {
        return Err(NativeUsernameRegistryAdapterReviewError::MissingSignerPublicKeyLabel);
    }

    if decision.canonical_handle != decision.canonical_username.handle() {
        return Err(NativeUsernameRegistryAdapterReviewError::CanonicalHandleMismatch);
    }

    Ok(())
}

fn validate_adapter_draft(
    decision: &NativeUsernameRegistryTransitionDecisionV1,
    draft: &NativeUsernameRegistryAdapterDraftV1,
) -> Result<(), NativeUsernameRegistryAdapterReviewError> {
    if draft.contract_domain != PHASE12B_USERNAME_REGISTRY_ADAPTER_DOMAIN {
        return Err(NativeUsernameRegistryAdapterReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE12B_USERNAME_REGISTRY_ADAPTER_VERSION {
        return Err(NativeUsernameRegistryAdapterReviewError::ContractVersionMismatch);
    }

    if !draft.allows_injected_private_beta_writer {
        return Err(NativeUsernameRegistryAdapterReviewError::InjectedPrivateBetaWriterNotAllowed);
    }

    if draft.transition_kind != decision.transition_kind {
        return Err(NativeUsernameRegistryAdapterReviewError::TransitionKindMismatch);
    }

    if draft.canonical_username != decision.canonical_username {
        return Err(NativeUsernameRegistryAdapterReviewError::CanonicalUsernameMismatch);
    }

    if draft.canonical_handle != decision.canonical_handle
        || draft.canonical_handle != draft.canonical_username.handle()
    {
        return Err(NativeUsernameRegistryAdapterReviewError::CanonicalHandleMismatch);
    }

    if draft.passport_id != decision.passport_id {
        return Err(NativeUsernameRegistryAdapterReviewError::PassportBindingMismatch);
    }

    if draft.device_id != decision.device_id {
        return Err(NativeUsernameRegistryAdapterReviewError::DeviceBindingMismatch);
    }

    if draft.current_owner_passport_id != decision.current_owner_passport_id {
        return Err(NativeUsernameRegistryAdapterReviewError::CurrentOwnerMismatch);
    }

    if draft.recipient_passport_id != decision.recipient_passport_id {
        return Err(NativeUsernameRegistryAdapterReviewError::RecipientMismatch);
    }

    if draft.challenge_id != decision.challenge_id {
        return Err(NativeUsernameRegistryAdapterReviewError::ChallengeIdMismatch);
    }

    if draft.operation_body_hash != decision.operation_body_hash {
        return Err(NativeUsernameRegistryAdapterReviewError::OperationBodyHashMismatch);
    }

    if draft.transition_hash != decision.transition_hash {
        return Err(NativeUsernameRegistryAdapterReviewError::TransitionHashMismatch);
    }

    if draft.idempotency_key_hash != decision.idempotency_key_hash {
        return Err(NativeUsernameRegistryAdapterReviewError::IdempotencyKeyMismatch);
    }

    if draft.fresh_proof_transcript_hash != decision.fresh_proof_transcript_hash {
        return Err(NativeUsernameRegistryAdapterReviewError::FreshProofTranscriptHashMismatch);
    }

    if draft.signer_public_key_label.is_empty() {
        return Err(NativeUsernameRegistryAdapterReviewError::MissingSignerPublicKeyLabel);
    }

    if draft.signer_public_key_label != decision.signer_public_key_label {
        return Err(NativeUsernameRegistryAdapterReviewError::SignerPublicKeyLabelMismatch);
    }

    if draft.transition_requested_at_ms != decision.transition_requested_at_ms
        || draft.transition_expires_at_ms != decision.transition_expires_at_ms
    {
        return Err(NativeUsernameRegistryAdapterReviewError::TransitionTimeMismatch);
    }

    if !draft.index_projection_only || draft.treats_index_projection_as_authority {
        return Err(NativeUsernameRegistryAdapterReviewError::IndexProjectionTreatedAsAuthority);
    }

    validate_no_unsafe_flags(draft)
}

fn validate_no_unsafe_flags(
    draft: &NativeUsernameRegistryAdapterDraftV1,
) -> Result<(), NativeUsernameRegistryAdapterReviewError> {
    if draft.treats_username_as_real_or_legal_name_authority
        || draft.treats_username_as_recovery_authority
        || draft.treats_username_as_root_authority
        || draft.treats_username_as_wallet_or_ledger_authority
        || draft.requests_namespace_route
        || draft.requests_storage_mutation_inside_native
        || draft.requests_secret_key_access
        || draft.requests_key_loading
        || draft.requests_key_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.requests_platform_sealer_unseal
        || draft.requests_runtime_io
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(
            NativeUsernameRegistryAdapterReviewError::UnsafeUsernameRegistryAdapterAuthorityFlag,
        );
    }

    Ok(())
}

fn validate_evidence(
    request: &NativeUsernameRegistryAdapterRequestV1,
    evidence: &NativeUsernameRegistryAdapterEvidenceV1,
) -> Result<(), NativeUsernameRegistryAdapterReviewError> {
    if !evidence.accepted {
        return Err(NativeUsernameRegistryAdapterReviewError::WriterRejected);
    }

    if evidence.private_beta_writer_label.is_empty() {
        return Err(NativeUsernameRegistryAdapterReviewError::WriterEvidenceMismatch);
    }

    if evidence.transition_kind != request.transition_kind
        || evidence.canonical_username != request.canonical_username
        || evidence.canonical_handle != request.canonical_handle
        || evidence.passport_id != request.passport_id
        || evidence.current_owner_passport_id != request.current_owner_passport_id
        || evidence.recipient_passport_id != request.recipient_passport_id
        || evidence.transition_hash != request.transition_hash
        || evidence.idempotency_key_hash != request.idempotency_key_hash
    {
        return Err(NativeUsernameRegistryAdapterReviewError::WriterEvidenceMismatch);
    }

    if !evidence.username_finalized || !evidence.index_projection_queued {
        return Err(NativeUsernameRegistryAdapterReviewError::WriterEvidenceMismatch);
    }

    if evidence.index_projection_authoritative
        || evidence.secret_material_exposed
        || evidence.runtime_io_performed
        || evidence.wallet_or_ledger_mutated
    {
        return Err(NativeUsernameRegistryAdapterReviewError::WriterEvidenceUnsafe);
    }

    Ok(())
}
