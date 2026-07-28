//! RO:WHAT — Native Passport private-beta `@username` exact-lookup projection contract.
//! RO:WHY — P3 Identity & Keys + P7 SDK/Interop. Defines the bounded projection event shape after a private-beta username writer finalizes a transition.
//! RO:INTERACTS — Phase 12B username registry adapter envelopes, ron-naming canonical UsernameV1, and future svc-index exact lookup projection.
//! RO:INVARIANTS — index state is a projection only. It is not username ownership authority, real/legal-name authority, recovery authority, wallet authority, ledger authority, or root authority. This phase adds no route, no index writer, no storage mutation, no runtime I/O, no wallet/ledger mutation, and no secret/key access.
//! RO:TEST — tests/native_passport_phase12c_username_private_beta_index_projection.rs.

use super::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, NativeUsernameRegistryAdapterEnvelopeV1,
    NativeUsernameRegistryTransitionKind, PassportIdV1, UsernameV1,
    PHASE12B_USERNAME_REGISTRY_ADAPTER_DOMAIN, PHASE12B_USERNAME_REGISTRY_ADAPTER_VERSION,
};

pub const NATIVE_PASSPORT_PHASE12C_LABEL: &str =
    "NATIVE_PASSPORT_PHASE12C_USERNAME_PRIVATE_BETA_INDEX_PROJECTION";

pub const PHASE12C_USERNAME_INDEX_PROJECTION_DOMAIN: &str =
    "native-passport/username-private-beta-index-projection/v1";

pub const PHASE12C_USERNAME_INDEX_PROJECTION_VERSION: u16 = 1;

