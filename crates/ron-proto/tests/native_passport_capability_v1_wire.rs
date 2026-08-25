//! RO:WHAT — Wire/validation tests for Native Passport V1 device-bound capability and PassportRequestProofV1.
//! RO:WHY — CN-4 must freeze strict protocol structure before ron-auth signs/verifies transcripts or svc-passport gains capability runtime authority.
//! RO:INTERACTS — ron-proto capability/request-proof DTOs, canonical Passport/Device/Capability IDs, B3 request hashes, and Ed25519 signature DTO.
//! RO:INVARIANTS — valid DTOs round-trip; unknown fields reject; capability scopes remain sorted/unique; capability lifetime/policy are valid; request method/path/timestamp fail closed.
//! RO:SECURITY — deterministic public test data only; no real key, signing runtime, persistence, route, capability issuance, username, wallet, or ledger mutation.
//! RO:TEST — cargo test -p ron-proto --test native_passport_capability_v1_wire.

use ron_proto::{
    B3DigestHex, CapabilityIdV1, DeviceIdV1, Ed25519SignatureV1,
    NativePassportCapabilityValidationError, NativePassportContextLabelV1,
    NativePassportDeviceBoundCapabilityV1, NativePassportScopeV1, PassportIdV1,
    PassportRequestProofV1, NATIVE_PASSPORT_DEVICE_BOUND_CAPABILITY_V1_VERSION,
    PASSPORT_REQUEST_PROOF_V1_VERSION,
};

const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

fn scope(value: &str) -> NativePassportScopeV1 {
    NativePassportScopeV1::parse(value).expect("scope")
}

fn capability() -> NativePassportDeviceBoundCapabilityV1 {
    NativePassportDeviceBoundCapabilityV1 {
        version: NATIVE_PASSPORT_DEVICE_BOUND_CAPABILITY_V1_VERSION,
        capability_id: CapabilityIdV1::parse(format!("capability:v1:b3:{HEX_A}"))
            .expect("capability ID"),
        passport_id: PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_B}"))
            .expect("Passport ID"),
        device_id: DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}")).expect("Device ID"),
        audience: NativePassportContextLabelV1::parse("svc-passport").expect("audience"),
        environment: NativePassportContextLabelV1::parse("private-beta").expect("environment"),
        scopes: vec![
            scope("identity.profile.update"),
            scope("identity.read"),
            scope("identity.username.claim"),
        ],
        issued_at_ms: 1_800_000_000_000,
        expires_at_ms: 1_800_003_600_000,
        policy_version: 1,
        root_key_epoch: Some(0),
    }
}

fn request_proof() -> PassportRequestProofV1 {
    PassportRequestProofV1 {
        version: PASSPORT_REQUEST_PROOF_V1_VERSION,
        capability_id: CapabilityIdV1::parse(format!("capability:v1:b3:{HEX_A}"))
            .expect("capability ID"),
        request_method: "POST".to_owned(),
        canonical_path: "/identity/passport/username/claim".to_owned(),
        canonical_query_hash: B3DigestHex::parse("canonical_query_hash", HEX_D)
            .expect("query hash"),
        body_hash: B3DigestHex::parse("body_hash", HEX_E).expect("body hash"),
        timestamp_ms: 1_800_000_030_000,
        request_nonce: B3DigestHex::parse("request_nonce", HEX_B).expect("request nonce"),
        device_id: DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}")).expect("Device ID"),
        device_signature: Ed25519SignatureV1::from_bytes([0x55; 64]),
    }
}

#[test]
fn device_bound_capability_is_strict_and_round_trips() {
    let capability = capability();

    capability.validate().expect("valid capability");

    let encoded = serde_json::to_value(&capability).expect("serialize");
    let decoded: NativePassportDeviceBoundCapabilityV1 =
        serde_json::from_value(encoded).expect("deserialize");

    assert_eq!(decoded, capability);
    assert_eq!(
        capability.capability_id.as_str(),
        format!("capability:v1:b3:{HEX_A}")
    );
    assert_eq!(capability.root_key_epoch, Some(0));
}

