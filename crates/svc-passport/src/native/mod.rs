//! RO:WHAT — Native Passport feature-gated module entry point.
//! RO:WHY — P3 Identity & Keys; Concerns: SEC/GOV. Keeps Native Passport code isolated until each behavior is explicitly enabled.
//! RO:INTERACTS — native_plan Phase 0 source/vector contracts; future native DTO, proof, vault, and route modules.
//! RO:INVARIANTS — native behavior is phase-gated; no routes, ambient service authority, wallet/ledger mutation, or frontend secret custody.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with the `native-passport` Cargo feature.
//! RO:SECURITY — default builds do not expose `svc_passport::native`; feature builds keep secret material Rust-side and each authority surface explicit.
//! RO:TEST — tests/native_passport_phase1a_feature_gates.rs.

use crate::native_plan::CANONICAL_PASSPORT_PACKAGE_OWNER;

pub mod authorization;
pub mod capability_adapter;
pub mod capability_contract;
pub mod challenge;
pub mod client_command_inventory;
pub mod client_command_surface_acceptance;
pub mod client_redacted_command_dto_adapter;
pub mod client_status_command;
pub mod client_status_command_wiring_inspection;
pub mod delegated_enrollment;
pub mod desktop_platform_storage_inspection;
pub mod device_authorization_context;
pub mod device_authorization_signing;
pub mod device_identity;
pub mod device_key;
pub mod device_key_generation;
pub mod device_session_signing;
pub mod dto;
pub mod enrollment;
pub mod gateway_fixed_route_admission;
pub mod gateway_omnigate_route_mount_acceptance;
pub mod gateway_omnigate_routes;
pub mod local_status_inspection;
pub mod omnigate_fixed_route_admission;
pub mod operational_device_payload;
pub mod passport_id;
pub mod pin;
pub mod platform_bound_vault;
pub mod platform_bound_vault_v2;
pub mod platform_storage;
pub mod proof;
pub mod proof_signing;
pub mod proof_signing_adapter;
pub mod proof_verification;
pub mod proof_verification_adapter;
pub mod recovery;
pub mod recovery_identity;
pub mod recovery_mnemonic_indices;
pub mod recovery_mnemonic_words;
pub mod replay_consumption;
pub mod replay_consumption_adapter;
pub mod request_proof;
pub mod restore;
pub mod root_identity;
pub mod root_registration_proof_signing;
pub mod sealer;
pub mod secure_surface;
// Raw persistence remains private so callers cannot bypass the proof-gated
// server registry authority that will consume this store in the next slice.
// Authority-bearing server challenge/registry layers remain private until
// durable one-time challenge issuance and replay consumption compose them.
#[allow(dead_code)]
mod server_challenge_issuer;
#[allow(dead_code)]
mod server_challenge_runtime;
#[allow(dead_code)]
mod server_challenge_store;
#[allow(dead_code)]
mod server_device_registration_runtime;
mod server_device_session_runtime;
#[allow(dead_code)]
mod server_registry_runtime;
#[allow(dead_code)]
mod server_registry_store;
#[allow(dead_code)]
mod server_root_registration_coordinator;
#[allow(dead_code)]
mod server_root_registration_txn_store;
mod server_runtime_mount;

pub use server_runtime_mount::{
    NativePassportServerRuntimeMountConfigV1, NativePassportServerRuntimeMountError,
};

pub(crate) use server_device_registration_runtime::{
    register_device_authorization_durable, NativePassportServerDeviceRegistrationDispositionV1,
    NativePassportServerDeviceRegistrationError,
};

pub(crate) use server_device_session_runtime::{
    issue_device_session_challenge_service, submit_device_session_proof_service,
    NativePassportDeviceSessionServiceError,
};

pub(crate) use server_runtime_mount::{
    issue_register_root_challenge_durable, preflight_native_passport_server_runtime_mount,
    submit_register_root_proof_durable, NativePassportRegisterRootSubmitDispositionV1,
};
pub mod status;
pub mod username;
pub mod username_index_projection;
pub mod username_registry;
pub mod username_registry_adapter;
pub mod vault;
pub mod vault_crypto;

pub use authorization::{
    native_passport_authorization_posture, review_device_authorization_draft,
    validate_device_authorization_grant,
    DeviceAuthorizationDraftV1 as NativeDeviceAuthorizationDraftV1, DeviceAuthorizationGrantV1,
    DeviceAuthorizationReviewError, NativePassportAuthorizationPosture, RootPassportDescriptorV1,
    NATIVE_PASSPORT_PHASE3A_LABEL, NATIVE_PASSPORT_PHASE3B_LABEL, PHASE3A_ALLOWED_DEVICE_SCOPES,
    PHASE3A_FORBIDDEN_AUTHORITY_FLAGS,
};

pub use passport_id::{derive_native_passport_id_v1, PHYSICAL_M1_PASSPORT_ID_DERIVATION_LABEL};

pub use root_identity::{
    derive_native_root_public_identity_v1, NativeRootIdentityDerivationError,
    PHYSICAL_M1_ROOT_IDENTITY_DERIVATION_LABEL,
};

pub use root_registration_proof_signing::{
    sign_native_root_registration_proof_v1, NativeRootRegistrationProofSigningError,
    NativeRootRegistrationProofSigningOutputV1, PHYSICAL_M1_ROOT_REGISTRATION_PROOF_SIGNING_LABEL,
};

pub use device_authorization_context::{
    build_root_admin_desktop_device_authorization_payload_v1,
    NativeDeviceAuthorizationContextError, NativeRootAdminDesktopAuthorizationContextV1,
    PHYSICAL_M1_DEVICE_AUTHORIZATION_CONTEXT_LABEL,
};

pub use device_authorization_signing::{
    sign_native_device_authorization_v1, NativeDeviceAuthorizationSigningError,
    PHYSICAL_M1_DEVICE_AUTHORIZATION_SIGNING_LABEL,
};

pub use device_session_signing::{
    sign_native_device_session_proof_v1, NativeDeviceSessionProofSigningError,
    PHYSICAL_M1_DEVICE_SESSION_PROOF_SIGNING_LABEL,
};

pub use device_identity::{
    derive_native_device_id_v1, derive_native_device_public_identity_v1,
    NativeDeviceIdentityDerivationError, NativeDevicePublicIdentityV1,
    DEVICE_ID_V1_SIGNING_SEED_BYTES, PHYSICAL_M1_DEVICE_IDENTITY_DERIVATION_LABEL,
};

pub use device_key_generation::{
    generate_native_device_signing_seed_v1_with_random, NativeDeviceKeyGenerationError,
    NativeDeviceKeyRandomSource, PHYSICAL_M1_DEVICE_KEY_GENERATION_LABEL,
};

pub use device_key::{
    native_passport_device_key_posture, review_native_device_key_draft,
    validate_native_device_key_descriptor, NativeDeviceKeyDescriptorV1, NativeDeviceKeyDraftV1,
    NativeDeviceKeyPurpose, NativeDeviceKeyReviewError, NativePassportDeviceKeyPosture,
    NATIVE_PASSPORT_PHASE4B_LABEL, PHASE4B_ALLOWED_DEVICE_KEY_PURPOSES, PHASE4B_DEVICE_KEY_DOMAIN,
    PHASE4B_FORBIDDEN_DEVICE_KEY_AUTHORITY_FLAGS,
};

pub use dto::{
    B3DigestHex, ChallengeIdV1, DeviceAuthorizationDraftV1, DeviceClass, DeviceIdV1,
    Ed25519PublicKeyHex, NativePassportDtoError, NativePassportScope, PassportIdV1,
    NATIVE_PASSPORT_PHASE1B_LABEL, NATIVE_PASSPORT_READ_ONLY_SCOPE_CEILING,
    NATIVE_PASSPORT_UNSAFE_SCOPE_STRINGS, PHASE1B_FORBIDDEN_DTO_FIELDS,
};

pub use challenge::{
    native_passport_challenge_contract_posture, review_native_passport_challenge_contract_draft,
    validate_native_passport_challenge_contract_descriptor, NativeChallengeSignatureAlgorithm,
    NativeChallengeTranscriptCodec, NativePassportChallengeContractDescriptorV1,
    NativePassportChallengeContractDraftV1, NativePassportChallengeContractPosture,
    NativePassportChallengeContractReviewError, NativePassportChallengePurpose,
    NATIVE_PASSPORT_PHASE8A_LABEL, PHASE8A_ALLOWED_CHALLENGE_SCOPES,
    PHASE8A_FORBIDDEN_CHALLENGE_AUTHORITY_FLAGS, PHASE8A_MAX_CHALLENGE_TTL_MS,
    PHASE8A_MAX_CLOCK_SKEW_MS, PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
    PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION, PHASE8A_REQUIRED_SERVICE_SIGNATURE_ALGORITHM,
    PHASE8A_REQUIRED_TRANSCRIPT_CODEC,
};

