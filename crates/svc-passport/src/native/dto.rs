//! RO:WHAT — Native Passport feature-gated DTO/value types.
//! RO:WHY — P3 Identity & Keys; Concerns: SEC/GOV. Provides reusable validated identifiers and bounded read-only DTOs.
//! RO:INTERACTS — native module gate, native_plan ID format guards, future proof/capability/vault modules.
//! RO:INVARIANTS — no secret fields; no signing; no route registration; no capability issuance; no wallet/ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the feature-gated native module.
//! RO:SECURITY — rejects unsafe scopes and malformed identifiers before future runtime behavior consumes them.
//! RO:TEST — tests/native_passport_phase1b_native_dto_types.rs.

use std::{collections::BTreeSet, fmt, str::FromStr};

/// Phase label for the Native Passport DTO/value-type slice.
pub const NATIVE_PASSPORT_PHASE1B_LABEL: &str = "NATIVE_PASSPORT_PHASE1B_NATIVE_DTO_TYPES";

/// Canonical challenge ID prefix for challenge fixtures and future challenge DTOs.
pub const CHALLENGE_ID_V1_B3_PREFIX: &str = ron_proto::CHALLENGE_ID_V1_B3_PREFIX;

/// Required lowercase hex digest length for BLAKE3-256 identifiers.
pub const B3_DIGEST_HEX_LEN: usize = ron_proto::B3_DIGEST_HEX_LEN;

/// Required lowercase hex length for Ed25519 public keys.
pub const ED25519_PUBLIC_KEY_HEX_LEN: usize = ron_proto::ED25519_PUBLIC_KEY_HEX_LEN;

/// Fields that must not appear in Native Passport DTOs at this layer.
pub const PHASE1B_FORBIDDEN_DTO_FIELDS: &[&str] = &[
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
];

/// Native Passport DTO validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativePassportDtoError {
    /// Required value was empty.
    Empty {
        /// Field name.
        field: &'static str,
    },
    /// Passport ID did not match `passport:v1:main:ed25519:b3:<64 lowercase hex>`.
    InvalidPassportId,
    /// Device ID did not match `device:v1:ed25519:b3:<64 lowercase hex>`.
    InvalidDeviceId,
    /// Challenge ID did not match `challenge:v1:b3:<64 lowercase hex>`.
    InvalidChallengeId,
    /// Hex value length or alphabet was invalid.
    InvalidLowerHex {
        /// Field name.
        field: &'static str,
        /// Required lowercase hex length.
        expected_len: usize,
        /// Actual length.
        actual_len: usize,
    },
    /// Scope is not a known Native Passport read-only scope.
    UnsupportedScope(String),
    /// Scope is known unsafe for the current DTO layer.
    UnsafeScope(String),
    /// Scope set was empty.
    EmptyScopeSet,
    /// Scope set contained a duplicate value.
    DuplicateScope(&'static str),
    /// Device class was not recognized.
    UnsupportedDeviceClass(String),
}

impl fmt::Display for NativePassportDtoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { field } => write!(f, "{field} must not be empty"),
            Self::InvalidPassportId => write!(f, "invalid Native Passport V1 passport id"),
            Self::InvalidDeviceId => write!(f, "invalid Native Passport V1 device id"),
            Self::InvalidChallengeId => write!(f, "invalid Native Passport V1 challenge id"),
            Self::InvalidLowerHex {
                field,
                expected_len,
                actual_len,
            } => write!(
                f,
                "{field} must be {expected_len} lowercase hex characters, got {actual_len}"
            ),
            Self::UnsupportedScope(scope) => {
                write!(f, "unsupported Native Passport scope: {scope}")
            }
            Self::UnsafeScope(scope) => {
                write!(
                    f,
                    "unsafe Native Passport scope rejected at DTO layer: {scope}"
                )
            }
            Self::EmptyScopeSet => write!(f, "scope set must not be empty"),
            Self::DuplicateScope(scope) => write!(f, "duplicate scope: {scope}"),
            Self::UnsupportedDeviceClass(class) => {
                write!(f, "unsupported Native Passport device class: {class}")
            }
        }
    }
}

impl std::error::Error for NativePassportDtoError {}

fn ensure_non_empty(field: &'static str, value: &str) -> Result<(), NativePassportDtoError> {
    if value.is_empty() {
        Err(NativePassportDtoError::Empty { field })
    } else {
        Ok(())
    }
}

