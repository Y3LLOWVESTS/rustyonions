//! RO:WHAT — Purpose-specific Native Passport root signing for canonical DeviceAuthorizationV1 records.
//! RO:WHY — Physical M1 must authorize a device with the recovery-derived Passport root while keeping root-key custody inside root_identity and transcript ownership inside ron-auth.
//! RO:INTERACTS — Native root public-identity derivation, native Device-ID derivation, NativeSecretBytes, ron-proto DeviceAuthorizationV1 DTOs, and root_identity's private purpose-specific signing primitive.
//! RO:INVARIANTS — payload validates before signing; payload Passport ID must equal the recovery-derived Passport; Device ID must derive from the supplied device public key; only canonical ron-auth transcript bytes reach the root signer.
//! RO:METRICS — none.
//! RO:CONFIG — native-passport feature only.
//! RO:SECURITY — no raw SigningKey, root seed, HKDF, generic-message signer, filesystem, Keychain, HTTP, capability issuance, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — tests/physical_m1_native_device_authorization_signing.rs.

use ron_proto::{DeviceAuthorizationSigningPayloadV1, DeviceAuthorizationV1, Ed25519SignatureV1};

use super::{
    derive_native_device_id_v1, derive_native_root_public_identity_v1,
    root_identity::sign_native_root_device_authorization_payload_v1,
    Ed25519PublicKeyHex as NativeEd25519PublicKeyHex, NativeSecretBytes,
};

pub const PHYSICAL_M1_DEVICE_AUTHORIZATION_SIGNING_LABEL: &str =
    "PHYSICAL_M1_NATIVE_DEVICE_AUTHORIZATION_SIGNING_V1";

/// Narrow signing failures before any durable authorization registry exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NativeDeviceAuthorizationSigningError {
    InvalidPayload,
    RootIdentityDerivationFailed,
    PassportBindingMismatch,
    DevicePublicKeyBridgeFailed,
    DeviceIdDerivationFailed,
    DeviceIdBindingMismatch,
    RootSignatureFailed,
    SignedAuthorizationConstructionFailed,
}

/// Produce one root-signed DeviceAuthorizationV1.
///
/// This function does not persist or register the authorization and therefore
/// does not itself grant server authority. Registration/replay/idempotency and
/// policy remain later explicit authority boundaries.
pub fn sign_native_device_authorization_v1(
    bip39_seed: &NativeSecretBytes,
    payload: DeviceAuthorizationSigningPayloadV1,
) -> Result<DeviceAuthorizationV1, NativeDeviceAuthorizationSigningError> {
    payload
        .validate()
        .map_err(|_| NativeDeviceAuthorizationSigningError::InvalidPayload)?;

    let root_identity = derive_native_root_public_identity_v1(bip39_seed)
        .map_err(|_| NativeDeviceAuthorizationSigningError::RootIdentityDerivationFailed)?;

    if root_identity.passport_id.as_str() != payload.passport_id.as_str() {
        return Err(NativeDeviceAuthorizationSigningError::PassportBindingMismatch);
    }

    let native_device_public_key =
        NativeEd25519PublicKeyHex::parse(payload.device_public_key.as_str())
            .map_err(|_| NativeDeviceAuthorizationSigningError::DevicePublicKeyBridgeFailed)?;

    let derived_device_id = derive_native_device_id_v1(&native_device_public_key)
        .map_err(|_| NativeDeviceAuthorizationSigningError::DeviceIdDerivationFailed)?;

    if derived_device_id.as_str() != payload.device_id.as_str() {
        return Err(NativeDeviceAuthorizationSigningError::DeviceIdBindingMismatch);
    }

    let signature = sign_native_root_device_authorization_payload_v1(bip39_seed, &payload)
        .map_err(|_| NativeDeviceAuthorizationSigningError::RootSignatureFailed)?;

    DeviceAuthorizationV1::from_signing_payload(payload, Ed25519SignatureV1::from_bytes(signature))
        .map_err(|_| NativeDeviceAuthorizationSigningError::SignedAuthorizationConstructionFailed)
}
