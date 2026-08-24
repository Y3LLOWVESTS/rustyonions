//! RO:WHAT — Physical M1 strict-wire tests for canonical DeviceAuthorizationV1.
//! RO:WHY — Freeze the real protocol DTO before any physical Passport-root signature is created.
//! RO:INTERACTS — ron-proto Native Passport IDs/classes, strict scope/context tokens, base64url nonce/signature fields, and ron-auth canonical transcript work.
//! RO:INVARIANTS — unknown fields rejected; root public key is not accepted from untrusted authorization input; nonce/signature sizes are exact; scope order is sorted/unique; optional expiry is explicit.
//! RO:METRICS — none.
//! RO:CONFIG — deterministic fixtures only.
//! RO:SECURITY — public fake fixtures only; no private keys, physical Passport, vault, Keychain, capability issuance, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — cargo test -p ron-proto --test physical_m1_device_authorization_v1_wire.

use ron_proto::{
    DeviceAuthorizationNonceV1, DeviceAuthorizationScopeCeilingV1,
    DeviceAuthorizationSigningPayloadV1, DeviceAuthorizationV1, DeviceClassV1, DeviceIdV1,
    Ed25519PublicKeyHex, Ed25519SignatureV1, NativePassportContextLabelV1, NativePassportScopeV1,
    PassportIdV1, DEVICE_AUTHORIZATION_V1_VERSION,
};
use serde_json::json;

const PASSPORT_ID: &str =
    "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";

const DEVICE_ID: &str =
    "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";

const DEVICE_PUBLIC_KEY: &str = "2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";

fn scope(value: &str) -> NativePassportScopeV1 {
    NativePassportScopeV1::parse(value).expect("valid scope fixture")
}

fn scopes() -> DeviceAuthorizationScopeCeilingV1 {
    DeviceAuthorizationScopeCeilingV1::new(vec![
        scope("capability.revoke_self"),
        scope("catalog.read"),
        scope("confirmed_roc.read"),
        scope("content.read"),
        scope("entitlement.read"),
        scope("identity.read"),
        scope("receipts.read"),
    ])
    .expect("canonical sorted scope fixture")
}

fn payload() -> DeviceAuthorizationSigningPayloadV1 {
    DeviceAuthorizationSigningPayloadV1 {
        version: DEVICE_AUTHORIZATION_V1_VERSION,
        network_id: NativePassportContextLabelV1::parse("rustyonions-devnet").unwrap(),
        environment: NativePassportContextLabelV1::parse("local-dev").unwrap(),
        passport_id: PassportIdV1::parse(PASSPORT_ID).unwrap(),
        root_key_epoch: 0,
        device_id: DeviceIdV1::parse(DEVICE_ID).unwrap(),
        device_public_key: Ed25519PublicKeyHex::parse(DEVICE_PUBLIC_KEY).unwrap(),
        device_class: DeviceClassV1::TvReadOnly,
        authorized_scope_ceiling: scopes(),
        authorization_nonce: DeviceAuthorizationNonceV1::from_bytes([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ]),
        issued_at_ms: 1_720_000_000_000,
        expires_at_ms: Some(1_720_086_400_000),
    }
}

fn authorization() -> DeviceAuthorizationV1 {
    DeviceAuthorizationV1::from_signing_payload(
        payload(),
        Ed25519SignatureV1::from_bytes([0x11; 64]),
    )
    .unwrap()
}

