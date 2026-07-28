//! RO:WHAT — Native Passport private-beta `@username` registry contract.
//! RO:WHY — P3 Identity & Keys + P7 SDK/Interop. Starts the durable username state machine as a pure contract/review surface that reuses `ron-naming` canonical handle parsing.
//! RO:INTERACTS — ron-naming canonical UsernameV1, Passport IDs, proof/challenge transcript bindings, future private-beta namespace writer, and future svc-index exact lookup projection.
//! RO:INVARIANTS — `@username` is an optional public handle bound to a Passport ID after a finalized transition. It is not a real/legal name, recovery authority, root authority, wallet authority, ledger authority, or index authority. This phase adds no routes, no durable writer, no index mutation, no storage mutation, no wallet/ledger mutation, and no secret access.
//! RO:TEST — tests/native_passport_phase12a_username_private_beta_registry_contract.rs.

use super::{B3DigestHex, ChallengeIdV1, DeviceIdV1, PassportIdV1, UsernameV1};

pub const NATIVE_PASSPORT_PHASE12A_LABEL: &str =
    "NATIVE_PASSPORT_PHASE12A_USERNAME_PRIVATE_BETA_REGISTRY_CONTRACT";

pub const PHASE12A_USERNAME_REGISTRY_CONTRACT_DOMAIN: &str =
    "native-passport/username-private-beta-registry-contract/v1";

pub const PHASE12A_USERNAME_REGISTRY_CONTRACT_VERSION: u16 = 1;

pub const PHASE12A_MAX_USERNAME_TRANSITION_TTL_MS: u64 = 120_000;