pub use proof::{
    native_passport_proof_contract_posture, review_native_passport_proof_contract_draft,
    validate_native_passport_proof_contract_descriptor, NativePassportProofAuthority,
    NativePassportProofContractDescriptorV1, NativePassportProofContractDraftV1,
    NativePassportProofContractPosture, NativePassportProofContractReviewError,
    NativePassportProofKind, NativeProofSignatureAlgorithm, NATIVE_PASSPORT_PHASE8B_LABEL,
    PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_FORBIDDEN_PROOF_AUTHORITY_FLAGS,
    PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
    PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC, PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
};

pub use request_proof::{
    native_passport_request_proof_contract_posture,
    review_native_passport_request_proof_contract_draft,
    validate_native_passport_request_proof_contract_descriptor, NativePassportRequestMethod,
    NativePassportRequestProofContractDescriptorV1, NativePassportRequestProofContractDraftV1,
    NativePassportRequestProofContractPosture, NativePassportRequestProofContractReviewError,
    NATIVE_PASSPORT_PHASE8C_LABEL, PHASE8C_ALLOWED_REQUEST_METHODS,
    PHASE8C_FORBIDDEN_REQUEST_PROOF_AUTHORITY_FLAGS, PHASE8C_MAX_CLOCK_SKEW_MS,
    PHASE8C_MAX_REQUEST_TTL_MS, PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN,
    PHASE8C_PROOF_REQUEST_CONTRACT_VERSION, PHASE8C_REQUIRED_REQUEST_TRANSCRIPT_CODEC,
};

pub use proof_signing::{
    native_proof_signing_contract_boundary_posture, review_native_proof_signing_contract_boundary,
    NativeProofSigningContractBoundaryDecisionV1, NativeProofSigningContractBoundaryDraftV1,
    NativeProofSigningContractBoundaryPosture, NativeProofSigningContractBoundaryReviewError,
    NativeProofSigningOperationKind, NativeProofSigningUnlockState, NATIVE_PASSPORT_PHASE9A_LABEL,
    PHASE9A_FORBIDDEN_PROOF_SIGNING_AUTHORITY_FLAGS, PHASE9A_PROOF_SIGNING_BOUNDARY_DOMAIN,
    PHASE9A_PROOF_SIGNING_BOUNDARY_VERSION,
};

pub use proof_signing_adapter::{
    execute_native_proof_signing_adapter, native_proof_signing_adapter_posture,
    NativeLocalProofSigningAdapter, NativeProofSignedPayloadHex, NativeProofSigningAdapterDraftV1,
    NativeProofSigningAdapterEnvelopeV1, NativeProofSigningAdapterPosture,
    NativeProofSigningAdapterRequestV1, NativeProofSigningAdapterReviewError,
    NATIVE_PASSPORT_PHASE9B_LABEL, PHASE9B_FORBIDDEN_SIGNING_ADAPTER_AUTHORITY_FLAGS,
    PHASE9B_PROOF_SIGNING_ADAPTER_DOMAIN, PHASE9B_PROOF_SIGNING_ADAPTER_VERSION,
};

pub use proof_verification::{
    native_proof_verification_contract_boundary_posture,
    review_native_proof_verification_contract_boundary,
    NativeProofVerificationContractBoundaryDecisionV1,
    NativeProofVerificationContractBoundaryDraftV1, NativeProofVerificationContractBoundaryPosture,
    NativeProofVerificationContractBoundaryReviewError, NATIVE_PASSPORT_PHASE10A_LABEL,
    PHASE10A_FORBIDDEN_PROOF_VERIFICATION_AUTHORITY_FLAGS,
    PHASE10A_PROOF_VERIFICATION_BOUNDARY_DOMAIN, PHASE10A_PROOF_VERIFICATION_BOUNDARY_VERSION,
};

pub use proof_verification_adapter::{
    execute_native_proof_verification_adapter, native_proof_verification_adapter_posture,
    NativeLocalProofVerificationAdapter, NativeProofVerificationAdapterDraftV1,
    NativeProofVerificationAdapterEnvelopeV1, NativeProofVerificationAdapterEvidenceV1,
    NativeProofVerificationAdapterPosture, NativeProofVerificationAdapterRequestV1,
    NativeProofVerificationAdapterReviewError, NATIVE_PASSPORT_PHASE10B_LABEL,
    PHASE10B_FORBIDDEN_VERIFICATION_ADAPTER_AUTHORITY_FLAGS,
    PHASE10B_PROOF_VERIFICATION_ADAPTER_DOMAIN, PHASE10B_PROOF_VERIFICATION_ADAPTER_VERSION,
};

pub use replay_consumption::{
    native_replay_challenge_consumption_contract_posture,
    review_native_replay_challenge_consumption_contract,
    NativeReplayChallengeConsumptionContractDecisionV1,
    NativeReplayChallengeConsumptionContractDraftV1,
    NativeReplayChallengeConsumptionContractPosture,
    NativeReplayChallengeConsumptionContractReviewError, NATIVE_PASSPORT_PHASE11A_LABEL,
    PHASE11A_FORBIDDEN_CONSUMPTION_AUTHORITY_FLAGS,
    PHASE11A_REPLAY_CHALLENGE_CONSUMPTION_CONTRACT_DOMAIN,
    PHASE11A_REPLAY_CHALLENGE_CONSUMPTION_CONTRACT_VERSION,
};

pub use replay_consumption_adapter::{
    execute_native_replay_challenge_consumption_adapter,
    native_replay_challenge_consumption_adapter_posture,
    NativeLocalReplayChallengeConsumptionAdapter, NativeReplayChallengeConsumptionAdapterDraftV1,
    NativeReplayChallengeConsumptionAdapterEnvelopeV1,
    NativeReplayChallengeConsumptionAdapterEvidenceV1,
    NativeReplayChallengeConsumptionAdapterPosture,
    NativeReplayChallengeConsumptionAdapterRequestV1,
    NativeReplayChallengeConsumptionAdapterReviewError, NATIVE_PASSPORT_PHASE11B_LABEL,
    PHASE11B_FORBIDDEN_CONSUMPTION_ADAPTER_AUTHORITY_FLAGS,
    PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_DOMAIN,
    PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_VERSION,
};

pub use capability_contract::{
    native_device_bound_capability_contract_posture,
    review_native_device_bound_capability_contract, NativeDeviceBoundCapabilityContractDecisionV1,
    NativeDeviceBoundCapabilityContractDraftV1, NativeDeviceBoundCapabilityContractPosture,
    NativeDeviceBoundCapabilityContractReviewError, NATIVE_PASSPORT_PHASE11C_LABEL,
    PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_DOMAIN,
    PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_VERSION,
    PHASE11C_FORBIDDEN_CAPABILITY_CONTRACT_AUTHORITY_FLAGS, PHASE11C_MAX_CAPABILITY_TTL_MS,
    PHASE11C_MAX_CLOCK_SKEW_MS,
};

pub use capability_adapter::{
    execute_native_device_bound_capability_adapter, native_device_bound_capability_adapter_posture,
    NativeDeviceBoundCapabilityAdapterDraftV1, NativeDeviceBoundCapabilityAdapterEnvelopeV1,
    NativeDeviceBoundCapabilityAdapterEvidenceV1, NativeDeviceBoundCapabilityAdapterPosture,
    NativeDeviceBoundCapabilityAdapterRequestV1, NativeDeviceBoundCapabilityAdapterReviewError,
    NativeLocalDeviceBoundCapabilityAdapter, NATIVE_PASSPORT_PHASE11D_LABEL,
    PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER_DOMAIN,
    PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER_VERSION,
    PHASE11D_FORBIDDEN_CAPABILITY_ADAPTER_AUTHORITY_FLAGS,
};

