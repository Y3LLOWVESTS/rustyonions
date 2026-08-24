//! RO:WHAT — Wire/domain tests for canonical Native Passport `PassportChallengeV1`.
//! RO:WHY — The one-time signed challenge must reject ambiguous, unbounded, reserved, or schema-drifted inputs before any cryptographic or replay authority.
//! RO:INTERACTS — ron-proto challenge IDs, context/scope tokens, B3 hashes, Ed25519 signatures, and PassportChallengeV1.
//! RO:INVARIANTS — V1 only; strict JSON; canonical scopes; exact purpose bindings; bounded lifetime; wallet purpose reserved.
//! RO:SECURITY — fixtures contain public/dummy material only.
//! RO:TEST — cargo test -p ron-proto --test native_passport_challenge_v1_wire.

use ron_proto::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, Ed25519SignatureV1, NativePassportContextLabelV1,
    NativePassportScopeV1, PassportChallengePurposeV1, PassportChallengeSigningPayloadV1,
    PassportChallengeV1, PassportChallengeValidationError, PassportIdV1, ServiceKeyIdV1,
    PASSPORT_CHALLENGE_V1_MAX_CLOCK_SKEW_MS, PASSPORT_CHALLENGE_V1_MAX_TTL_MS,
    PASSPORT_CHALLENGE_V1_SERVICE_SIGNATURE_ALGORITHM, PASSPORT_CHALLENGE_V1_VERSION,
};

const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

fn context(value: &str) -> NativePassportContextLabelV1 {
    NativePassportContextLabelV1::parse(value).expect("context")
}

fn scope(value: &str) -> NativePassportScopeV1 {
    NativePassportScopeV1::parse(value).expect("scope")
}

fn service_key_id(value: &str) -> ServiceKeyIdV1 {
    ServiceKeyIdV1::parse(value).expect("service key id")
}

fn challenge_id() -> ChallengeIdV1 {
    ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_A}")).expect("challenge id")
}

fn passport_id() -> PassportIdV1 {
    PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_B}")).expect("passport id")
}

fn device_id() -> DeviceIdV1 {
    DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}")).expect("device id")
}

fn digest(label: &'static str, value: &str) -> B3DigestHex {
    B3DigestHex::parse(label, value).expect("digest")
}

fn valid_payload() -> PassportChallengeSigningPayloadV1 {
    PassportChallengeSigningPayloadV1 {
        version: PASSPORT_CHALLENGE_V1_VERSION,
        challenge_id: challenge_id(),
        network_id: context("rustyonions-devnet"),
        environment: context("private-beta"),
        audience: context("svc-passport"),
        issuing_service_id: context("svc-passport"),
        service_key_id: service_key_id("ed25519/default/v1"),
        purpose: PassportChallengePurposeV1::IssueCapability,
        requested_scopes: vec![
            scope("catalog.read"),
            scope("content.read"),
            scope("identity.read"),
        ],
        passport_id: Some(passport_id()),
        device_id: Some(device_id()),
        operation_body_hash: Some(digest("operation_body_hash", HEX_D)),
        nonce: digest("challenge_nonce", HEX_A),
        issued_at_ms: 1_000_000,
        expires_at_ms: 1_060_000,
    }
}

fn valid_challenge() -> PassportChallengeV1 {
    let payload = valid_payload();

    PassportChallengeV1 {
        version: payload.version,
        challenge_id: payload.challenge_id,
        network_id: payload.network_id,
        environment: payload.environment,
        audience: payload.audience,
        issuing_service_id: payload.issuing_service_id,
        service_key_id: payload.service_key_id,
        purpose: payload.purpose,
        requested_scopes: payload.requested_scopes,
        passport_id: payload.passport_id,
        device_id: payload.device_id,
        operation_body_hash: payload.operation_body_hash,
        nonce: payload.nonce,
        issued_at_ms: payload.issued_at_ms,
        expires_at_ms: payload.expires_at_ms,
        service_signature: Ed25519SignatureV1::from_bytes([0x44; 64]),
    }
}

#[test]
fn challenge_wire_roundtrips_strictly_and_unknown_fields_fail_closed() {
    let challenge = valid_challenge();

    challenge.validate().expect("valid challenge");

    let encoded = serde_json::to_vec(&challenge).expect("serialize challenge");
    let decoded: PassportChallengeV1 =
        serde_json::from_slice(&encoded).expect("deserialize challenge");

    assert_eq!(decoded, challenge);
    assert_eq!(decoded.signing_payload(), valid_payload());

    let mut value = serde_json::to_value(&challenge).expect("challenge value");
    value
        .as_object_mut()
        .expect("challenge object")
        .insert("unexpected_authority".to_string(), serde_json::json!(true));

    assert!(serde_json::from_value::<PassportChallengeV1>(value).is_err());

    assert_eq!(PASSPORT_CHALLENGE_V1_MAX_TTL_MS, 300_000);
    assert_eq!(PASSPORT_CHALLENGE_V1_MAX_CLOCK_SKEW_MS, 30_000);
    assert_eq!(PASSPORT_CHALLENGE_V1_SERVICE_SIGNATURE_ALGORITHM, "ed25519");
}