#[test]
fn capability_scope_time_and_policy_shape_fail_closed() {
    let mut empty = capability();
    empty.scopes.clear();

    assert_eq!(
        empty.validate(),
        Err(NativePassportCapabilityValidationError::EmptyCapabilityScopes),
    );

    let mut duplicate = capability();
    duplicate.scopes = vec![scope("identity.read"), scope("identity.read")];

    assert_eq!(
        duplicate.validate(),
        Err(NativePassportCapabilityValidationError::DuplicateCapabilityScope),
    );

    let mut unsorted = capability();
    unsorted.scopes = vec![scope("identity.username.claim"), scope("identity.read")];

    assert_eq!(
        unsorted.validate(),
        Err(NativePassportCapabilityValidationError::NonCanonicalCapabilityScopeOrder),
    );

    let mut expired_at_issue = capability();
    expired_at_issue.expires_at_ms = expired_at_issue.issued_at_ms;

    assert_eq!(
        expired_at_issue.validate(),
        Err(NativePassportCapabilityValidationError::InvalidCapabilityExpiry),
    );

    let mut no_policy = capability();
    no_policy.policy_version = 0;

    assert_eq!(
        no_policy.validate(),
        Err(NativePassportCapabilityValidationError::InvalidCapabilityPolicyVersion),
    );
}

#[test]
fn request_proof_binds_canonical_request_shape() {
    let proof = request_proof();

    proof.validate().expect("valid request proof");

    let encoded = serde_json::to_value(&proof).expect("serialize");
    let decoded: PassportRequestProofV1 = serde_json::from_value(encoded).expect("deserialize");

    assert_eq!(decoded, proof);

    let mut lower_method = proof.clone();
    lower_method.request_method = "post".to_owned();

    assert_eq!(
        lower_method.validate(),
        Err(NativePassportCapabilityValidationError::InvalidRequestMethod),
    );

    let mut query_in_path = proof.clone();
    query_in_path.canonical_path = "/identity/passport/username/claim?x=1".to_owned();

    assert_eq!(
        query_in_path.validate(),
        Err(NativePassportCapabilityValidationError::InvalidCanonicalPath),
    );

    let mut fragment_in_path = proof.clone();
    fragment_in_path.canonical_path = "/identity/passport/username/claim#fragment".to_owned();

    assert_eq!(
        fragment_in_path.validate(),
        Err(NativePassportCapabilityValidationError::InvalidCanonicalPath),
    );

    let mut zero_timestamp = proof;
    zero_timestamp.timestamp_ms = 0;

    assert_eq!(
        zero_timestamp.validate(),
        Err(NativePassportCapabilityValidationError::InvalidRequestTimestamp),
    );
}

#[test]
fn wire_unknown_fields_and_noncanonical_capability_ids_reject() {
    assert!(CapabilityIdV1::parse(format!("capability:v1:b3:{HEX_A}")).is_ok());

    assert!(CapabilityIdV1::parse(format!("challenge:v1:b3:{HEX_A}")).is_err());

    let mut capability_json = serde_json::to_value(capability()).expect("capability JSON");

    capability_json
        .as_object_mut()
        .expect("capability object")
        .insert("caller_authority".to_owned(), serde_json::json!(true));

    assert!(
        serde_json::from_value::<NativePassportDeviceBoundCapabilityV1>(capability_json).is_err()
    );

    let mut proof_json = serde_json::to_value(request_proof()).expect("proof JSON");

    proof_json.as_object_mut().expect("proof object").insert(
        "device_private_key".to_owned(),
        serde_json::json!("forbidden"),
    );

    assert!(serde_json::from_value::<PassportRequestProofV1>(proof_json).is_err());
}
