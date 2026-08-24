//! RO:WHAT — Builds the trusted Physical M1 RootAdminDesktop device-authorization signing payload.
//!
//! RO:WHY — A real Passport root must sign only policy-owned class/scopes and trusted native runtime facts, never WebView-selected authority.
//!
//! RO:INTERACTS — `ron-policy` private-beta device policy, canonical `ron-proto` authorization DTOs, public Passport identity, and public device identity.
//!
//! RO:INVARIANTS — class is fixed to RootAdminDesktop; scopes come only from policy; Passport/device public bindings are re-derived before payload construction.
//!
//! RO:CONFIG — network, environment, root epoch, nonce, issue time, and optional expiry are explicit trusted inputs with no hidden defaults.
//!
//! RO:SECURITY — public-data construction only; no RNG, clock access, root/device secret access, signing, persistence, routes, capabilities, username mutation, wallet, or ledger authority.
//!
//! RO:TEST — `tests/physical_m1_native_device_authorization_context.rs`.

#![forbid(unsafe_code)]

use ron_policy::private_beta_device_authorization_scope_ceiling_v1;
use ron_proto::{
    DeviceAuthorizationNonceV1, DeviceAuthorizationSigningPayloadV1, DeviceClassV1,
    DeviceIdV1 as ProtoDeviceIdV1, Ed25519PublicKeyHex as ProtoEd25519PublicKeyHex,
    NativePassportContextLabelV1, PassportIdV1 as ProtoPassportIdV1,
    DEVICE_AUTHORIZATION_V1_VERSION,
};
use thiserror::Error;

use super::{
    derive_native_device_id_v1, derive_native_passport_id_v1, NativeDevicePublicIdentityV1,
    RootPassportDescriptorV1,
};

/// Physical M1 label for trusted desktop DeviceAuthorization payload construction.
pub const PHYSICAL_M1_DEVICE_AUTHORIZATION_CONTEXT_LABEL: &str =
    "PHYSICAL_M1_NATIVE_ROOT_ADMIN_DESKTOP_DEVICE_AUTHORIZATION_CONTEXT_V1";

/// Explicit trusted runtime facts needed to build one desktop root
/// authorization.
///
/// This structure deliberately contains no device class and no scope list.
/// Those authority-bearing values are selected by trusted product/policy code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRootAdminDesktopAuthorizationContextV1 {
    network_id: NativePassportContextLabelV1,
    environment: NativePassportContextLabelV1,
    root_key_epoch: u64,
    authorization_nonce: DeviceAuthorizationNonceV1,
    issued_at_ms: u64,
    expires_at_ms: Option<u64>,
}

impl NativeRootAdminDesktopAuthorizationContextV1 {
    /// Construct explicit trusted runtime context.
    ///
    /// Root epoch zero is intentionally permitted because the private-beta V1
    /// model may use epoch zero. This constructor does not invent an epoch,
    /// expiration, clock value, or nonce.
    ///
    /// # Errors
    ///
    /// Returns [`NativeDeviceAuthorizationContextError::InvalidIssuedAt`] when
    /// issue time is zero, or
    /// [`NativeDeviceAuthorizationContextError::InvalidExpiry`] when an
    /// explicit expiry does not follow issue time.
    pub fn new(
        network_id: NativePassportContextLabelV1,
        environment: NativePassportContextLabelV1,
        root_key_epoch: u64,
        authorization_nonce: DeviceAuthorizationNonceV1,
        issued_at_ms: u64,
        expires_at_ms: Option<u64>,
    ) -> Result<Self, NativeDeviceAuthorizationContextError> {
        if issued_at_ms == 0 {
            return Err(NativeDeviceAuthorizationContextError::InvalidIssuedAt);
        }

        if expires_at_ms.is_some_and(|expires_at_ms| expires_at_ms <= issued_at_ms) {
            return Err(NativeDeviceAuthorizationContextError::InvalidExpiry);
        }

        Ok(Self {
            network_id,
            environment,
            root_key_epoch,
            authorization_nonce,
            issued_at_ms,
            expires_at_ms,
        })
    }
}