pub use username_registry::{
    native_username_registry_contract_posture, review_native_username_registry_transition_contract,
    NativeUsernameRegistryContractPosture, NativeUsernameRegistryTransitionDecisionV1,
    NativeUsernameRegistryTransitionDraftV1, NativeUsernameRegistryTransitionKind,
    NativeUsernameRegistryTransitionReviewError, NATIVE_PASSPORT_PHASE12A_LABEL,
    PHASE12A_FORBIDDEN_USERNAME_REGISTRY_AUTHORITY_FLAGS, PHASE12A_MAX_USERNAME_TRANSITION_TTL_MS,
    PHASE12A_USERNAME_REGISTRY_CONTRACT_DOMAIN, PHASE12A_USERNAME_REGISTRY_CONTRACT_VERSION,
};

pub use username_registry_adapter::{
    execute_native_username_registry_adapter, native_username_registry_adapter_posture,
    NativeLocalUsernameRegistryAdapter, NativeUsernameRegistryAdapterDraftV1,
    NativeUsernameRegistryAdapterEnvelopeV1, NativeUsernameRegistryAdapterEvidenceV1,
    NativeUsernameRegistryAdapterPosture, NativeUsernameRegistryAdapterRequestV1,
    NativeUsernameRegistryAdapterReviewError, NATIVE_PASSPORT_PHASE12B_LABEL,
    PHASE12B_FORBIDDEN_USERNAME_REGISTRY_ADAPTER_AUTHORITY_FLAGS,
    PHASE12B_USERNAME_REGISTRY_ADAPTER_DOMAIN, PHASE12B_USERNAME_REGISTRY_ADAPTER_VERSION,
};

pub use username_index_projection::{
    native_username_index_projection_posture, review_native_username_index_projection_contract,
    NativeUsernameIndexProjectionDecisionV1, NativeUsernameIndexProjectionDraftV1,
    NativeUsernameIndexProjectionKind, NativeUsernameIndexProjectionPosture,
    NativeUsernameIndexProjectionReviewError, NATIVE_PASSPORT_PHASE12C_LABEL,
    PHASE12C_FORBIDDEN_USERNAME_INDEX_PROJECTION_AUTHORITY_FLAGS,
    PHASE12C_USERNAME_INDEX_PROJECTION_DOMAIN, PHASE12C_USERNAME_INDEX_PROJECTION_VERSION,
};

pub use local_status_inspection::{
    native_local_status_inspection_posture, review_native_local_status_inspection,
    NativeLocalStatusInspectionDraftV1, NativeLocalStatusInspectionPosture,
    NativeLocalStatusInspectionReviewError, NativeLocalStatusInspectionSnapshotV1,
    NATIVE_PASSPORT_PHASE13_LABEL, PHASE13_FORBIDDEN_LOCAL_STATUS_AUTHORITY_FLAGS,
    PHASE13_LOCAL_STATUS_INSPECTION_DOMAIN, PHASE13_LOCAL_STATUS_INSPECTION_VERSION,
    PHASE13_REDACTED_VALUE,
};

pub use gateway_omnigate_routes::{
    native_gateway_omnigate_route_contract_posture,
    review_native_gateway_omnigate_problem_envelope, review_native_gateway_omnigate_route_catalog,
    NativeGatewayOmnigateProblemEnvelopeDraftV1, NativeGatewayOmnigateProblemEnvelopeV1,
    NativeGatewayOmnigateRouteCatalogDecisionV1, NativeGatewayOmnigateRouteContractPosture,
    NativeGatewayOmnigateRouteKind, NativeGatewayOmnigateRouteMethod,
    NativeGatewayOmnigateRouteReviewError, NativeGatewayOmnigateRouteSpecV1,
    NATIVE_PASSPORT_PHASE14A_LABEL, PHASE14A_FORBIDDEN_GATEWAY_OMNIGATE_AUTHORITY_FLAGS,
    PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG, PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN,
    PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION, PHASE14A_MAX_BODY_CAP_BYTES,
    PHASE14A_MAX_DEADLINE_MS,
};

pub use omnigate_fixed_route_admission::{
    native_omnigate_fixed_route_admission_posture, review_native_omnigate_fixed_route_admission,
    NativeOmnigateFixedRouteAdmissionDecisionV1, NativeOmnigateFixedRouteAdmissionDraftV1,
    NativeOmnigateFixedRouteAdmissionPosture, NativeOmnigateFixedRouteAdmissionReviewError,
    NATIVE_PASSPORT_PHASE14B_LABEL, PHASE14B_FORBIDDEN_ADMISSION_AUTHORITY_FLAGS,
    PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_DOMAIN,
    PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_VERSION,
};

pub use gateway_fixed_route_admission::{
    native_gateway_fixed_route_admission_posture, review_native_gateway_fixed_route_admission,
    NativeGatewayFixedRouteAdmissionDecisionV1, NativeGatewayFixedRouteAdmissionDraftV1,
    NativeGatewayFixedRouteAdmissionPosture, NativeGatewayFixedRouteAdmissionReviewError,
    NATIVE_PASSPORT_PHASE14C_LABEL, PHASE14C_FORBIDDEN_GATEWAY_ADMISSION_AUTHORITY_FLAGS,
    PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_DOMAIN, PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_VERSION,
};

pub use gateway_omnigate_route_mount_acceptance::{
    native_gateway_omnigate_route_mount_acceptance_fixed_route_count,
    native_gateway_omnigate_route_mount_acceptance_fixed_route_kinds,
    native_gateway_omnigate_route_mount_acceptance_fixed_route_methods,
    native_gateway_omnigate_route_mount_acceptance_posture,
    review_native_gateway_omnigate_route_mount_acceptance,
    NativeGatewayOmnigateRouteMountAcceptanceDecisionV1,
    NativeGatewayOmnigateRouteMountAcceptanceDraftV1,
    NativeGatewayOmnigateRouteMountAcceptancePosture,
    NativeGatewayOmnigateRouteMountAcceptanceReviewError, NATIVE_PASSPORT_PHASE14D_LABEL,
    PHASE14D_FORBIDDEN_ROUTE_MOUNT_AUTHORITY_FLAGS,
    PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE_DOMAIN,
    PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE_VERSION,
};

pub use client_status_command::{
    native_client_status_command_posture, review_native_client_status_command,
    NativeClientStatusCapabilityState, NativeClientStatusCommandDecisionV1,
    NativeClientStatusCommandDraftV1, NativeClientStatusCommandPosture,
    NativeClientStatusCommandReviewError, NativeClientStatusLockState,
    NATIVE_PASSPORT_PHASE15A_LABEL, PHASE15A_CLIENT_STATUS_COMMAND_DOMAIN,
    PHASE15A_CLIENT_STATUS_COMMAND_VERSION, PHASE15A_EXPECTED_DESKTOP_COMMANDS,
    PHASE15A_FORBIDDEN_DESKTOP_COMMANDS, PHASE15A_FORBIDDEN_STATUS_COMMAND_AUTHORITY_FLAGS,
    PHASE15A_PASSPORT_STATUS_COMMAND,
};

pub use client_command_inventory::{
    native_client_command_inventory_command_names,
    native_client_command_inventory_forbidden_command_names,
    native_client_command_inventory_posture, native_client_command_inventory_status_command_name,
    review_native_client_command_inventory, NativeClientCommandCategory,
    NativeClientCommandInventoryDecisionV1, NativeClientCommandInventoryDraftV1,
    NativeClientCommandInventoryEntryV1, NativeClientCommandInventoryPosture,
    NativeClientCommandInventoryReviewError, NativeClientCommandRisk,
    NativeClientForbiddenCommandEntryV1, NATIVE_PASSPORT_PHASE15B_LABEL,
    PHASE15B_CLIENT_COMMAND_DENYLIST, PHASE15B_CLIENT_COMMAND_INVENTORY,
    PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN, PHASE15B_CLIENT_COMMAND_INVENTORY_VERSION,
    PHASE15B_EXPECTED_COMMAND_COUNT, PHASE15B_EXPECTED_FORBIDDEN_COMMAND_COUNT,
    PHASE15B_FORBIDDEN_CLIENT_COMMAND_AUTHORITY_FLAGS,
};

pub use client_redacted_command_dto_adapter::{
    adapt_native_client_status_to_redacted_command_dto,
    native_client_redacted_command_dto_adapter_posture,
    NativeClientRedactedCommandDtoAdapterDraftV1, NativeClientRedactedCommandDtoAdapterPosture,
    NativeClientRedactedCommandDtoAdapterReviewError, NativeClientRedactedCommandDtoEnvelopeV1,
    NATIVE_PASSPORT_PHASE15C_LABEL, PHASE15C_FORBIDDEN_REDACTED_DTO_ADAPTER_FLAGS,
    PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_DOMAIN, PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_VERSION,
    PHASE15C_REDACTED_DTO_TARGET,
};