pub const PHASE12A_FORBIDDEN_USERNAME_REGISTRY_AUTHORITY_FLAGS: &[&str] = &[
    "username_as_real_or_legal_name_authority",
    "username_as_recovery_authority",
    "username_as_root_authority",
    "username_as_wallet_authority",
    "username_as_ledger_authority",
    "index_projection_authority",
    "durable_username_writer_execution",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeUsernameRegistryTransitionKind {
    Claim,
    Transfer,
    Release,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUsernameRegistryTransitionDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub transition_kind: NativeUsernameRegistryTransitionKind,
    pub username_input: &'static str,
    pub canonical_username: UsernameV1,
    pub passport_id: PassportIdV1,
    pub device_id: Option<DeviceIdV1>,
    pub current_owner_passport_id: Option<PassportIdV1>,
    pub recipient_passport_id: Option<PassportIdV1>,
    pub current_primary_username: Option<UsernameV1>,
    pub challenge_id: ChallengeIdV1,
    pub operation_body_hash: B3DigestHex,
    pub transition_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub fresh_proof_transcript_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub transition_requested_at_ms: u64,
    pub transition_expires_at_ms: u64,
    pub release_cooldown_until_ms: Option<u64>,
    pub proof_fresh: bool,
    pub root_or_admin_fresh_proof_present: bool,
    pub policy_reserved_name: bool,
    pub policy_rate_limited: bool,
    pub policy_existing_primary_for_passport: bool,
    pub policy_username_already_owned_by_other: bool,
    pub expects_single_writer_private_beta: bool,
    pub index_projection_only: bool,
    pub contract_only: bool,
    pub treats_username_as_real_or_legal_name_authority: bool,
    pub treats_username_as_recovery_authority: bool,
    pub treats_username_as_root_authority: bool,
    pub treats_username_as_wallet_or_ledger_authority: bool,
    pub treats_index_projection_as_authority: bool,
    pub requests_durable_username_writer_execution: bool,
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
pub struct NativeUsernameRegistryTransitionDecisionV1 {
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
    pub single_writer_private_beta_reviewed: bool,
    pub index_projection_only: bool,
    pub username_finalized: bool,
    pub durable_writer_called: bool,
    pub index_projection_mutated: bool,
    pub storage_mutated_inside_native: bool,
    pub wallet_or_ledger_mutated: bool,
    pub secret_material_exposed: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeUsernameRegistryTransitionReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    CanonicalUsernameRejected,
    CanonicalUsernameMismatch,
    UnsupportedTransitionShape,
    MissingCurrentOwner,
    CurrentOwnerMismatch,
    MissingRecipient,
    RecipientMatchesCurrentOwner,
    PassportAlreadyHasPrimaryUsername,
    UsernameAlreadyOwnedByOther,
    ReservedUsername,
    RateLimited,
    ReleaseCooldownActive,
    InvalidTransitionTimeWindow,
    TransitionTtlTooLong,
    MissingFreshProof,
    MissingRootOrAdminFreshProof,
    MissingSignerPublicKeyLabel,
    NotSingleWriterPrivateBeta,
    IndexProjectionTreatedAsAuthority,
    UsernameRegistryContractNotContractOnly,
    UnsafeUsernameRegistryAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUsernameRegistryContractPosture {
    pub phase_label: &'static str,
    pub username_registry_contract_added: bool,
    pub canonical_username_owner: &'static str,
    pub canonical_ron_naming_reuse: bool,
    pub one_primary_username_per_passport_review_added: bool,
    pub single_writer_private_beta_review_added: bool,
    pub idempotency_binding_added: bool,
    pub transfer_release_fresh_proof_review_added: bool,
    pub durable_username_writer_execution_added: bool,
    pub index_projection_mutation_added: bool,
    pub routes_added: bool,
    pub storage_mutation_inside_native_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub secret_key_access_added: bool,
    pub runtime_authority_changed: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_username_registry_contract_posture() -> NativeUsernameRegistryContractPosture {
    NativeUsernameRegistryContractPosture {
        phase_label: NATIVE_PASSPORT_PHASE12A_LABEL,
        username_registry_contract_added: true,
        canonical_username_owner: "ron-naming",
        canonical_ron_naming_reuse: true,
        one_primary_username_per_passport_review_added: true,
        single_writer_private_beta_review_added: true,
        idempotency_binding_added: true,
        transfer_release_fresh_proof_review_added: true,
        durable_username_writer_execution_added: false,
        index_projection_mutation_added: false,
        routes_added: false,
        storage_mutation_inside_native_added: false,
        wallet_or_ledger_mutation_added: false,
        secret_key_access_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE12A_FORBIDDEN_USERNAME_REGISTRY_AUTHORITY_FLAGS,
    }
}

pub fn review_native_username_registry_transition_contract(
    draft: NativeUsernameRegistryTransitionDraftV1,
) -> Result<NativeUsernameRegistryTransitionDecisionV1, NativeUsernameRegistryTransitionReviewError>
{
    if draft.contract_domain != PHASE12A_USERNAME_REGISTRY_CONTRACT_DOMAIN {
        return Err(NativeUsernameRegistryTransitionReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE12A_USERNAME_REGISTRY_CONTRACT_VERSION {
        return Err(NativeUsernameRegistryTransitionReviewError::ContractVersionMismatch);
    }

    let parsed_username = UsernameV1::parse(draft.username_input)
        .map_err(|_| NativeUsernameRegistryTransitionReviewError::CanonicalUsernameRejected)?;

    if parsed_username != draft.canonical_username {
        return Err(NativeUsernameRegistryTransitionReviewError::CanonicalUsernameMismatch);
    }

    validate_transition_shape(&draft)?;
    validate_time_and_proof(&draft)?;
    validate_no_unsafe_flags(&draft)?;

    let canonical_handle = draft.canonical_username.handle();

    Ok(NativeUsernameRegistryTransitionDecisionV1 {
        contract_domain: PHASE12A_USERNAME_REGISTRY_CONTRACT_DOMAIN,
        contract_version: PHASE12A_USERNAME_REGISTRY_CONTRACT_VERSION,
        transition_kind: draft.transition_kind,
        canonical_username: draft.canonical_username,
        canonical_handle,
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
        transition_requested_at_ms: draft.transition_requested_at_ms,
        transition_expires_at_ms: draft.transition_expires_at_ms,
        single_writer_private_beta_reviewed: true,
        index_projection_only: true,
        username_finalized: false,
        durable_writer_called: false,
        index_projection_mutated: false,
        storage_mutated_inside_native: false,
        wallet_or_ledger_mutated: false,
        secret_material_exposed: false,
        contract_only: true,
    })
}

fn validate_transition_shape(
    draft: &NativeUsernameRegistryTransitionDraftV1,
) -> Result<(), NativeUsernameRegistryTransitionReviewError> {
    if draft.policy_reserved_name {
        return Err(NativeUsernameRegistryTransitionReviewError::ReservedUsername);
    }

    if draft.policy_rate_limited {
        return Err(NativeUsernameRegistryTransitionReviewError::RateLimited);
    }

    if draft.policy_existing_primary_for_passport || draft.current_primary_username.is_some() {
        return Err(NativeUsernameRegistryTransitionReviewError::PassportAlreadyHasPrimaryUsername);
    }

    if draft.policy_username_already_owned_by_other {
        return Err(NativeUsernameRegistryTransitionReviewError::UsernameAlreadyOwnedByOther);
    }

    if !draft.expects_single_writer_private_beta {
        return Err(NativeUsernameRegistryTransitionReviewError::NotSingleWriterPrivateBeta);
    }

    if !draft.index_projection_only || draft.treats_index_projection_as_authority {
        return Err(NativeUsernameRegistryTransitionReviewError::IndexProjectionTreatedAsAuthority);
    }

    if !draft.contract_only {
        return Err(
            NativeUsernameRegistryTransitionReviewError::UsernameRegistryContractNotContractOnly,
        );
    }

    match draft.transition_kind {
        NativeUsernameRegistryTransitionKind::Claim => {
            if draft.current_owner_passport_id.is_some() || draft.recipient_passport_id.is_some() {
                return Err(
                    NativeUsernameRegistryTransitionReviewError::UnsupportedTransitionShape,
                );
            }
        }
        NativeUsernameRegistryTransitionKind::Transfer => {
            if draft.current_owner_passport_id.as_ref() != Some(&draft.passport_id) {
                return Err(NativeUsernameRegistryTransitionReviewError::CurrentOwnerMismatch);
            }

            let recipient = draft
                .recipient_passport_id
                .as_ref()
                .ok_or(NativeUsernameRegistryTransitionReviewError::MissingRecipient)?;

            if recipient == &draft.passport_id {
                return Err(
                    NativeUsernameRegistryTransitionReviewError::RecipientMatchesCurrentOwner,
                );
            }

            if !draft.root_or_admin_fresh_proof_present {
                return Err(
                    NativeUsernameRegistryTransitionReviewError::MissingRootOrAdminFreshProof,
                );
            }
        }
        NativeUsernameRegistryTransitionKind::Release => {
            if draft.current_owner_passport_id.as_ref() != Some(&draft.passport_id) {
                return Err(NativeUsernameRegistryTransitionReviewError::CurrentOwnerMismatch);
            }

            if draft.recipient_passport_id.is_some() {
                return Err(
                    NativeUsernameRegistryTransitionReviewError::UnsupportedTransitionShape,
                );
            }

            if !draft.root_or_admin_fresh_proof_present {
                return Err(
                    NativeUsernameRegistryTransitionReviewError::MissingRootOrAdminFreshProof,
                );
            }

            if let Some(cooldown_until_ms) = draft.release_cooldown_until_ms {
                if draft.transition_requested_at_ms < cooldown_until_ms {
                    return Err(NativeUsernameRegistryTransitionReviewError::ReleaseCooldownActive);
                }
            }
        }
    }

    Ok(())
}

fn validate_time_and_proof(
    draft: &NativeUsernameRegistryTransitionDraftV1,
) -> Result<(), NativeUsernameRegistryTransitionReviewError> {
    if draft.transition_expires_at_ms <= draft.transition_requested_at_ms {
        return Err(NativeUsernameRegistryTransitionReviewError::InvalidTransitionTimeWindow);
    }

    if draft.transition_expires_at_ms - draft.transition_requested_at_ms
        > PHASE12A_MAX_USERNAME_TRANSITION_TTL_MS
    {
        return Err(NativeUsernameRegistryTransitionReviewError::TransitionTtlTooLong);
    }

    if !draft.proof_fresh {
        return Err(NativeUsernameRegistryTransitionReviewError::MissingFreshProof);
    }

    if draft.signer_public_key_label.is_empty() {
        return Err(NativeUsernameRegistryTransitionReviewError::MissingSignerPublicKeyLabel);
    }

    Ok(())
}

fn validate_no_unsafe_flags(
    draft: &NativeUsernameRegistryTransitionDraftV1,
) -> Result<(), NativeUsernameRegistryTransitionReviewError> {
    if draft.treats_username_as_real_or_legal_name_authority
        || draft.treats_username_as_recovery_authority
        || draft.treats_username_as_root_authority
        || draft.treats_username_as_wallet_or_ledger_authority
        || draft.requests_durable_username_writer_execution
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
            NativeUsernameRegistryTransitionReviewError::UnsafeUsernameRegistryAuthorityFlag,
        );
    }

    Ok(())
}
