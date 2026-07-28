//! RO:WHAT — svc-passport Native Passport reuse of ron-naming username/handle DTOs.
//! RO:WHY — P3 Identity & Keys + P7 SDK/Interop. Prevents duplicate `@username` rules inside svc-passport.
//! RO:INTERACTS — ron-naming canonical UsernameV1/HandleV1, native status/posture surfaces, future profile-route migration.
//! RO:INVARIANTS — handles are optional public labels only; not legal names, Passport IDs, wallet authority, ledger authority, or secret material.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — exposes parse/reuse helpers only; no route registration, signing, vault runtime, or capability issuance.
//! RO:TEST — tests/native_passport_phase2b_ron_naming_username_reuse.rs.

/// Phase label for svc-passport reuse of ron-naming username/handle DTOs.
pub const NATIVE_PASSPORT_PHASE2B_LABEL: &str = "NATIVE_PASSPORT_PHASE2B_RON_NAMING_USERNAME_REUSE";

/// Optional username/handle status for Native Passport.
pub const OPTIONAL_USERNAME_HANDLE_STATUS: &str = "optional_public_handle_not_identity_authority";

/// Re-export canonical username type from ron-naming.
pub type UsernameV1 = ron_naming::UsernameV1;

/// Re-export canonical handle type from ron-naming.
pub type HandleV1 = ron_naming::HandleV1;

/// Re-export username parse error from ron-naming.
pub type UsernameParseError = ron_naming::PassportUsernameParseError;

/// Fields and meanings that must not be assigned to optional public handles.
pub const PHASE2B_FORBIDDEN_HANDLE_MEANINGS: &[&str] = &[
    "legal name",
    "real human name",
    "Passport ID",
    "Device ID",
    "Challenge ID",
    "root public key",
    "device public key",
    "root private key",
    "device private key",
    "wallet spend authority",
    "ledger mutation authority",
    "capability issuance authority",
    "secret recovery material",
];

/// Posture for svc-passport Native Passport username/handle reuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportUsernameReusePosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Canonical owner of username syntax.
    pub canonical_username_owner: &'static str,
    /// Optional public handle posture.
    pub optional_handle_status: &'static str,
    /// Whether svc-passport reuses ron-naming for username parsing.
    pub ron_naming_reuse_enabled: bool,
    /// Minimum username length.
    pub username_min_len: usize,
    /// Maximum username length.
    pub username_max_len: usize,
    /// Reserved username label count.
    pub reserved_label_count: usize,
    /// Meanings forbidden for optional handles.
    pub forbidden_handle_meanings: &'static [&'static str],
    /// Whether this phase changes runtime authority.
    pub runtime_authority_changed: bool,
    /// Whether this phase adds native secret custody.
    pub native_secret_implementation_added: bool,
    /// Whether this phase adds routes.
    pub routes_added: bool,
    /// Whether this phase adds signing or verification runtime.
    pub signing_or_verification_runtime_added: bool,
    /// Whether this phase adds vault encryption/decryption runtime.
    pub vault_runtime_added: bool,
    /// Whether this phase issues capabilities.
    pub capability_issuance_added: bool,
    /// Whether this phase adds wallet or ledger mutation.
    pub wallet_or_ledger_mutation_added: bool,
}

/// Parse an optional public handle using ron-naming canonical rules.
pub fn parse_optional_username_handle(
    value: Option<&str>,
) -> Result<Option<HandleV1>, UsernameParseError> {
    value.map(HandleV1::parse).transpose()
}

/// Return the Phase 2B username reuse posture.
pub fn native_passport_username_reuse_posture() -> NativePassportUsernameReusePosture {
    NativePassportUsernameReusePosture {
        phase_label: NATIVE_PASSPORT_PHASE2B_LABEL,
        canonical_username_owner: "ron-naming",
        optional_handle_status: OPTIONAL_USERNAME_HANDLE_STATUS,
        ron_naming_reuse_enabled: true,
        username_min_len: ron_naming::USERNAME_MIN_LEN,
        username_max_len: ron_naming::USERNAME_MAX_LEN,
        reserved_label_count: ron_naming::RESERVED_USERNAME_LABELS.len(),
        forbidden_handle_meanings: PHASE2B_FORBIDDEN_HANDLE_MEANINGS,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        routes_added: false,
        signing_or_verification_runtime_added: false,
        vault_runtime_added: false,
        capability_issuance_added: false,
        wallet_or_ledger_mutation_added: false,
    }
}