pub use client_command_surface_acceptance::{
    native_client_command_surface_acceptance_posture,
    review_native_client_command_surface_acceptance,
    NativeClientCommandSurfaceAcceptanceDecisionV1, NativeClientCommandSurfaceAcceptanceDraftV1,
    NativeClientCommandSurfaceAcceptancePosture, NativeClientCommandSurfaceAcceptanceReviewError,
    NATIVE_PASSPORT_PHASE15D_LABEL, PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_DOMAIN,
    PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_VERSION, PHASE15D_FORBIDDEN_COMMAND_SURFACE_FLAGS,
};

pub use client_status_command_wiring_inspection::{
    native_client_status_command_wiring_inspection_posture,
    review_native_client_status_command_wiring_inspection,
    NativeClientStatusCommandWiringInspectionDecisionV1,
    NativeClientStatusCommandWiringInspectionDraftV1,
    NativeClientStatusCommandWiringInspectionPosture,
    NativeClientStatusCommandWiringInspectionReviewError, NATIVE_PASSPORT_PHASE15E_LABEL,
    PHASE15E_APP_STATE_PATH, PHASE15E_COMMAND_HANDLER_REGISTRY_PATH,
    PHASE15E_COMMAND_MODULE_REGISTRY_PATH, PHASE15E_EXISTING_IDENTITY_COMMAND_NAME,
    PHASE15E_EXISTING_IDENTITY_COMMAND_PATH, PHASE15E_FORBIDDEN_WIRING_INSPECTION_FLAGS,
    PHASE15E_PROPOSED_PASSPORT_COMMAND_PATH, PHASE15E_STATUS_COMMAND_WIRING_INSPECTION_DOMAIN,
    PHASE15E_STATUS_COMMAND_WIRING_INSPECTION_VERSION, PHASE15E_TAURI_CARGO_PATH,
    PHASE15E_TAURI_CRATE_ROOT,
};

pub use enrollment::{
    native_passport_new_device_enrollment_posture,
    review_native_new_device_enrollment_contract_draft,
    validate_native_new_device_enrollment_contract_descriptor, NativeEnrollmentIntent,
    NativeEnrollmentSource, NativeNewDeviceEnrollmentContractDescriptorV1,
    NativeNewDeviceEnrollmentContractDraftV1, NativeNewDeviceEnrollmentContractReviewError,
    NativePassportNewDeviceEnrollmentPosture, NATIVE_PASSPORT_PHASE7B_LABEL,
    PHASE7B_ALLOWED_ENROLLMENT_SOURCES, PHASE7B_ALLOWED_TARGET_DEVICE_CLASSES,
    PHASE7B_FORBIDDEN_ENROLLMENT_AUTHORITY_FLAGS, PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN,
    PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION, PHASE7B_REQUIRED_ENROLLMENT_INTENTS,
    PHASE7B_REQUIRED_ENROLLMENT_SOURCES,
};

pub use delegated_enrollment::{
    native_passport_delegated_enrollment_posture,
    review_native_delegated_enrollment_contract_draft,
    validate_native_delegated_enrollment_contract_descriptor, NativeDelegatedApprovingAuthority,
    NativeDelegatedChallengePlaceholder, NativeDelegatedEnrollmentContractDescriptorV1,
    NativeDelegatedEnrollmentContractDraftV1, NativeDelegatedEnrollmentContractReviewError,
    NativeDelegatedEnrollmentSource, NativeDelegatedPolicyMetadata,
    NativeDelegatedProofPlaceholder, NativePassportDelegatedEnrollmentPosture,
    NATIVE_PASSPORT_PHASE7C_LABEL, PHASE7C_ALLOWED_TARGET_DEVICE_CLASSES,
    PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_DOMAIN, PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_VERSION,
    PHASE7C_FORBIDDEN_DELEGATED_AUTHORITY_FLAGS, PHASE7C_REQUIRED_APPROVING_AUTHORITIES,
    PHASE7C_REQUIRED_CHALLENGE_PLACEHOLDERS, PHASE7C_REQUIRED_DELEGATED_SOURCES,
    PHASE7C_REQUIRED_POLICY_METADATA, PHASE7C_REQUIRED_PROOF_PLACEHOLDERS,
};

pub use pin::{
    native_passport_pin_unlock_posture, review_native_pin_unlock_contract_draft,
    validate_native_pin_unlock_contract_descriptor, NativePassportPinUnlockPosture,
    NativePinPolicyV1, NativePinUnlockContractDescriptorV1, NativePinUnlockContractDraftV1,
    NativePinUnlockContractReviewError, NATIVE_PASSPORT_PHASE6B_LABEL, PHASE6B_COOLDOWN_SECONDS,
    PHASE6B_FORBIDDEN_PIN_UNLOCK_AUTHORITY_FLAGS, PHASE6B_MAX_PIN_LENGTH,
    PHASE6B_MAX_UNLOCK_ATTEMPTS, PHASE6B_MIN_PIN_LENGTH, PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN,
    PHASE6B_PIN_UNLOCK_CONTRACT_VERSION, PHASE6B_REQUIRED_PIN_POLICY,
};
pub use recovery::{
    native_passport_recovery_root_posture, review_native_recovery_root_draft,
    validate_native_recovery_root_descriptor, NativePassportRecoveryRootPosture,
    NativeRecoveryRootDescriptorV1, NativeRecoveryRootDraftV1, NativeRecoveryRootReviewError,
    NativeRecoveryRootScope, NATIVE_PASSPORT_PHASE4A_LABEL, PHASE4A_ALLOWED_RECOVERY_ROOT_SCOPES,
    PHASE4A_FORBIDDEN_RECOVERY_ROOT_AUTHORITY_FLAGS, PHASE4A_RECOVERY_ROOT_DOMAIN,
};
pub use recovery_identity::{
    derive_native_recovery_public_identity_v1, sign_native_recovery_device_authorization_v1,
    sign_native_recovery_root_registration_proof_v1, NativeRecoveryDeviceAuthorizationSigningError,
    NativeRecoveryIdentityDerivationError, NativeRecoveryRootRegistrationProofSigningError,
    PHYSICAL_M1_RECOVERY_DEVICE_AUTHORIZATION_SIGNING_LABEL,
    PHYSICAL_M1_RECOVERY_IDENTITY_DERIVATION_LABEL,
    PHYSICAL_M1_RECOVERY_ROOT_REGISTRATION_PROOF_SIGNING_LABEL,
};
pub use recovery_mnemonic_indices::{
    derive_native_recovery_mnemonic_indices, NativeRecoveryMnemonicIndicesError,
    NativeRecoveryMnemonicIndicesV1, ONBOARDING_PHASE6B1_NATIVE_LABEL,
    PHASE6B1_RECOVERY_CHECKSUM_BITS, PHASE6B1_RECOVERY_ENTROPY_BYTES,
    PHASE6B1_RECOVERY_FINGERPRINT_DOMAIN, PHASE6B1_RECOVERY_FINGERPRINT_HEX_LENGTH,
    PHASE6B1_RECOVERY_WORDLIST_LENGTH, PHASE6B1_RECOVERY_WORD_COUNT,
    PHASE6B1_RECOVERY_WORD_INDEX_BITS,
};
pub use recovery_mnemonic_words::{
    bip39_english_wordlist_count, with_native_recovery_mnemonic_phrase,
    NativeRecoveryMnemonicWordsError, ONBOARDING_PHASE6B2A_NATIVE_LABEL,
    PHASE6B2A_BIP39_ENGLISH_WORDLIST_SHA256,
};