fn is_lower_hex_len(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_lower_hex(
    field: &'static str,
    value: &str,
    expected_len: usize,
) -> Result<(), NativePassportDtoError> {
    ensure_non_empty(field, value)?;
    if is_lower_hex_len(value, expected_len) {
        Ok(())
    } else {
        Err(NativePassportDtoError::InvalidLowerHex {
            field,
            expected_len,
            actual_len: value.len(),
        })
    }
}

/// Validated Native Passport V1 Passport ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PassportIdV1(String);

impl PassportIdV1 {
    /// Parse and validate a Passport ID.
    pub fn parse(value: impl Into<String>) -> Result<Self, NativePassportDtoError> {
        let value = value.into();
        if ron_proto::PassportIdV1::parse(value.as_str()).is_ok() {
            Ok(Self(value))
        } else {
            Err(NativePassportDtoError::InvalidPassportId)
        }
    }

    /// Borrow the canonical string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PassportIdV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PassportIdV1 {
    type Err = NativePassportDtoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Validated Native Passport V1 Device ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceIdV1(String);

impl DeviceIdV1 {
    /// Parse and validate a Device ID.
    pub fn parse(value: impl Into<String>) -> Result<Self, NativePassportDtoError> {
        let value = value.into();
        if ron_proto::DeviceIdV1::parse(value.as_str()).is_ok() {
            Ok(Self(value))
        } else {
            Err(NativePassportDtoError::InvalidDeviceId)
        }
    }

    /// Borrow the canonical string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceIdV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DeviceIdV1 {
    type Err = NativePassportDtoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Validated Native Passport V1 Challenge ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChallengeIdV1(String);

impl ChallengeIdV1 {
    /// Parse and validate a Challenge ID.
    pub fn parse(value: impl Into<String>) -> Result<Self, NativePassportDtoError> {
        let value = value.into();
        if ron_proto::ChallengeIdV1::parse(value.as_str()).is_ok() {
            Ok(Self(value))
        } else {
            Err(NativePassportDtoError::InvalidChallengeId)
        }
    }

    /// Borrow the canonical string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ChallengeIdV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ChallengeIdV1 {
    type Err = NativePassportDtoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Validated BLAKE3-256 lowercase hex digest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct B3DigestHex(String);

impl B3DigestHex {
    /// Parse and validate a BLAKE3 digest hex string.
    pub fn parse(
        field: &'static str,
        value: impl Into<String>,
    ) -> Result<Self, NativePassportDtoError> {
        let value = value.into();
        validate_lower_hex(field, &value, B3_DIGEST_HEX_LEN)?;
        Ok(Self(value))
    }

    /// Borrow the digest string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for B3DigestHex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Validated Ed25519 public key lowercase hex.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ed25519PublicKeyHex(String);

impl Ed25519PublicKeyHex {
    /// Parse and validate an Ed25519 public key hex string.
    pub fn parse(value: impl Into<String>) -> Result<Self, NativePassportDtoError> {
        let value = value.into();
        validate_lower_hex("ed25519_public_key_hex", &value, ED25519_PUBLIC_KEY_HEX_LEN)?;
        Ok(Self(value))
    }

    /// Borrow the public key hex string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Ed25519PublicKeyHex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Read-only Native Passport scopes allowed at the DTO layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NativePassportScope {
    /// Read Passport identity state.
    IdentityRead,
    /// Read catalog state.
    CatalogRead,
    /// Read content state.
    ContentRead,
    /// Read entitlement state.
    EntitlementRead,
    /// Read receipts.
    ReceiptsRead,
    /// Read confirmed ROC state.
    ConfirmedRocRead,
    /// Revoke only the caller's own capability.
    CapabilityRevokeSelf,
}

impl NativePassportScope {
    /// Canonical scope string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IdentityRead => "identity.read",
            Self::CatalogRead => "catalog.read",
            Self::ContentRead => "content.read",
            Self::EntitlementRead => "entitlement.read",
            Self::ReceiptsRead => "receipts.read",
            Self::ConfirmedRocRead => "confirmed_roc.read",
            Self::CapabilityRevokeSelf => "capability.revoke_self",
        }
    }

    /// Parse a read-only scope string.
    pub fn parse_read_only(value: &str) -> Result<Self, NativePassportDtoError> {
        match value {
            "identity.read" => Ok(Self::IdentityRead),
            "catalog.read" => Ok(Self::CatalogRead),
            "content.read" => Ok(Self::ContentRead),
            "entitlement.read" => Ok(Self::EntitlementRead),
            "receipts.read" => Ok(Self::ReceiptsRead),
            "confirmed_roc.read" => Ok(Self::ConfirmedRocRead),
            "capability.revoke_self" => Ok(Self::CapabilityRevokeSelf),
            "wallet.spend"
            | "wallet.transfer"
            | "ledger.write"
            | "reward.issue"
            | "node.control"
            | "operator.admin"
            | "content.publish"
            | "capability.delegate_unbounded" => {
                Err(NativePassportDtoError::UnsafeScope(value.to_owned()))
            }
            other => Err(NativePassportDtoError::UnsupportedScope(other.to_owned())),
        }
    }
}

impl fmt::Display for NativePassportScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

/// Safe read-only scope ceiling for Phase 1 DTOs.
pub const NATIVE_PASSPORT_READ_ONLY_SCOPE_CEILING: &[NativePassportScope] = &[
    NativePassportScope::IdentityRead,
    NativePassportScope::CatalogRead,
    NativePassportScope::ContentRead,
    NativePassportScope::EntitlementRead,
    NativePassportScope::ReceiptsRead,
    NativePassportScope::ConfirmedRocRead,
    NativePassportScope::CapabilityRevokeSelf,
];

/// Unsafe scope strings rejected by DTO parsing.
pub const NATIVE_PASSPORT_UNSAFE_SCOPE_STRINGS: &[&str] = &[
    "wallet.spend",
    "wallet.transfer",
    "ledger.write",
    "reward.issue",
    "node.control",
    "operator.admin",
    "content.publish",
    "capability.delegate_unbounded",
];

/// Native Passport device class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceClass {
    /// TV read-only device posture.
    TvReadOnly,
    /// Desktop read-only device posture.
    DesktopReadOnly,
    /// Mobile read-only device posture.
    MobileReadOnly,
}

impl DeviceClass {
    /// Canonical device class string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TvReadOnly => "tv_read_only",
            Self::DesktopReadOnly => "desktop_read_only",
            Self::MobileReadOnly => "mobile_read_only",
        }
    }

    /// Parse a supported device class.
    pub fn parse(value: &str) -> Result<Self, NativePassportDtoError> {
        match value {
            "tv_read_only" => Ok(Self::TvReadOnly),
            "desktop_read_only" => Ok(Self::DesktopReadOnly),
            "mobile_read_only" => Ok(Self::MobileReadOnly),
            other => Err(NativePassportDtoError::UnsupportedDeviceClass(
                other.to_owned(),
            )),
        }
    }
}