pub const PHASE12C_FORBIDDEN_USERNAME_INDEX_PROJECTION_AUTHORITY_FLAGS: &[&str] = &[
    "index_as_username_ownership_authority",
    "index_as_real_or_legal_name_authority",
    "index_as_recovery_authority",
    "index_as_root_authority",
    "index_as_wallet_authority",
    "index_as_ledger_authority",
    "namespace_route_added",
    "index_writer_execution_inside_svc_passport_native",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeUsernameIndexProjectionKind {
    ExactLookupUpsert,
    ExactLookupDelete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUsernameIndexProjectionDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub projection_kind: NativeUsernameIndexProjectionKind,
    pub transition_kind: NativeUsernameRegistryTransitionKind,
    pub canonical_username: UsernameV1,
    pub canonical_handle: String,
    pub lookup_key: String,
    pub passport_id: PassportIdV1,
    pub projected_owner_passport_id: Option<PassportIdV1>,
    pub device_id: Option<DeviceIdV1>,
    pub current_owner_passport_id: Option<PassportIdV1>,
    pub recipient_passport_id: Option<PassportIdV1>,
    pub challenge_id: ChallengeIdV1,
    pub operation_body_hash: B3DigestHex,
    pub transition_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub fresh_proof_transcript_hash: B3DigestHex,
    pub projection_event_hash: B3DigestHex,
    pub lookup_key_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub private_beta_writer_label: &'static str,
    pub transition_requested_at_ms: u64,
    pub transition_expires_at_ms: u64,
    pub projection_queued_at_ms: u64,
    pub expects_username_finalized_by_adapter: bool,
    pub expects_index_projection_queued_by_adapter: bool,
    pub expects_index_projection_non_authoritative: bool,
    pub projection_only: bool,
    pub contract_only: bool,
    pub treats_index_as_username_ownership_authority: bool,
    pub treats_index_as_real_or_legal_name_authority: bool,
    pub treats_index_as_recovery_authority: bool,
    pub treats_index_as_root_authority: bool,
    pub treats_index_as_wallet_or_ledger_authority: bool,
    pub requests_namespace_route: bool,
    pub requests_index_writer_execution_inside_native: bool,
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
pub struct NativeUsernameIndexProjectionDecisionV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub projection_kind: NativeUsernameIndexProjectionKind,
    pub transition_kind: NativeUsernameRegistryTransitionKind,
    pub canonical_username: UsernameV1,
    pub canonical_handle: String,
    pub lookup_key: String,
    pub passport_id: PassportIdV1,
    pub projected_owner_passport_id: Option<PassportIdV1>,
    pub device_id: Option<DeviceIdV1>,
    pub current_owner_passport_id: Option<PassportIdV1>,
    pub recipient_passport_id: Option<PassportIdV1>,
    pub challenge_id: ChallengeIdV1,
    pub operation_body_hash: B3DigestHex,
    pub transition_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub fresh_proof_transcript_hash: B3DigestHex,
    pub projection_event_hash: B3DigestHex,
    pub lookup_key_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub private_beta_writer_label: &'static str,
    pub projection_queued_at_ms: u64,
    pub projection_only: bool,
    pub username_authority_changed: bool,
    pub index_writer_called: bool,
    pub storage_mutated_inside_native: bool,
    pub runtime_io_performed: bool,
    pub wallet_or_ledger_mutated: bool,
    pub secret_material_exposed: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeUsernameIndexProjectionReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    RegistryAdapterEnvelopeDomainMismatch,
    RegistryAdapterEnvelopeVersionMismatch,
    RegistryAdapterEnvelopeNotFinalized,
    RegistryAdapterEnvelopeUnsafe,
    ProjectionKindMismatch,
    TransitionKindMismatch,
    CanonicalUsernameMismatch,
    CanonicalHandleMismatch,
    LookupKeyMismatch,
    PassportBindingMismatch,
    ProjectedOwnerMismatch,
    DeviceBindingMismatch,
    CurrentOwnerMismatch,
    RecipientMismatch,
    ChallengeIdMismatch,
    OperationBodyHashMismatch,
    TransitionHashMismatch,
    IdempotencyKeyMismatch,
    FreshProofTranscriptHashMismatch,
    MissingProjectionEventHash,
    MissingLookupKeyHash,
    MissingSignerPublicKeyLabel,
    SignerPublicKeyLabelMismatch,
    MissingPrivateBetaWriterLabel,
    PrivateBetaWriterLabelMismatch,
    InvalidProjectionTime,
    IndexProjectionTreatedAsAuthority,
    UsernameIndexProjectionNotProjectionOnly,
    UnsafeUsernameIndexProjectionAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUsernameIndexProjectionPosture {
    pub phase_label: &'static str,
    pub username_index_projection_contract_added: bool,
    pub exact_lookup_projection_added: bool,
    pub projection_event_binding_added: bool,
    pub lookup_key_binding_added: bool,
    pub index_writer_execution_inside_native_added: bool,
    pub namespace_routes_added: bool,
    pub storage_mutation_inside_native_added: bool,
    pub index_as_username_ownership_authority_added: bool,
    pub index_as_real_or_legal_name_authority_added: bool,
    pub index_as_recovery_authority_added: bool,
    pub index_as_root_authority_added: bool,
    pub index_as_wallet_or_ledger_authority_added: bool,
    pub secret_key_access_added: bool,
    pub runtime_io_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub runtime_authority_changed: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_username_index_projection_posture() -> NativeUsernameIndexProjectionPosture {
    NativeUsernameIndexProjectionPosture {
        phase_label: NATIVE_PASSPORT_PHASE12C_LABEL,
        username_index_projection_contract_added: true,
        exact_lookup_projection_added: true,
        projection_event_binding_added: true,
        lookup_key_binding_added: true,
        index_writer_execution_inside_native_added: false,
        namespace_routes_added: false,
        storage_mutation_inside_native_added: false,
        index_as_username_ownership_authority_added: false,
        index_as_real_or_legal_name_authority_added: false,
        index_as_recovery_authority_added: false,
        index_as_root_authority_added: false,
        index_as_wallet_or_ledger_authority_added: false,
        secret_key_access_added: false,
        runtime_io_added: false,
        wallet_or_ledger_mutation_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE12C_FORBIDDEN_USERNAME_INDEX_PROJECTION_AUTHORITY_FLAGS,
    }
}

pub fn review_native_username_index_projection_contract(
    envelope: &NativeUsernameRegistryAdapterEnvelopeV1,
    draft: NativeUsernameIndexProjectionDraftV1,
) -> Result<NativeUsernameIndexProjectionDecisionV1, NativeUsernameIndexProjectionReviewError> {
    validate_registry_adapter_envelope(envelope)?;
    validate_draft_against_envelope(envelope, &draft)?;
    validate_no_unsafe_flags(&draft)?;

    Ok(NativeUsernameIndexProjectionDecisionV1 {
        contract_domain: PHASE12C_USERNAME_INDEX_PROJECTION_DOMAIN,
        contract_version: PHASE12C_USERNAME_INDEX_PROJECTION_VERSION,
        projection_kind: draft.projection_kind,
        transition_kind: draft.transition_kind,
        canonical_username: draft.canonical_username,
        canonical_handle: draft.canonical_handle,
        lookup_key: draft.lookup_key,
        passport_id: draft.passport_id,
        projected_owner_passport_id: draft.projected_owner_passport_id,
        device_id: draft.device_id,
        current_owner_passport_id: draft.current_owner_passport_id,
        recipient_passport_id: draft.recipient_passport_id,
        challenge_id: draft.challenge_id,
        operation_body_hash: draft.operation_body_hash,
        transition_hash: draft.transition_hash,
        idempotency_key_hash: draft.idempotency_key_hash,
        fresh_proof_transcript_hash: draft.fresh_proof_transcript_hash,
        projection_event_hash: draft.projection_event_hash,
        lookup_key_hash: draft.lookup_key_hash,
        signer_public_key_label: draft.signer_public_key_label,
        private_beta_writer_label: draft.private_beta_writer_label,
        projection_queued_at_ms: draft.projection_queued_at_ms,
        projection_only: true,
        username_authority_changed: false,
        index_writer_called: false,
        storage_mutated_inside_native: false,
        runtime_io_performed: false,
        wallet_or_ledger_mutated: false,
        secret_material_exposed: false,
        contract_only: true,
    })
}

fn validate_registry_adapter_envelope(
    envelope: &NativeUsernameRegistryAdapterEnvelopeV1,
) -> Result<(), NativeUsernameIndexProjectionReviewError> {
    if envelope.contract_domain != PHASE12B_USERNAME_REGISTRY_ADAPTER_DOMAIN {
        return Err(
            NativeUsernameIndexProjectionReviewError::RegistryAdapterEnvelopeDomainMismatch,
        );
    }

    if envelope.contract_version != PHASE12B_USERNAME_REGISTRY_ADAPTER_VERSION {
        return Err(
            NativeUsernameIndexProjectionReviewError::RegistryAdapterEnvelopeVersionMismatch,
        );
    }

    if !envelope.username_finalized_by_adapter || !envelope.index_projection_queued_by_adapter {
        return Err(NativeUsernameIndexProjectionReviewError::RegistryAdapterEnvelopeNotFinalized);
    }

    if envelope.index_projection_authoritative
        || envelope.storage_mutated_inside_native
        || envelope.runtime_io_performed
        || envelope.wallet_or_ledger_mutated
        || envelope.secret_material_exposed
    {
        return Err(NativeUsernameIndexProjectionReviewError::RegistryAdapterEnvelopeUnsafe);
    }

    Ok(())
}

fn validate_draft_against_envelope(
    envelope: &NativeUsernameRegistryAdapterEnvelopeV1,
    draft: &NativeUsernameIndexProjectionDraftV1,
) -> Result<(), NativeUsernameIndexProjectionReviewError> {
    if draft.contract_domain != PHASE12C_USERNAME_INDEX_PROJECTION_DOMAIN {
        return Err(NativeUsernameIndexProjectionReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE12C_USERNAME_INDEX_PROJECTION_VERSION {
        return Err(NativeUsernameIndexProjectionReviewError::ContractVersionMismatch);
    }

    let expected_projection_kind = expected_projection_kind(envelope.transition_kind);
    if draft.projection_kind != expected_projection_kind {
        return Err(NativeUsernameIndexProjectionReviewError::ProjectionKindMismatch);
    }

    if draft.transition_kind != envelope.transition_kind {
        return Err(NativeUsernameIndexProjectionReviewError::TransitionKindMismatch);
    }

    if draft.canonical_username != envelope.canonical_username {
        return Err(NativeUsernameIndexProjectionReviewError::CanonicalUsernameMismatch);
    }

    if draft.canonical_handle != envelope.canonical_handle
        || draft.canonical_handle != draft.canonical_username.handle()
    {
        return Err(NativeUsernameIndexProjectionReviewError::CanonicalHandleMismatch);
    }

    if draft.lookup_key != expected_lookup_key(&draft.canonical_handle) {
        return Err(NativeUsernameIndexProjectionReviewError::LookupKeyMismatch);
    }

    if draft.passport_id != envelope.passport_id {
        return Err(NativeUsernameIndexProjectionReviewError::PassportBindingMismatch);
    }

    let expected_owner = expected_projected_owner(envelope);
    if draft.projected_owner_passport_id != expected_owner {
        return Err(NativeUsernameIndexProjectionReviewError::ProjectedOwnerMismatch);
    }

    if draft.device_id != envelope.device_id {
        return Err(NativeUsernameIndexProjectionReviewError::DeviceBindingMismatch);
    }

    if draft.current_owner_passport_id != envelope.current_owner_passport_id {
        return Err(NativeUsernameIndexProjectionReviewError::CurrentOwnerMismatch);
    }

    if draft.recipient_passport_id != envelope.recipient_passport_id {
        return Err(NativeUsernameIndexProjectionReviewError::RecipientMismatch);
    }

    if draft.challenge_id != envelope.challenge_id {
        return Err(NativeUsernameIndexProjectionReviewError::ChallengeIdMismatch);
    }

    if draft.operation_body_hash != envelope.operation_body_hash {
        return Err(NativeUsernameIndexProjectionReviewError::OperationBodyHashMismatch);
    }

    if draft.transition_hash != envelope.transition_hash {
        return Err(NativeUsernameIndexProjectionReviewError::TransitionHashMismatch);
    }

    if draft.idempotency_key_hash != envelope.idempotency_key_hash {
        return Err(NativeUsernameIndexProjectionReviewError::IdempotencyKeyMismatch);
    }

    if draft.fresh_proof_transcript_hash != envelope.fresh_proof_transcript_hash {
        return Err(NativeUsernameIndexProjectionReviewError::FreshProofTranscriptHashMismatch);
    }

    if draft.projection_event_hash.as_str().is_empty() {
        return Err(NativeUsernameIndexProjectionReviewError::MissingProjectionEventHash);
    }

    if draft.lookup_key_hash.as_str().is_empty() {
        return Err(NativeUsernameIndexProjectionReviewError::MissingLookupKeyHash);
    }

    if draft.signer_public_key_label.is_empty() {
        return Err(NativeUsernameIndexProjectionReviewError::MissingSignerPublicKeyLabel);
    }

    if draft.signer_public_key_label != envelope.signer_public_key_label {
        return Err(NativeUsernameIndexProjectionReviewError::SignerPublicKeyLabelMismatch);
    }

    if draft.private_beta_writer_label.is_empty() {
        return Err(NativeUsernameIndexProjectionReviewError::MissingPrivateBetaWriterLabel);
    }

    if draft.private_beta_writer_label != envelope.private_beta_writer_label {
        return Err(NativeUsernameIndexProjectionReviewError::PrivateBetaWriterLabelMismatch);
    }

    if draft.projection_queued_at_ms < envelope.transition_requested_at_ms
        || draft.projection_queued_at_ms > envelope.transition_expires_at_ms
    {
        return Err(NativeUsernameIndexProjectionReviewError::InvalidProjectionTime);
    }

    if !draft.expects_username_finalized_by_adapter
        || !draft.expects_index_projection_queued_by_adapter
        || !draft.expects_index_projection_non_authoritative
    {
        return Err(NativeUsernameIndexProjectionReviewError::RegistryAdapterEnvelopeNotFinalized);
    }

    if !draft.projection_only || !draft.contract_only {
        return Err(
            NativeUsernameIndexProjectionReviewError::UsernameIndexProjectionNotProjectionOnly,
        );
    }

    if draft.treats_index_as_username_ownership_authority
        || draft.treats_index_as_real_or_legal_name_authority
        || draft.treats_index_as_recovery_authority
        || draft.treats_index_as_root_authority
        || draft.treats_index_as_wallet_or_ledger_authority
    {
        return Err(NativeUsernameIndexProjectionReviewError::IndexProjectionTreatedAsAuthority);
    }

    Ok(())
}

fn expected_projection_kind(
    transition_kind: NativeUsernameRegistryTransitionKind,
) -> NativeUsernameIndexProjectionKind {
    match transition_kind {
        NativeUsernameRegistryTransitionKind::Claim
        | NativeUsernameRegistryTransitionKind::Transfer => {
            NativeUsernameIndexProjectionKind::ExactLookupUpsert
        }
        NativeUsernameRegistryTransitionKind::Release => {
            NativeUsernameIndexProjectionKind::ExactLookupDelete
        }
    }
}

fn expected_projected_owner(
    envelope: &NativeUsernameRegistryAdapterEnvelopeV1,
) -> Option<PassportIdV1> {
    match envelope.transition_kind {
        NativeUsernameRegistryTransitionKind::Claim => Some(envelope.passport_id.clone()),
        NativeUsernameRegistryTransitionKind::Transfer => envelope.recipient_passport_id.clone(),
        NativeUsernameRegistryTransitionKind::Release => None,
    }
}

fn expected_lookup_key(canonical_handle: &str) -> String {
    format!("username/exact/{canonical_handle}")
}

fn validate_no_unsafe_flags(
    draft: &NativeUsernameIndexProjectionDraftV1,
) -> Result<(), NativeUsernameIndexProjectionReviewError> {
    if draft.requests_namespace_route
        || draft.requests_index_writer_execution_inside_native
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
            NativeUsernameIndexProjectionReviewError::UnsafeUsernameIndexProjectionAuthorityFlag,
        );
    }

    Ok(())
}