pub use restore::{
    native_passport_restore_posture, review_native_restore_contract_draft,
    validate_native_restore_contract_descriptor, NativePassportRestorePosture,
    NativeRestoreContractDescriptorV1, NativeRestoreContractDraftV1,
    NativeRestoreContractReviewError, NativeRestoreIntent, NativeRestoreSource,
    NATIVE_PASSPORT_PHASE7A_LABEL, PHASE7A_ALLOWED_RESTORE_SOURCES,
    PHASE7A_FORBIDDEN_RESTORE_AUTHORITY_FLAGS, PHASE7A_REQUIRED_RESTORE_INTENTS,
    PHASE7A_RESTORE_CONTRACT_DOMAIN, PHASE7A_RESTORE_CONTRACT_VERSION,
};
pub use sealer::{
    native_passport_platform_sealer_posture, review_native_platform_sealer_contract_draft,
    validate_native_platform_sealer_contract_descriptor, NativePassportPlatformSealerPosture,
    NativePlatformFamily, NativePlatformSealerContractDescriptorV1,
    NativePlatformSealerContractDraftV1, NativePlatformSealerContractReviewError,
    NativeSecureCompartment, NATIVE_PASSPORT_PHASE5A_LABEL,
    PHASE5A_FORBIDDEN_PLATFORM_SEALER_AUTHORITY_FLAGS, PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
    PHASE5A_REQUIRED_SECURE_COMPARTMENTS,
};
pub use secure_surface::{
    native_passport_secure_surface_posture, review_native_secure_surface_contract_draft,
    validate_native_secure_surface_contract_descriptor, NativePassportSecureSurfacePosture,
    NativeSecureSurfaceCompartmentBindingV1, NativeSecureSurfaceContractDescriptorV1,
    NativeSecureSurfaceContractDraftV1, NativeSecureSurfaceContractReviewError,
    NATIVE_PASSPORT_PHASE5B_LABEL, PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL,
    PHASE5B_FORBIDDEN_SECURE_SURFACE_AUTHORITY_FLAGS, PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL,
    PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS, PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
};
pub use vault::{
    native_passport_vault_header_posture, review_native_two_compartment_vault_header_draft,
    validate_native_two_compartment_vault_header_descriptor, NativePassportVaultHeaderPosture,
    NativeTwoCompartmentVaultHeaderDescriptorV1, NativeTwoCompartmentVaultHeaderDraftV1,
    NativeVaultCompartmentHeaderV1, NativeVaultHeaderReviewError, NATIVE_PASSPORT_PHASE6A_LABEL,
    PHASE6A_AEAD_ALGORITHM_LABEL, PHASE6A_AEAD_NONCE_LEN, PHASE6A_AEAD_TAG_LEN,
    PHASE6A_AUTHENTICATED_HEADER_LABEL, PHASE6A_FORBIDDEN_VAULT_HEADER_AUTHORITY_FLAGS,
    PHASE6A_KDF_ALGORITHM_LABEL, PHASE6A_KDF_SALT_LEN, PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS,
    PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN, PHASE6A_VAULT_HEADER_VERSION,
};

pub use status::{
    native_passport_redacted_status, redacted_value_for, NativePassportRedactedStatusReadiness,
    NativePassportRedactedStatusV1, NATIVE_PASSPORT_PHASE1E_LABEL,
    NATIVE_PASSPORT_REDACTED_STATUS_SCHEMA_V1, NATIVE_PASSPORT_REDACTED_VALUE,
    PHASE1E_REDACTED_FIELD_NAMES,
};

pub use username::{
    native_passport_username_reuse_posture, parse_optional_username_handle, HandleV1,
    NativePassportUsernameReusePosture, UsernameParseError, UsernameV1,
    NATIVE_PASSPORT_PHASE2B_LABEL as NATIVE_PASSPORT_USERNAME_PHASE2B_LABEL,
    OPTIONAL_USERNAME_HANDLE_STATUS, PHASE2B_FORBIDDEN_HANDLE_MEANINGS,
};

/// Cargo feature that exposes the Native Passport module surface.
pub const NATIVE_PASSPORT_FEATURE_NAME: &str = "native-passport";

/// Phase label for the feature isolation gate.
pub const NATIVE_PASSPORT_PHASE1A_LABEL: &str =
    "NATIVE_PASSPORT_PHASE1A_FEATURE_ISOLATION_NATIVE_MODULE_GATES";

/// Native module surface exposed in Phase 1A.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePassportSurface {
    /// Read-only posture and compile-gate facts.
    FeaturePosture,
    /// Phase 0 route/vector contracts remain the source of truth.
    Phase0Contracts,
    /// Feature-gated Native Passport DTO/value types.
    NativeDtoTypes,
    /// Feature-gated redacted Native Passport status snapshot.
    NativeStatusRedaction,
    /// Feature-gated reuse of ron-naming optional username/handle DTOs.
    NativeUsernameHandles,
    /// Feature-gated root/device authorization DTO foundations.
    RootDeviceAuthorizationDtos,
    /// Feature-gated pure root/device authorization review.
    PureAuthorizationReview,
    /// Feature-gated native recovery-root DTO foundations.
    NativeRecoveryRootDto,
    /// Feature-gated native device-key DTO foundations.
    NativeDeviceKeyDto,
    /// Feature-gated native platform sealer contract DTO foundations.
    PlatformSealerContractDto,
    /// Feature-gated native secure-surface compartment contract DTO foundations.
    SecureSurfaceCompartmentContract,
    /// Feature-gated native two-compartment vault header DTO foundations.
    TwoCompartmentVaultHeaderDto,
    /// Feature-gated native PIN unlock contract DTO foundations.
    PinUnlockContractDto,
    /// Feature-gated native restore contract DTO foundations.
    RestoreContractDto,
    /// Feature-gated native new-device enrollment contract DTO foundations.
    NewDeviceEnrollmentContractDto,
    /// Feature-gated native delegated enrollment contract DTO foundations.
    DelegatedEnrollmentContractDto,
    /// Feature-gated native purpose-bound challenge contract DTO foundations.
    PurposeBoundChallengeContractDto,
    /// Feature-gated native proof contract DTO foundations.
    ProofContractDto,
    /// Feature-gated native request-proof contract DTO foundations.
    RequestProofContractDto,
    /// Feature-gated native proof-signing contract boundary DTO foundations.
    ProofSigningContractBoundaryDto,
    /// Feature-gated native proof-signing adapter DTO foundations.
    ProofSigningAdapterDto,
    /// Feature-gated native proof-verification contract boundary DTO foundations.
    ProofVerificationContractBoundaryDto,
    /// Feature-gated native proof-verification adapter DTO foundations.
    ProofVerificationAdapterDto,
    /// Feature-gated native replay and challenge-consumption contract DTO foundations.
    ReplayChallengeConsumptionContractDto,
    /// Feature-gated native replay and challenge-consumption adapter DTO foundations.
    ReplayChallengeConsumptionAdapterDto,
    /// Feature-gated native device-bound capability contract DTO foundations.
    DeviceBoundCapabilityContractDto,
    /// Feature-gated native device-bound capability adapter DTO foundations.
    DeviceBoundCapabilityAdapterDto,
    /// Feature-gated native private-beta username registry contract DTO foundations.
    UsernamePrivateBetaRegistryContractDto,
    /// Feature-gated native private-beta username registry adapter DTO foundations.
    UsernamePrivateBetaRegistryAdapterDto,
    /// Feature-gated native private-beta username exact-lookup projection DTO foundations.
    UsernamePrivateBetaIndexProjectionDto,
    /// Feature-gated native local status inspection DTO foundations.
    LocalStatusInspectionDto,
    /// Feature-gated native gateway/Omnigate route contract DTO foundations.
    GatewayOmnigateRouteContractDto,
    /// Feature-gated native Omnigate fixed-route admission DTO foundations.
    OmnigateFixedRouteAdmissionDto,
    /// Feature-gated native gateway fixed-route admission DTO foundations.
    GatewayFixedRouteAdmissionDto,
    /// Feature-gated native gateway/Omnigate route mount acceptance DTO foundations.
    GatewayOmnigateRouteMountAcceptanceDto,
    /// Feature-gated native desktop client status command DTO foundations.
    ClientStatusCommandDto,
    /// Feature-gated native desktop client command inventory DTO foundations.
    ClientCommandInventoryDto,
    /// Feature-gated native desktop redacted command DTO adapter foundations.
    ClientRedactedCommandDtoAdapter,
    /// Feature-gated native desktop command surface acceptance DTO foundations.
    ClientCommandSurfaceAcceptanceDto,
}