#[test]
fn purpose_vocabulary_is_closed_and_has_stable_wire_spelling() {
    let cases = [
        (PassportChallengePurposeV1::RegisterRoot, "register_root"),
        (
            PassportChallengePurposeV1::AuthorizeDevice,
            "authorize_device",
        ),
        (PassportChallengePurposeV1::RevokeDevice, "revoke_device"),
        (PassportChallengePurposeV1::ProveSession, "prove_session"),
        (
            PassportChallengePurposeV1::IssueCapability,
            "issue_capability",
        ),
        (
            PassportChallengePurposeV1::RefreshCapability,
            "refresh_capability",
        ),
        (PassportChallengePurposeV1::ClaimUsername, "claim_username"),
        (
            PassportChallengePurposeV1::TransferUsername,
            "transfer_username",
        ),
        (
            PassportChallengePurposeV1::ReleaseUsername,
            "release_username",
        ),
        (
            PassportChallengePurposeV1::PublishProfile,
            "publish_profile",
        ),
        (PassportChallengePurposeV1::RegisterSite, "register_site"),
        (PassportChallengePurposeV1::UpdateSite, "update_site"),
        (
            PassportChallengePurposeV1::WalletAuthorizationRequestReserved,
            "wallet_authorization_request_reserved",
        ),
    ];

    for (purpose, expected) in cases {
        assert_eq!(purpose.as_str(), expected);

        let encoded = serde_json::to_string(&purpose).expect("serialize purpose");
        assert_eq!(encoded, format!("\"{expected}\""));

        let decoded: PassportChallengePurposeV1 =
            serde_json::from_str(&encoded).expect("deserialize purpose");

        assert_eq!(decoded, purpose);
    }

    assert!(
        serde_json::from_str::<PassportChallengePurposeV1>("\"future_unknown_purpose\"").is_err()
    );
}

#[test]
fn scopes_must_be_nonempty_bounded_sorted_and_unique() {
    let mut empty = valid_payload();
    empty.requested_scopes.clear();

    assert_eq!(
        empty.validate(),
        Err(PassportChallengeValidationError::EmptyRequestedScopes)
    );

    let mut duplicate = valid_payload();
    duplicate.requested_scopes = vec![
        scope("catalog.read"),
        scope("catalog.read"),
        scope("identity.read"),
    ];

    assert_eq!(
        duplicate.validate(),
        Err(PassportChallengeValidationError::DuplicateRequestedScope)
    );

    let mut unsorted = valid_payload();
    unsorted.requested_scopes = vec![scope("identity.read"), scope("catalog.read")];

    assert_eq!(
        unsorted.validate(),
        Err(PassportChallengeValidationError::NonCanonicalRequestedScopeOrder)
    );
}

#[test]
fn purpose_required_bindings_and_reserved_wallet_fail_closed() {
    let mut missing_passport = valid_payload();
    missing_passport.purpose = PassportChallengePurposeV1::RegisterRoot;
    missing_passport.passport_id = None;

    assert_eq!(
        missing_passport.validate(),
        Err(PassportChallengeValidationError::MissingPassportBinding)
    );

    let mut missing_device = valid_payload();
    missing_device.purpose = PassportChallengePurposeV1::RevokeDevice;
    missing_device.device_id = None;

    assert_eq!(
        missing_device.validate(),
        Err(PassportChallengeValidationError::MissingDeviceBinding)
    );

    let mut missing_body = valid_payload();
    missing_body.purpose = PassportChallengePurposeV1::RegisterRoot;
    missing_body.operation_body_hash = None;

    assert_eq!(
        missing_body.validate(),
        Err(PassportChallengeValidationError::MissingOperationBodyHash)
    );

    let mut reserved = valid_payload();
    reserved.purpose = PassportChallengePurposeV1::WalletAuthorizationRequestReserved;

    assert_eq!(
        reserved.validate(),
        Err(PassportChallengeValidationError::ReservedWalletPurpose)
    );
}

#[test]
fn service_key_id_accepts_kms_namespace_and_rejects_drift() {
    let kid = ServiceKeyIdV1::parse("ed25519/default/v1").expect("canonical service KID");

    assert_eq!(kid.as_str(), "ed25519/default/v1",);

    for invalid in [
        "",
        "/ed25519/default/v1",
        "Ed25519/default/v1",
        "ed25519 default v1",
        "ed25519/default/v1?",
    ] {
        assert_eq!(
            ServiceKeyIdV1::parse(invalid),
            Err(PassportChallengeValidationError::InvalidServiceKeyId),
        );
    }
}

#[test]
fn version_and_time_window_are_strict_and_bounded() {
    let mut wrong_version = valid_payload();
    wrong_version.version = 2;

    assert_eq!(
        wrong_version.validate(),
        Err(PassportChallengeValidationError::UnsupportedVersion)
    );

    let mut zero_issue = valid_payload();
    zero_issue.issued_at_ms = 0;

    assert_eq!(
        zero_issue.validate(),
        Err(PassportChallengeValidationError::InvalidIssuedAt)
    );

    let mut backwards = valid_payload();
    backwards.expires_at_ms = backwards.issued_at_ms;

    assert_eq!(
        backwards.validate(),
        Err(PassportChallengeValidationError::InvalidExpiry)
    );

    let mut too_long = valid_payload();
    too_long.expires_at_ms = too_long.issued_at_ms + PASSPORT_CHALLENGE_V1_MAX_TTL_MS + 1;

    assert_eq!(
        too_long.validate(),
        Err(PassportChallengeValidationError::ChallengeTtlTooLong)
    );
}