impl fmt::Display for DeviceClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

fn validate_scope_set(scopes: &[NativePassportScope]) -> Result<(), NativePassportDtoError> {
    if scopes.is_empty() {
        return Err(NativePassportDtoError::EmptyScopeSet);
    }

    let mut seen = BTreeSet::new();
    for scope in scopes {
        if !seen.insert(*scope) {
            return Err(NativePassportDtoError::DuplicateScope(scope.as_str()));
        }
    }

    Ok(())
}

/// Bounded DeviceAuthorizationV1 DTO draft.
///
/// This DTO is intentionally a data contract only. It does not sign, verify,
/// persist, issue, revoke, or encrypt anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAuthorizationDraftV1 {
    passport_id: PassportIdV1,
    device_id: DeviceIdV1,
    root_public_key_hex: Ed25519PublicKeyHex,
    root_key_epoch: u64,
    device_public_key_hex: Ed25519PublicKeyHex,
    device_class: DeviceClass,
    authorized_scope_ceiling: Vec<NativePassportScope>,
    authorization_nonce_hex: B3DigestHex,
}

impl DeviceAuthorizationDraftV1 {
    /// Construct a validated DeviceAuthorizationV1 DTO draft.
    pub fn new(
        passport_id: PassportIdV1,
        device_id: DeviceIdV1,
        root_public_key_hex: Ed25519PublicKeyHex,
        root_key_epoch: u64,
        device_public_key_hex: Ed25519PublicKeyHex,
        device_class: DeviceClass,
        authorized_scope_ceiling: Vec<NativePassportScope>,
        authorization_nonce_hex: B3DigestHex,
    ) -> Result<Self, NativePassportDtoError> {
        validate_scope_set(&authorized_scope_ceiling)?;

        Ok(Self {
            passport_id,
            device_id,
            root_public_key_hex,
            root_key_epoch,
            device_public_key_hex,
            device_class,
            authorized_scope_ceiling,
            authorization_nonce_hex,
        })
    }

    /// Passport ID.
    pub fn passport_id(&self) -> &PassportIdV1 {
        &self.passport_id
    }

    /// Device ID.
    pub fn device_id(&self) -> &DeviceIdV1 {
        &self.device_id
    }

    /// Root public key hex.
    pub fn root_public_key_hex(&self) -> &Ed25519PublicKeyHex {
        &self.root_public_key_hex
    }

    /// Root key epoch.
    pub fn root_key_epoch(&self) -> u64 {
        self.root_key_epoch
    }

    /// Device public key hex.
    pub fn device_public_key_hex(&self) -> &Ed25519PublicKeyHex {
        &self.device_public_key_hex
    }

    /// Device class.
    pub fn device_class(&self) -> DeviceClass {
        self.device_class
    }

    /// Authorized read-only scope ceiling.
    pub fn authorized_scope_ceiling(&self) -> &[NativePassportScope] {
        &self.authorized_scope_ceiling
    }

    /// Authorization nonce fixture digest/hex value.
    pub fn authorization_nonce_hex(&self) -> &B3DigestHex {
        &self.authorization_nonce_hex
    }

    /// Scope ceiling as canonical CSV, matching Phase 0 vector ordering.
    pub fn scope_ceiling_csv(&self) -> String {
        self.authorized_scope_ceiling
            .iter()
            .map(|scope| scope.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }
}