/// Compile-time posture for the feature-enabled Native Passport module.
///
/// This intentionally records that the module is present but no runtime
/// authority has been enabled yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportFeaturePosture {
    /// Canonical crate/package owner.
    pub owner: &'static str,
    /// Cargo feature name.
    pub feature_name: &'static str,
    /// Active phase label.
    pub phase_label: &'static str,
    /// Surfaces exposed by this phase.
    pub enabled_surfaces: &'static [NativePassportSurface],
    /// Whether this feature gate changes runtime authority.
    pub runtime_authority_changed: bool,
    /// Whether this feature gate adds native secret custody.
    pub native_secret_implementation_added: bool,
    /// Whether this feature gate adds route handlers.
    pub routes_added: bool,
    /// Whether this feature gate adds signing or verification runtime.
    pub signing_or_verification_runtime_added: bool,
    /// Whether this feature gate adds vault encryption/decryption runtime.
    pub vault_runtime_added: bool,
    /// Whether this feature gate issues capabilities.
    pub capability_issuance_added: bool,
    /// Behaviors still forbidden at this gate.
    pub forbidden_behaviors: &'static [&'static str],
}

/// Surfaces that Phase 1A exposes when the feature is enabled.
pub const PHASE1A_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
];

/// Behaviors that remain forbidden while the module gate is introduced.
pub const PHASE1A_FORBIDDEN_BEHAVIORS: &[&str] = &[
    "mnemonic generation",
    "root key derivation runtime",
    "device key generation runtime",
    "root signature generation",
    "device signature generation",
    "challenge route implementation",
    "request verification route",
    "capability issuance",
    "vault encryption",
    "vault decryption",
    "PIN capture",
    "platform keystore access",
    "wallet mutation",
    "ledger mutation",
    "new Passport crate",
];

/// Phase label for the Native Passport DTO posture integration slice.
pub const NATIVE_PASSPORT_PHASE1C_LABEL: &str = "NATIVE_PASSPORT_PHASE1C_NATIVE_MODULE_DTO_POSTURE";

/// Surfaces that are now available behind the Native Passport feature gate.
pub const PHASE1C_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
];

/// Surfaces available after the redacted status surface is added in Phase 1E.
pub const PHASE1E_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
];

/// Surfaces available after ron-naming username/handle reuse is added in Phase 2B.
pub const PHASE2B_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
];

/// Surfaces available after root/device authorization DTO foundations are added in Phase 3A.
pub const PHASE3A_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
];

/// Surfaces available after pure root/device authorization review is added in Phase 3B.
pub const PHASE3B_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
];

/// Surfaces available after native recovery-root DTO foundations are added in Phase 4A.
pub const PHASE4A_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
];

/// Surfaces available after native device-key DTO foundations are added in Phase 4B.
pub const PHASE4B_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
];

/// Surfaces available after native platform sealer contract DTOs are added in Phase 5A.
pub const PHASE5A_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
];

/// Surfaces available after native secure-surface compartment contracts are added in Phase 5B.
pub const PHASE5B_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
];

/// Surfaces available after native two-compartment vault header DTOs are added in Phase 6A.
pub const PHASE6A_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
];

/// Surfaces available after native PIN unlock contract DTOs are added in Phase 6B.
pub const PHASE6B_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
];

/// Surfaces available after native restore contract DTOs are added in Phase 7A.
pub const PHASE7A_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
];

/// Surfaces available after native new-device enrollment contract DTOs are added in Phase 7B.
pub const PHASE7B_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
];

/// Surfaces available after native delegated enrollment contract DTOs are added in Phase 7C.
pub const PHASE7C_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
];

/// Surfaces available after native purpose-bound challenge contract DTOs are added in Phase 8A.
pub const PHASE8A_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
];

/// Surfaces available after native proof contract DTOs are added in Phase 8B.
pub const PHASE8B_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
];

/// Surfaces available after native request-proof contract DTOs are added in Phase 8C.
pub const PHASE8C_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
];

/// Surfaces available after native proof-signing contract boundary DTOs are added in Phase 9A.
pub const PHASE9A_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
];

/// Surfaces available after native proof-signing adapter DTOs are added in Phase 9B.
pub const PHASE9B_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
];

/// Surfaces available after native proof-verification contract boundary DTOs are added in Phase 10A.
pub const PHASE10A_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
];

/// Surfaces available after native proof-verification adapter DTOs are added in Phase 10B.
pub const PHASE10B_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
];

/// Surfaces available after replay and challenge-consumption contract DTOs are added in Phase 11A.
pub const PHASE11A_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
];

/// Surfaces available after replay and challenge-consumption adapter DTOs are added in Phase 11B.
pub const PHASE11B_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
];

/// Surfaces available after device-bound capability contract DTOs are added in Phase 11C.
pub const PHASE11C_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
];

/// Surfaces available after device-bound capability adapter DTOs are added in Phase 11D.
pub const PHASE11D_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
    NativePassportSurface::DeviceBoundCapabilityAdapterDto,
];

/// Surfaces available after private-beta username registry contract DTOs are added in Phase 12A.
pub const PHASE12A_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
    NativePassportSurface::DeviceBoundCapabilityAdapterDto,
    NativePassportSurface::UsernamePrivateBetaRegistryContractDto,
];

/// Surfaces available after private-beta username registry adapter DTOs are added in Phase 12B.
pub const PHASE12B_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
    NativePassportSurface::DeviceBoundCapabilityAdapterDto,
    NativePassportSurface::UsernamePrivateBetaRegistryContractDto,
    NativePassportSurface::UsernamePrivateBetaRegistryAdapterDto,
];

/// Surfaces available after private-beta username index projection DTOs are added in Phase 12C.
pub const PHASE12C_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
    NativePassportSurface::DeviceBoundCapabilityAdapterDto,
    NativePassportSurface::UsernamePrivateBetaRegistryContractDto,
    NativePassportSurface::UsernamePrivateBetaRegistryAdapterDto,
    NativePassportSurface::UsernamePrivateBetaIndexProjectionDto,
];

/// Surfaces available after local status inspection DTOs are added in Phase 13.
pub const PHASE13_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
    NativePassportSurface::DeviceBoundCapabilityAdapterDto,
    NativePassportSurface::UsernamePrivateBetaRegistryContractDto,
    NativePassportSurface::UsernamePrivateBetaRegistryAdapterDto,
    NativePassportSurface::UsernamePrivateBetaIndexProjectionDto,
    NativePassportSurface::LocalStatusInspectionDto,
];

/// Surfaces available after gateway/Omnigate route contract DTOs are added in Phase 14A.
pub const PHASE14A_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
    NativePassportSurface::DeviceBoundCapabilityAdapterDto,
    NativePassportSurface::UsernamePrivateBetaRegistryContractDto,
    NativePassportSurface::UsernamePrivateBetaRegistryAdapterDto,
    NativePassportSurface::UsernamePrivateBetaIndexProjectionDto,
    NativePassportSurface::LocalStatusInspectionDto,
    NativePassportSurface::GatewayOmnigateRouteContractDto,
];

/// Surfaces available after Omnigate fixed-route admission DTOs are added in Phase 14B.
pub const PHASE14B_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
    NativePassportSurface::DeviceBoundCapabilityAdapterDto,
    NativePassportSurface::UsernamePrivateBetaRegistryContractDto,
    NativePassportSurface::UsernamePrivateBetaRegistryAdapterDto,
    NativePassportSurface::UsernamePrivateBetaIndexProjectionDto,
    NativePassportSurface::LocalStatusInspectionDto,
    NativePassportSurface::GatewayOmnigateRouteContractDto,
    NativePassportSurface::OmnigateFixedRouteAdmissionDto,
];

/// Surfaces available after gateway fixed-route admission DTOs are added in Phase 14C.
pub const PHASE14C_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
    NativePassportSurface::DeviceBoundCapabilityAdapterDto,
    NativePassportSurface::UsernamePrivateBetaRegistryContractDto,
    NativePassportSurface::UsernamePrivateBetaRegistryAdapterDto,
    NativePassportSurface::UsernamePrivateBetaIndexProjectionDto,
    NativePassportSurface::LocalStatusInspectionDto,
    NativePassportSurface::GatewayOmnigateRouteContractDto,
    NativePassportSurface::OmnigateFixedRouteAdmissionDto,
    NativePassportSurface::GatewayFixedRouteAdmissionDto,
];

