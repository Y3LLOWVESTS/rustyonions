//! RO:WHAT — Focused tests for trusted Physical M1 RootAdminDesktop DeviceAuthorization payload construction.
//!
//! RO:WHY — Proves class/scopes cannot come from client fixtures while runtime context remains explicit and public identity bindings fail closed.
//!
//! RO:INTERACTS — `svc_passport::native`, canonical `ron_proto` authorization DTOs, and the private-beta `ron-policy` scope ceiling.
//!
//! RO:INVARIANTS — exact RootAdminDesktop policy ceiling; no hidden expiry; root/device binding mismatch rejected; no signing or runtime authority.
//!
//! RO:SECURITY — public deterministic fixtures only; no RecoveryRoot, PIN, vault, filesystem, capability, username, wallet, ledger, or network mutation.
//!
//! RO:TEST — focused test target of this file.

#![cfg(feature = "native-passport")]

use ron_proto::{DeviceAuthorizationNonceV1, DeviceClassV1, NativePassportContextLabelV1};
use svc_passport::native::{
    build_root_admin_desktop_device_authorization_payload_v1, derive_native_device_id_v1,
    derive_native_passport_id_v1, Ed25519PublicKeyHex, NativeDeviceAuthorizationContextError,
    NativeDevicePublicIdentityV1, NativeRootAdminDesktopAuthorizationContextV1,
    RootPassportDescriptorV1, PHYSICAL_M1_DEVICE_AUTHORIZATION_CONTEXT_LABEL,
};

const ROOT_PUBLIC_KEY: &str = "3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909";

const DEVICE_PUBLIC_KEY: &str = "2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";

const ISSUED_AT_MS: u64 = 1_720_000_000_000;
const EXPIRES_AT_MS: u64 = 1_720_086_400_000;

fn root_identity() -> RootPassportDescriptorV1 {
    let root_public_key = Ed25519PublicKeyHex::parse(ROOT_PUBLIC_KEY).expect("root public key");

    let passport_id = derive_native_passport_id_v1(&root_public_key).expect("derived Passport ID");

    RootPassportDescriptorV1 {
        passport_id,
        root_public_key,
        optional_handle: None,
    }
}

fn device_identity() -> NativeDevicePublicIdentityV1 {
    let device_public_key =
        Ed25519PublicKeyHex::parse(DEVICE_PUBLIC_KEY).expect("device public key");

    let device_id = derive_native_device_id_v1(&device_public_key).expect("derived Device ID");

    NativeDevicePublicIdentityV1 {
        device_id,
        device_public_key,
    }
}

fn context(
    issued_at_ms: u64,
    expires_at_ms: Option<u64>,
) -> Result<NativeRootAdminDesktopAuthorizationContextV1, NativeDeviceAuthorizationContextError> {
    NativeRootAdminDesktopAuthorizationContextV1::new(
        NativePassportContextLabelV1::parse("rustyonions-devnet").expect("network fixture"),
        NativePassportContextLabelV1::parse("local-dev").expect("environment fixture"),
        7,
        DeviceAuthorizationNonceV1::from_bytes([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ]),
        issued_at_ms,
        expires_at_ms,
    )
}

fn scope_names(payload: &ron_proto::DeviceAuthorizationSigningPayloadV1) -> Vec<&str> {
    payload
        .authorized_scope_ceiling
        .as_slice()
        .iter()
        .map(ron_proto::NativePassportScopeV1::as_str)
        .collect()
}

#[test]
fn physical_m1_root_admin_desktop_payload_uses_policy_owned_class_and_scope_ceiling() {
    assert_eq!(
        PHYSICAL_M1_DEVICE_AUTHORIZATION_CONTEXT_LABEL,
        "PHYSICAL_M1_NATIVE_ROOT_ADMIN_DESKTOP_DEVICE_AUTHORIZATION_CONTEXT_V1",
    );

    let root = root_identity();
    let device = device_identity();

    let payload = build_root_admin_desktop_device_authorization_payload_v1(
        &root,
        &device,
        context(ISSUED_AT_MS, Some(EXPIRES_AT_MS)).unwrap(),
    )
    .expect("trusted RootAdminDesktop payload");

    assert_eq!(payload.device_class, DeviceClassV1::RootAdminDesktop);

    assert_eq!(
        scope_names(&payload),
        vec![
            "capability.revoke_self",
            "catalog.read",
            "confirmed_roc.read",
            "content.read",
            "entitlement.read",
            "identity.device.authorize",
            "identity.device.list",
            "identity.device.revoke",
            "identity.profile.read",
            "identity.profile.update",
            "identity.read",
            "identity.username.claim",
            "identity.username.release",
            "identity.username.transfer",
            "receipts.read",
        ],
    );

    assert_eq!(payload.passport_id.as_str(), root.passport_id.as_str());
    assert_eq!(payload.device_id.as_str(), device.device_id.as_str());
    assert_eq!(
        payload.device_public_key.as_str(),
        device.device_public_key.as_str(),
    );

    assert_eq!(payload.network_id.as_str(), "rustyonions-devnet");
    assert_eq!(payload.environment.as_str(), "local-dev");
    assert_eq!(payload.root_key_epoch, 7);
    assert_eq!(
        payload.authorization_nonce.as_bytes(),
        &[
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ],
    );
    assert_eq!(payload.issued_at_ms, ISSUED_AT_MS);
    assert_eq!(payload.expires_at_ms, Some(EXPIRES_AT_MS));

    payload.validate().expect("canonical payload validation");
}

