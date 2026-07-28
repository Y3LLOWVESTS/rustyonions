//! RO:WHAT — Feature-gated Native Passport redacted status DTO.
//! RO:WHY — P3 Identity & Keys; Concerns: SEC/GOV. Future status/routes need safe posture facts without leaking identifiers or secrets.
//! RO:INTERACTS — native module posture, DTO posture, service KMS injection posture.
//! RO:INVARIANTS — status is inspection-only and redacts IDs, keys, signatures, vault fields, capabilities, wallet/ledger authority, and secret material.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — exposes safe posture only; never emits raw secret-bearing or authority-bearing values.
//! RO:TEST — tests/native_passport_phase1e_native_status_redaction.rs.

use crate::kms::{service_kms_injection_posture, ServiceKmsConstructionMode};

use super::{
    native_passport_dto_posture, native_passport_feature_posture, NativePassportSurface,
    NATIVE_PASSPORT_FEATURE_NAME, PHASE1E_ENABLED_SURFACES,
};

/// Phase label for redacted Native Passport status.
pub const NATIVE_PASSPORT_PHASE1E_LABEL: &str = "NATIVE_PASSPORT_PHASE1E_NATIVE_STATUS_REDACTION";

/// Schema label for the redacted status snapshot.
pub const NATIVE_PASSPORT_REDACTED_STATUS_SCHEMA_V1: &str =
    "svc-passport.native-passport.redacted-status.v1";

/// Placeholder used when callers ask for a known redacted field.
pub const NATIVE_PASSPORT_REDACTED_VALUE: &str = "REDACTED";

/// Fields that the Native Passport status surface must never expose raw.
pub const PHASE1E_REDACTED_FIELD_NAMES: &[&str] = &[
    "passport_id",
    "device_id",
    "challenge_id",
    "root_public_key_hex",
    "device_public_key_hex",
    "authorization_transcript_b3_hex",
    "challenge_proof_transcript_b3_hex",
    "request_transcript_b3_hex",
    "capability_hash_hex",
    "mnemonic_words",
    "bip39_seed",
    "root_private_key",
    "root_signing_seed",
    "device_private_key",
    "pin",
    "vault_master_key",
    "derived_vault_key",
    "platform_device_secret",
    "wallet_spend_authority",
    "ledger_mutation_authority",
    "raw_long_lived_capability",
    "root_signature_hex",
    "device_signature_hex",
    "device_request_signature_hex",
];

/// Status readiness value for the redacted Native Passport surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePassportRedactedStatusReadiness {
    /// Feature gate and Phase 0 contracts are present, but runtime authority is off.
    ContractsReadyNoRuntimeAuthority,
}

/// Safe, redacted status snapshot for Native Passport.
///
/// This intentionally avoids raw IDs, public keys, signatures, capabilities,
/// vault material, wallet/ledger authority, and secret-bearing values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportRedactedStatusV1 {
    /// Status schema.
    pub schema: &'static str,
    /// Active phase label.
    pub phase_label: &'static str,
    /// Canonical package owner.
    pub owner: &'static str,
    /// Cargo feature name.
    pub feature_name: &'static str,
    /// Readiness posture.
    pub readiness: NativePassportRedactedStatusReadiness,
    /// Feature-gated surfaces exposed so far.
    pub enabled_surfaces: &'static [NativePassportSurface],
    /// Service KMS construction mode label.
    pub service_kms_mode: &'static str,
    /// Number of DTO types exposed through native posture.
    pub dto_type_count: usize,
    /// Number of read-only scopes accepted by DTO parsing.
    pub read_only_scope_count: usize,
    /// Number of unsafe scopes rejected by DTO parsing.
    pub unsafe_scope_count: usize,
    /// Field names known to be redacted by this status surface.
    pub redacted_field_names: &'static [&'static str],
    /// Whether status exposure changed runtime authority.
    pub runtime_authority_changed: bool,
    /// Whether status exposure added native secret custody.
    pub native_secret_implementation_added: bool,
    /// Whether status exposure added routes.
    pub routes_added: bool,
    /// Whether status exposure added signing or verification runtime.
    pub signing_or_verification_runtime_added: bool,
    /// Whether status exposure added vault encryption/decryption runtime.
    pub vault_runtime_added: bool,
    /// Whether status exposure added capability issuance.
    pub capability_issuance_added: bool,
    /// Whether status exposure added wallet or ledger mutation.
    pub wallet_or_ledger_mutation_added: bool,
}

fn service_kms_mode_label(mode: ServiceKmsConstructionMode) -> &'static str {
    match mode {
        ServiceKmsConstructionMode::DevelopmentInProcess => "development_in_process",
        ServiceKmsConstructionMode::ExternalInjected => "external_injected",
    }
}

/// Return a safe redacted Native Passport status snapshot.
///
/// This performs no I/O and does not derive keys, read secrets, register routes,
/// sign, verify, encrypt, decrypt, persist, issue, revoke, or mutate state.
pub fn native_passport_redacted_status() -> NativePassportRedactedStatusV1 {
    let feature = native_passport_feature_posture();
    let dto = native_passport_dto_posture();
    let kms = service_kms_injection_posture();

    NativePassportRedactedStatusV1 {
        schema: NATIVE_PASSPORT_REDACTED_STATUS_SCHEMA_V1,
        phase_label: NATIVE_PASSPORT_PHASE1E_LABEL,
        owner: feature.owner,
        feature_name: NATIVE_PASSPORT_FEATURE_NAME,
        readiness: NativePassportRedactedStatusReadiness::ContractsReadyNoRuntimeAuthority,
        enabled_surfaces: PHASE1E_ENABLED_SURFACES,
        service_kms_mode: service_kms_mode_label(kms.default_mode),
        dto_type_count: dto.dto_type_names.len(),
        read_only_scope_count: dto.read_only_scope_count,
        unsafe_scope_count: dto.unsafe_scope_count,
        redacted_field_names: PHASE1E_REDACTED_FIELD_NAMES,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        routes_added: false,
        signing_or_verification_runtime_added: false,
        vault_runtime_added: false,
        capability_issuance_added: false,
        wallet_or_ledger_mutation_added: false,
    }
}

/// Return the redacted placeholder for a known redacted field.
pub fn redacted_value_for(field_name: &str) -> Option<&'static str> {
    PHASE1E_REDACTED_FIELD_NAMES
        .contains(&field_name)
        .then_some(NATIVE_PASSPORT_REDACTED_VALUE)
}