/// Surfaces available after gateway/Omnigate route mount acceptance DTOs are added in Phase 14D.
pub const PHASE14D_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
    NativePassportSurface::DeviceBoundCapabilityAdapterDto,
    NativePassportSurface::UsernamePrivateBetaRegistryContractDto,
    NativePassportSurface::UsernamePrivateBetaRegistryAdapterDto,
    NativePassportSurface::UsernamePrivateBetaIndexProjectionDto,
    NativePassportSurface::LocalStatusInspectionDto,
    NativePassportSurface::GatewayOmnigateRouteContractDto,
    NativePassportSurface::OmnigateFixedRouteAdmissionDto,
    NativePassportSurface::GatewayFixedRouteAdmissionDto,
    NativePassportSurface::GatewayOmnigateRouteMountAcceptanceDto,
];

/// Surfaces available after desktop client status command DTOs are added in Phase 15A.
pub const PHASE15A_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
    NativePassportSurface::DeviceBoundCapabilityAdapterDto,
    NativePassportSurface::UsernamePrivateBetaRegistryContractDto,
    NativePassportSurface::UsernamePrivateBetaRegistryAdapterDto,
    NativePassportSurface::UsernamePrivateBetaIndexProjectionDto,
    NativePassportSurface::LocalStatusInspectionDto,
    NativePassportSurface::GatewayOmnigateRouteContractDto,
    NativePassportSurface::OmnigateFixedRouteAdmissionDto,
    NativePassportSurface::GatewayFixedRouteAdmissionDto,
    NativePassportSurface::GatewayOmnigateRouteMountAcceptanceDto,
    NativePassportSurface::ClientStatusCommandDto,
];

/// Surfaces available after desktop client command inventory DTOs are added in Phase 15B.
pub const PHASE15B_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
    NativePassportSurface::DeviceBoundCapabilityAdapterDto,
    NativePassportSurface::UsernamePrivateBetaRegistryContractDto,
    NativePassportSurface::UsernamePrivateBetaRegistryAdapterDto,
    NativePassportSurface::UsernamePrivateBetaIndexProjectionDto,
    NativePassportSurface::LocalStatusInspectionDto,
    NativePassportSurface::GatewayOmnigateRouteContractDto,
    NativePassportSurface::OmnigateFixedRouteAdmissionDto,
    NativePassportSurface::GatewayFixedRouteAdmissionDto,
    NativePassportSurface::GatewayOmnigateRouteMountAcceptanceDto,
    NativePassportSurface::ClientStatusCommandDto,
    NativePassportSurface::ClientCommandInventoryDto,
];

/// Surfaces available after desktop redacted command DTO adapters are added in Phase 15C.
pub const PHASE15C_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
    NativePassportSurface::DeviceBoundCapabilityAdapterDto,
    NativePassportSurface::UsernamePrivateBetaRegistryContractDto,
    NativePassportSurface::UsernamePrivateBetaRegistryAdapterDto,
    NativePassportSurface::UsernamePrivateBetaIndexProjectionDto,
    NativePassportSurface::LocalStatusInspectionDto,
    NativePassportSurface::GatewayOmnigateRouteContractDto,
    NativePassportSurface::OmnigateFixedRouteAdmissionDto,
    NativePassportSurface::GatewayFixedRouteAdmissionDto,
    NativePassportSurface::GatewayOmnigateRouteMountAcceptanceDto,
    NativePassportSurface::ClientStatusCommandDto,
    NativePassportSurface::ClientCommandInventoryDto,
    NativePassportSurface::ClientRedactedCommandDtoAdapter,
];

/// Surfaces available after desktop command surface acceptance is added in Phase 15D.
pub const PHASE15D_ENABLED_SURFACES: &[NativePassportSurface] = &[
    NativePassportSurface::FeaturePosture,
    NativePassportSurface::Phase0Contracts,
    NativePassportSurface::NativeDtoTypes,
    NativePassportSurface::NativeStatusRedaction,
    NativePassportSurface::NativeUsernameHandles,
    NativePassportSurface::RootDeviceAuthorizationDtos,
    NativePassportSurface::PureAuthorizationReview,
    NativePassportSurface::NativeRecoveryRootDto,
    NativePassportSurface::NativeDeviceKeyDto,
    NativePassportSurface::PlatformSealerContractDto,
    NativePassportSurface::SecureSurfaceCompartmentContract,
    NativePassportSurface::TwoCompartmentVaultHeaderDto,
    NativePassportSurface::PinUnlockContractDto,
    NativePassportSurface::RestoreContractDto,
    NativePassportSurface::NewDeviceEnrollmentContractDto,
    NativePassportSurface::DelegatedEnrollmentContractDto,
    NativePassportSurface::PurposeBoundChallengeContractDto,
    NativePassportSurface::ProofContractDto,
    NativePassportSurface::RequestProofContractDto,
    NativePassportSurface::ProofSigningContractBoundaryDto,
    NativePassportSurface::ProofSigningAdapterDto,
    NativePassportSurface::ProofVerificationContractBoundaryDto,
    NativePassportSurface::ProofVerificationAdapterDto,
    NativePassportSurface::ReplayChallengeConsumptionContractDto,
    NativePassportSurface::ReplayChallengeConsumptionAdapterDto,
    NativePassportSurface::DeviceBoundCapabilityContractDto,
    NativePassportSurface::DeviceBoundCapabilityAdapterDto,
    NativePassportSurface::UsernamePrivateBetaRegistryContractDto,
    NativePassportSurface::UsernamePrivateBetaRegistryAdapterDto,
    NativePassportSurface::UsernamePrivateBetaIndexProjectionDto,
    NativePassportSurface::LocalStatusInspectionDto,
    NativePassportSurface::GatewayOmnigateRouteContractDto,
    NativePassportSurface::OmnigateFixedRouteAdmissionDto,
    NativePassportSurface::GatewayFixedRouteAdmissionDto,
    NativePassportSurface::GatewayOmnigateRouteMountAcceptanceDto,
    NativePassportSurface::ClientStatusCommandDto,
    NativePassportSurface::ClientCommandInventoryDto,
    NativePassportSurface::ClientRedactedCommandDtoAdapter,
    NativePassportSurface::ClientCommandSurfaceAcceptanceDto,
];

/// DTO/value type names intentionally exposed by the feature-gated native module.
pub const PHASE1C_DTO_TYPE_NAMES: &[&str] = &[
    "PassportIdV1",
    "DeviceIdV1",
    "ChallengeIdV1",
    "B3DigestHex",
    "Ed25519PublicKeyHex",
    "NativePassportScope",
    "DeviceClass",
    "DeviceAuthorizationDraftV1",
];

/// DTO posture exposed by the feature-gated Native Passport module.
///
/// This is still static inspection only. It confirms DTO availability and
/// rejects any implication that DTO exposure enables routes, signing, vault
/// runtime, capability issuance, or secret custody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportDtoPosture {
    /// Canonical crate/package owner.
    pub owner: &'static str,
    /// Cargo feature name.
    pub feature_name: &'static str,
    /// Active phase label.
    pub phase_label: &'static str,
    /// Feature-gated surfaces available after Phase 1C.
    pub enabled_surfaces: &'static [NativePassportSurface],
    /// DTO type names available through `svc_passport::native`.
    pub dto_type_names: &'static [&'static str],
    /// DTO field names that remain forbidden.
    pub forbidden_dto_fields: &'static [&'static str],
    /// Count of read-only scopes accepted by DTO parsing.
    pub read_only_scope_count: usize,
    /// Count of unsafe scope strings rejected by DTO parsing.
    pub unsafe_scope_count: usize,
    /// Whether this posture changes runtime authority.
    pub runtime_authority_changed: bool,
    /// Whether this posture adds native secret custody.
    pub native_secret_implementation_added: bool,
    /// Whether this posture adds routes.
    pub routes_added: bool,
    /// Whether this posture adds signing or verification runtime.
    pub signing_or_verification_runtime_added: bool,
    /// Whether this posture adds vault encryption/decryption runtime.
    pub vault_runtime_added: bool,
    /// Whether this posture issues capabilities.
    pub capability_issuance_added: bool,
}

/// Return the Phase 1C native DTO posture.
///
/// This helper performs no I/O and does not create, sign, verify, encrypt,
/// decrypt, persist, issue, revoke, or mutate anything.
pub fn native_passport_dto_posture() -> NativePassportDtoPosture {
    NativePassportDtoPosture {
        owner: CANONICAL_PASSPORT_PACKAGE_OWNER,
        feature_name: NATIVE_PASSPORT_FEATURE_NAME,
        phase_label: NATIVE_PASSPORT_PHASE1C_LABEL,
        enabled_surfaces: PHASE1C_ENABLED_SURFACES,
        dto_type_names: PHASE1C_DTO_TYPE_NAMES,
        forbidden_dto_fields: PHASE1B_FORBIDDEN_DTO_FIELDS,
        read_only_scope_count: NATIVE_PASSPORT_READ_ONLY_SCOPE_CEILING.len(),
        unsafe_scope_count: NATIVE_PASSPORT_UNSAFE_SCOPE_STRINGS.len(),
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        routes_added: false,
        signing_or_verification_runtime_added: false,
        vault_runtime_added: false,
        capability_issuance_added: false,
    }
}