#[test]
fn physical_m1_context_preserves_explicit_no_expiry_without_inventing_ttl() {
    let root = root_identity();
    let device = device_identity();

    let payload = build_root_admin_desktop_device_authorization_payload_v1(
        &root,
        &device,
        context(ISSUED_AT_MS, None).unwrap(),
    )
    .expect("explicit no-expiry fixture");

    assert_eq!(payload.expires_at_ms, None);
    assert_eq!(payload.issued_at_ms, ISSUED_AT_MS);
}

#[test]
fn physical_m1_context_rejects_invalid_time_ordering_before_payload_construction() {
    assert_eq!(
        context(0, None).unwrap_err(),
        NativeDeviceAuthorizationContextError::InvalidIssuedAt,
    );

    assert_eq!(
        context(ISSUED_AT_MS, Some(ISSUED_AT_MS)).unwrap_err(),
        NativeDeviceAuthorizationContextError::InvalidExpiry,
    );

    assert_eq!(
        context(ISSUED_AT_MS, Some(ISSUED_AT_MS - 1)).unwrap_err(),
        NativeDeviceAuthorizationContextError::InvalidExpiry,
    );
}

#[test]
fn physical_m1_builder_rejects_mismatched_public_passport_binding() {
    let mut root = root_identity();

    let other_root_public_key = Ed25519PublicKeyHex::parse(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .expect("alternate public root");

    root.passport_id =
        derive_native_passport_id_v1(&other_root_public_key).expect("alternate Passport ID");

    let error = build_root_admin_desktop_device_authorization_payload_v1(
        &root,
        &device_identity(),
        context(ISSUED_AT_MS, None).unwrap(),
    )
    .expect_err("mismatched root binding must fail closed");

    assert_eq!(
        error,
        NativeDeviceAuthorizationContextError::PassportBindingMismatch,
    );
}

#[test]
fn physical_m1_builder_rejects_mismatched_public_device_binding() {
    let mut device = device_identity();

    let other_device_public_key = Ed25519PublicKeyHex::parse(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .expect("alternate public device key");

    device.device_id =
        derive_native_device_id_v1(&other_device_public_key).expect("alternate Device ID");

    let error = build_root_admin_desktop_device_authorization_payload_v1(
        &root_identity(),
        &device,
        context(ISSUED_AT_MS, None).unwrap(),
    )
    .expect_err("mismatched device binding must fail closed");

    assert_eq!(
        error,
        NativeDeviceAuthorizationContextError::DeviceBindingMismatch,
    );
}

#[test]
fn physical_m1_context_builder_source_has_no_runtime_or_signing_authority() {
    let source = include_str!("../src/native/device_authorization_context.rs");

    for required in [
        "DeviceClassV1::RootAdminDesktop",
        "private_beta_device_authorization_scope_ceiling_v1",
        "derive_native_passport_id_v1",
        "derive_native_device_id_v1",
        "DeviceAuthorizationSigningPayloadV1",
    ] {
        assert!(
            source.contains(required),
            "trusted builder source missing required boundary {required}",
        );
    }

    for forbidden in [
        "sign_native_device_authorization_v1",
        "sign_native_recovery_device_authorization_v1",
        "getrandom::",
        "SystemTime",
        "UNIX_EPOCH",
        "std::fs",
        "tokio::fs",
        "tauri::",
        "#[tauri::command]",
        "issue_capability(",
        "wallet_spend",
        "ledger_mutation",
    ] {
        assert!(
            !source.contains(forbidden),
            "trusted builder source unexpectedly contains runtime authority marker {forbidden}",
        );
    }

    assert!(
        !source.contains("authorized_scope_ceiling: DeviceAuthorizationScopeCeilingV1,"),
        "caller must not supply an authorization scope ceiling",
    );
}