/// Trusted Physical M1 DeviceAuthorization payload-construction failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum NativeDeviceAuthorizationContextError {
    /// Trusted runtime supplied a zero issue time.
    #[error("device authorization issue time must be nonzero")]
    InvalidIssuedAt,

    /// Trusted runtime supplied an invalid explicit expiry.
    #[error("device authorization expiry must be later than issue time")]
    InvalidExpiry,

    /// The public Passport ID could not be re-derived from its root public key.
    #[error("failed to derive Passport ID from trusted public root key")]
    PassportIdDerivationFailed,

    /// The public Passport descriptor contains mismatched ID/root-key material.
    #[error("public Passport ID does not match its root public key")]
    PassportBindingMismatch,

    /// The public Device ID could not be re-derived from its device public key.
    #[error("failed to derive Device ID from trusted public device key")]
    DeviceIdDerivationFailed,

    /// The public device identity contains mismatched ID/public-key material.
    #[error("public Device ID does not match its device public key")]
    DeviceBindingMismatch,

    /// Native Passport ID could not bridge to the canonical protocol type.
    #[error("failed to bridge Passport ID to canonical protocol type")]
    PassportIdBridgeFailed,

    /// Native Device ID could not bridge to the canonical protocol type.
    #[error("failed to bridge Device ID to canonical protocol type")]
    DeviceIdBridgeFailed,

    /// Native device public key could not bridge to the canonical protocol type.
    #[error("failed to bridge device public key to canonical protocol type")]
    DevicePublicKeyBridgeFailed,

    /// The policy owner refused or failed to construct the class scope ceiling.
    #[error("private-beta device authorization scope policy rejected the device class")]
    ScopePolicyRejected,

    /// The resulting canonical signing payload failed protocol validation.
    #[error("constructed DeviceAuthorization signing payload failed canonical validation")]
    InvalidPayload,
}

/// Build one policy-owned Physical M1 RootAdminDesktop signing payload.
///
/// The caller supplies only trusted runtime facts and already-established
/// public identities. It cannot select the device class or scope ceiling.
///
/// # Errors
///
/// Fails closed on public Passport/device binding mismatch, canonical-type
/// bridge failure, policy rejection, or invalid canonical payload fields.
pub fn build_root_admin_desktop_device_authorization_payload_v1(
    root: &RootPassportDescriptorV1,
    device: &NativeDevicePublicIdentityV1,
    context: NativeRootAdminDesktopAuthorizationContextV1,
) -> Result<DeviceAuthorizationSigningPayloadV1, NativeDeviceAuthorizationContextError> {
    validate_public_passport_binding(root)?;
    validate_public_device_binding(device)?;

    let passport_id = ProtoPassportIdV1::parse(root.passport_id.as_str())
        .map_err(|_| NativeDeviceAuthorizationContextError::PassportIdBridgeFailed)?;

    let device_id = ProtoDeviceIdV1::parse(device.device_id.as_str())
        .map_err(|_| NativeDeviceAuthorizationContextError::DeviceIdBridgeFailed)?;

    let device_public_key = ProtoEd25519PublicKeyHex::parse(device.device_public_key.as_str())
        .map_err(|_| NativeDeviceAuthorizationContextError::DevicePublicKeyBridgeFailed)?;

    let device_class = DeviceClassV1::RootAdminDesktop;

    let authorized_scope_ceiling = private_beta_device_authorization_scope_ceiling_v1(device_class)
        .map_err(|_| NativeDeviceAuthorizationContextError::ScopePolicyRejected)?;

    let payload = DeviceAuthorizationSigningPayloadV1 {
        version: DEVICE_AUTHORIZATION_V1_VERSION,
        network_id: context.network_id,
        environment: context.environment,
        passport_id,
        root_key_epoch: context.root_key_epoch,
        device_id,
        device_public_key,
        device_class,
        authorized_scope_ceiling,
        authorization_nonce: context.authorization_nonce,
        issued_at_ms: context.issued_at_ms,
        expires_at_ms: context.expires_at_ms,
    };

    payload
        .validate()
        .map_err(|_| NativeDeviceAuthorizationContextError::InvalidPayload)?;

    Ok(payload)
}

fn validate_public_passport_binding(
    root: &RootPassportDescriptorV1,
) -> Result<(), NativeDeviceAuthorizationContextError> {
    let expected = derive_native_passport_id_v1(&root.root_public_key)
        .map_err(|_| NativeDeviceAuthorizationContextError::PassportIdDerivationFailed)?;

    if expected != root.passport_id {
        return Err(NativeDeviceAuthorizationContextError::PassportBindingMismatch);
    }

    Ok(())
}

fn validate_public_device_binding(
    device: &NativeDevicePublicIdentityV1,
) -> Result<(), NativeDeviceAuthorizationContextError> {
    let expected = derive_native_device_id_v1(&device.device_public_key)
        .map_err(|_| NativeDeviceAuthorizationContextError::DeviceIdDerivationFailed)?;

    if expected != device.device_id {
        return Err(NativeDeviceAuthorizationContextError::DeviceBindingMismatch);
    }

    Ok(())
}
