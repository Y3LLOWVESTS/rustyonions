//! RO:WHAT — KMS boundary (trait) + local dev implementation.
//! RO:WHY  — Swap to ron-kms without touching service code.

pub mod client;
pub mod keyslot;
pub mod rotation;

/// Phase label for server KMS dev isolation.
pub const NATIVE_PASSPORT_PHASE1D_LABEL: &str = "NATIVE_PASSPORT_PHASE1D_SERVER_KMS_DEV_ISOLATION";

/// KMS construction mode reported by the service boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceKmsConstructionMode {
    /// Development/default in-process KMS helper is available through `dev-kms`.
    DevelopmentInProcess,
    /// A caller injects an external service KMS implementation.
    ExternalInjected,
}

/// Static posture for the svc-passport service KMS seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceKmsInjectionPosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Default construction mode for this build.
    pub default_mode: ServiceKmsConstructionMode,
    /// Whether router construction supports explicit KMS injection.
    pub explicit_injection_supported: bool,
    /// Whether direct development KMS construction is isolated behind a helper.
    pub dev_default_constructor_isolated: bool,
    /// Whether this phase changes runtime authority.
    pub runtime_authority_changed: bool,
    /// Whether this phase adds native secret custody.
    pub native_secret_implementation_added: bool,
    /// Whether this phase adds Passport root/device signing runtime.
    pub native_passport_signing_runtime_added: bool,
    /// Whether this phase adds vault encryption/decryption runtime.
    pub vault_runtime_added: bool,
    /// Whether this phase issues device-bound capabilities.
    pub capability_issuance_added: bool,
}

/// Return the Phase 1D service KMS injection posture.
///
/// This is inspection-only. It does not create keys, sign messages, verify
/// proofs, issue capabilities, or mutate wallet/ledger state.
pub fn service_kms_injection_posture() -> ServiceKmsInjectionPosture {
    ServiceKmsInjectionPosture {
        phase_label: NATIVE_PASSPORT_PHASE1D_LABEL,
        default_mode: if cfg!(feature = "dev-kms") {
            ServiceKmsConstructionMode::DevelopmentInProcess
        } else {
            ServiceKmsConstructionMode::ExternalInjected
        },
        explicit_injection_supported: true,
        dev_default_constructor_isolated: true,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        native_passport_signing_runtime_added: false,
        vault_runtime_added: false,
        capability_issuance_added: false,
    }
}