/// Return the Phase 1A feature posture.
///
/// This is intentionally a pure, static inspection helper. It does not derive
/// keys, read secrets, touch the filesystem, register routes, issue tokens, or
/// mutate state.
pub fn native_passport_feature_posture() -> NativePassportFeaturePosture {
    NativePassportFeaturePosture {
        owner: CANONICAL_PASSPORT_PACKAGE_OWNER,
        feature_name: NATIVE_PASSPORT_FEATURE_NAME,
        phase_label: NATIVE_PASSPORT_PHASE1A_LABEL,
        enabled_surfaces: PHASE1A_ENABLED_SURFACES,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        routes_added: false,
        signing_or_verification_runtime_added: false,
        vault_runtime_added: false,
        capability_issuance_added: false,
        forbidden_behaviors: PHASE1A_FORBIDDEN_BEHAVIORS,
    }
}

pub use desktop_platform_storage_inspection::{
    native_desktop_platform_storage_inspection_posture,
    review_native_desktop_platform_storage_inspection,
    NativeDesktopPlatformStorageInspectionDecisionV1,
    NativeDesktopPlatformStorageInspectionDraftV1, NativeDesktopPlatformStorageInspectionPosture,
    NativeDesktopPlatformStorageInspectionReviewError, NATIVE_PASSPORT_PHASE15H_LABEL,
    PHASE15H_DESKTOP_PLATFORM_STORAGE_INSPECTION_DOMAIN,
    PHASE15H_DESKTOP_PLATFORM_STORAGE_INSPECTION_VERSION,
    PHASE15H_FORBIDDEN_PLATFORM_STORAGE_FLAGS, PHASE15H_FRONTEND_OWNER,
    PHASE15H_PLATFORM_ADAPTER_OWNER, PHASE15H_PLATFORM_NEUTRAL_OWNER,
    PHASE15H_REQUIRED_ATOMIC_WRITE_STEPS, PHASE15H_REQUIRED_DESKTOP_PLATFORM_FAMILIES,
    PHASE15H_TAURI_CARGO_PATH, PHASE15H_TAURI_PASSPORT_COMMAND_PATH, PHASE15H_TAURI_STATE_PATH,
};

pub use vault_crypto::{
    decode_native_pin_wrapped_vault_keys, encode_native_pin_wrapped_vault_keys,
    encode_native_vault_authenticated_header, native_vault_crypto_posture, native_vault_kek_info,
    unlock_native_operational_vmk, verify_native_recovery_root_pin, wrap_native_compartment_vmk,
    NativePinWrappedCompartmentVmkV1, NativePinWrappedVaultKeysV1, NativeVaultCryptoError,
    NativeVaultCryptoPosture, NativeVaultKdfProfileV1, NATIVE_PASSPORT_PHASE15Q_LABEL,
    PHASE15Q_AEAD_DISPLAY_LABEL, PHASE15Q_ARGON2_PARALLELISM, PHASE15Q_ARGON2_TIME_COST,
    PHASE15Q_AUTHENTICATED_HEADER_DOMAIN, PHASE15Q_AUTHENTICATED_HEADER_ENCODING,
    PHASE15Q_AUTHENTICATED_HEADER_INPUT_FORMAT, PHASE15Q_DERIVED_KEY_BYTES,
    PHASE15Q_KDF_DISPLAY_LABEL, PHASE15Q_KDF_VERSION_LABEL, PHASE15Q_KEK_DOMAIN,
    PHASE15Q_MAX_ENCODED_VAULT_KEY_BYTES, PHASE15Q_OPERATIONAL_DOMAIN_TAG,
    PHASE15Q_OPERATIONAL_KDF_PROFILE, PHASE15Q_OPERATIONAL_MEMORY_MIB,
    PHASE15Q_OPERATIONAL_PURPOSE, PHASE15Q_PLATFORM_FACTOR_BYTES,
    PHASE15Q_REQUIRED_COMPARTMENT_COUNT, PHASE15Q_ROOT_DOMAIN_TAG, PHASE15Q_ROOT_KDF_PROFILE,
    PHASE15Q_ROOT_MEMORY_MIB, PHASE15Q_ROOT_PURPOSE, PHASE15Q_VAULT_KEY_CODEC_MAGIC,
    PHASE15Q_VAULT_KEY_CODEC_VERSION, PHASE15Q_VAULT_MASTER_KEY_BYTES, PHASE15Q_WRAPPED_VMK_BYTES,
};

pub use operational_device_payload::{
    decode_native_encrypted_operational_device_payload_v1,
    decrypt_native_operational_device_payload_v1,
    encode_native_encrypted_operational_device_payload_v1,
    encrypt_native_operational_device_payload_v1, NativeEncryptedOperationalDevicePayloadV1,
    NativeOperationalDevicePayloadError, NativeOperationalDevicePayloadV1,
    PHYSICAL_M1_DEVICE_PAYLOAD_AAD_DOMAIN, PHYSICAL_M1_DEVICE_PAYLOAD_ENVELOPE_MAGIC,
    PHYSICAL_M1_DEVICE_PAYLOAD_KEY_DOMAIN, PHYSICAL_M1_DEVICE_PAYLOAD_MAX_CIPHERTEXT_BYTES,
    PHYSICAL_M1_DEVICE_PAYLOAD_MAX_ENCODED_BYTES, PHYSICAL_M1_DEVICE_PAYLOAD_PLAINTEXT_MAGIC,
    PHYSICAL_M1_DEVICE_PAYLOAD_VERSION, PHYSICAL_M1_OPERATIONAL_DEVICE_PAYLOAD_LABEL,
    PHYSICAL_M1_OPERATIONAL_VAULT_DOMAIN,
};

pub use platform_bound_vault::{
    decode_native_platform_bound_vault, encode_native_platform_bound_vault,
    native_platform_bound_vault_posture, NativePlatformBoundVaultError,
    NativePlatformBoundVaultPosture, NativePlatformBoundVaultV1,
    NATIVE_PASSPORT_PHASE15R_CORE_LABEL, PHASE15R_PLATFORM_BOUND_VAULT_MAGIC,
    PHASE15R_PLATFORM_BOUND_VAULT_VERSION, PHASE15R_REQUIRED_SEALED_FACTOR_COUNT,
};

pub use platform_bound_vault_v2::{
    decode_native_platform_bound_vault_v2, decode_native_platform_bound_vault_versioned,
    encode_native_platform_bound_vault_v2, prepare_native_platform_bound_vault_v1_to_v2_migration,
    NativePlatformBoundVaultV2, NativePlatformBoundVaultV2Error, NativePlatformBoundVaultVersioned,
    PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_LABEL, PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_MAGIC,
    PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_VERSION,
};

pub use platform_storage::{
    load_native_encrypted_vault, native_platform_storage_trait_posture,
    recover_native_interrupted_vault_write, remove_native_encrypted_vault, seal_native_secret,
    unseal_native_secret, write_native_encrypted_vault_atomic, NativeEncryptedVaultV1,
    NativePlatformSealer, NativePlatformStorageError, NativePlatformStorageOperation,
    NativePlatformStorageTraitPosture, NativeSealedMaterialV1, NativeSecretBytes,
    NativeVaultRecoveryOutcome, NativeVaultRemovalOutcome, NativeVaultStore,
    NATIVE_PASSPORT_PHASE15I_LABEL, PHASE15I_ATOMIC_VAULT_WRITE_STEPS,
    PHASE15I_FORBIDDEN_PLATFORM_STORAGE_AUTHORITY_FLAGS, PHASE15I_MAX_ENCRYPTED_VAULT_BYTES,
    PHASE15I_MAX_SEALED_MATERIAL_BYTES, PHASE15I_MAX_SECRET_MATERIAL_BYTES,
    PHASE15I_PLATFORM_STORAGE_DOMAIN, PHASE15I_PLATFORM_STORAGE_VERSION,
};