#[test]
fn physical_m1_wire_shape_matches_active_plan_and_excludes_root_key_trust_input() {
    let authorization = authorization();

    authorization.validate().unwrap();

    let value = serde_json::to_value(&authorization).unwrap();

    for required in [
        "version",
        "network_id",
        "environment",
        "passport_id",
        "root_key_epoch",
        "device_id",
        "device_public_key",
        "device_class",
        "authorized_scope_ceiling",
        "authorization_nonce",
        "issued_at_ms",
        "expires_at_ms",
        "root_signature",
    ] {
        assert!(
            value.get(required).is_some(),
            "missing active-plan field {required}",
        );
    }

    for forbidden in [
        "root_public_key",
        "root_public_key_hex",
        "root_private_key",
        "device_private_key",
        "pin",
        "username",
        "wallet_account",
        "ledger_account",
    ] {
        assert!(
            value.get(forbidden).is_none(),
            "forbidden authority/secret field present: {forbidden}",
        );
    }

    assert_eq!(
        value["authorization_nonce"],
        json!("AAECAwQFBgcICQoLDA0ODw"),
    );

    let signature = value["root_signature"].as_str().unwrap();

    assert_eq!(signature.len(), 86);
    assert!(!signature.contains('='));

    assert_eq!(
        value["authorized_scope_ceiling"],
        json!([
            "capability.revoke_self",
            "catalog.read",
            "confirmed_roc.read",
            "content.read",
            "entitlement.read",
            "identity.read",
            "receipts.read",
        ]),
    );

    let roundtrip: DeviceAuthorizationV1 = serde_json::from_value(value).unwrap();

    assert_eq!(roundtrip, authorization);
}

#[test]
fn physical_m1_wire_rejects_unknown_fields_and_legacy_root_key_field() {
    let mut value = serde_json::to_value(authorization()).unwrap();

    value.as_object_mut().unwrap().insert(
        "root_public_key_hex".to_owned(),
        json!("3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909"),
    );

    assert!(serde_json::from_value::<DeviceAuthorizationV1>(value,).is_err(),);
}

#[test]
fn physical_m1_wire_rejects_noncanonical_nonce_signature_and_scope_order() {
    let mut bad_nonce = serde_json::to_value(authorization()).unwrap();

    bad_nonce["authorization_nonce"] = json!("000102030405060708090a0b0c0d0e0f");

    assert!(
        serde_json::from_value::<DeviceAuthorizationV1>(bad_nonce,).is_err(),
        "legacy hex nonce is not the active binary-field JSON encoding",
    );

    let mut bad_signature = serde_json::to_value(authorization()).unwrap();

    bad_signature["root_signature"] = json!("ERER");

    assert!(serde_json::from_value::<DeviceAuthorizationV1>(bad_signature,).is_err(),);

    let mut unsorted = serde_json::to_value(authorization()).unwrap();

    unsorted["authorized_scope_ceiling"] = json!(["identity.read", "catalog.read",]);

    assert!(serde_json::from_value::<DeviceAuthorizationV1>(unsorted,).is_err(),);

    let mut duplicate = serde_json::to_value(authorization()).unwrap();

    duplicate["authorized_scope_ceiling"] = json!(["catalog.read", "catalog.read",]);

    assert!(serde_json::from_value::<DeviceAuthorizationV1>(duplicate,).is_err(),);
}

#[test]
fn physical_m1_wire_supports_optional_expiry_without_weakening_time_validation() {
    let mut no_expiry = payload();

    no_expiry.expires_at_ms = None;
    no_expiry.validate().unwrap();

    let value = serde_json::to_value(&no_expiry).unwrap();

    assert!(value.get("expires_at_ms").is_none(),);

    let mut invalid = payload();

    invalid.expires_at_ms = Some(invalid.issued_at_ms);

    assert!(invalid.validate().is_err());

    let mut zero_issue = payload();

    zero_issue.issued_at_ms = 0;

    assert!(zero_issue.validate().is_err());
}

#[test]
fn physical_m1_scope_type_is_syntax_only_and_policy_remains_external() {
    for accepted in [
        "identity.read",
        "identity.device.authorize",
        "identity.username.claim",
        "content.publish",
    ] {
        assert!(
            NativePassportScopeV1::parse(accepted,).is_ok(),
            "wire syntax must not silently become class policy: {accepted}",
        );
    }

    for rejected in ["", "IDENTITY.READ", " identity.read", "identity/read"] {
        assert!(NativePassportScopeV1::parse(rejected,).is_err(),);
    }
}
