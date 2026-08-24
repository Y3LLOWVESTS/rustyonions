//! RO:WHAT — Cryptographic acceptance tests for Native Passport signed service challenges.
//! RO:WHY — Every signed challenge field must be transcript-bound and verified against trusted service context before root/device signing.
//! RO:INTERACTS — ron-proto PassportChallengeV1, ron-auth canonical challenge transcript, Ed25519 signing fixtures, and strict service verification.
//! RO:INVARIANTS — exact transcript bytes are signed; every field mutation rejects; wrong service key/context/time rejects; JSON is never signable.
//! RO:SECURITY — deterministic test-only Ed25519 keys; no production key loading, persistence, network, replay, wallet, or ledger behavior.
//! RO:TEST — cargo test -p ron-auth --test native_passport_challenge_v1.

use std::fmt::Write as _;

use ed25519_dalek::{Signer as _, SigningKey};
use ron_auth::native_passport::{
    canonical_passport_challenge_v1_transcript, passport_challenge_v1_transcript_b3_hex,
    verify_passport_challenge_v1_strict, PassportChallengeVerificationContextV1,
    PASSPORT_CHALLENGE_V1_CANONICAL_TRANSCRIPT_ENCODING,
    PASSPORT_CHALLENGE_V1_JSON_TRANSCRIPT_SIGNABLE, PASSPORT_CHALLENGE_V1_TRANSCRIPT_DOMAIN,
};
use ron_proto::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, Ed25519PublicKeyHex, Ed25519SignatureV1,
    NativePassportContextLabelV1, NativePassportScopeV1, PassportChallengePurposeV1,
    PassportChallengeSigningPayloadV1, PassportChallengeV1, PassportIdV1, ServiceKeyIdV1,
    PASSPORT_CHALLENGE_V1_VERSION,
};

const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

struct Fixture {
    signing_key: SigningKey,
    service_public_key: Ed25519PublicKeyHex,
    network: NativePassportContextLabelV1,
    environment: NativePassportContextLabelV1,
    audience: NativePassportContextLabelV1,
    issuing_service: NativePassportContextLabelV1,
    service_key_id: ServiceKeyIdV1,
}

impl Fixture {
    fn new() -> Self {
        let signing_key = SigningKey::from_bytes(&[0x55; 32]);

        let service_public_key =
            Ed25519PublicKeyHex::parse(lower_hex(&signing_key.verifying_key().to_bytes()))
                .expect("service public key");

        Self {
            signing_key,
            service_public_key,
            network: context("rustyonions-devnet"),
            environment: context("private-beta"),
            audience: context("svc-passport"),
            issuing_service: context("svc-passport"),
            service_key_id: service_key_id("ed25519/default/v1"),
        }
    }

    fn payload(&self) -> PassportChallengeSigningPayloadV1 {
        PassportChallengeSigningPayloadV1 {
            version: PASSPORT_CHALLENGE_V1_VERSION,
            challenge_id: challenge_id(HEX_A),
            network_id: self.network.clone(),
            environment: self.environment.clone(),
            audience: self.audience.clone(),
            issuing_service_id: self.issuing_service.clone(),
            service_key_id: self.service_key_id.clone(),
            purpose: PassportChallengePurposeV1::IssueCapability,
            requested_scopes: vec![
                scope("catalog.read"),
                scope("content.read"),
                scope("identity.read"),
            ],
            passport_id: Some(passport_id(HEX_B)),
            device_id: Some(device_id(HEX_C)),
            operation_body_hash: Some(digest("operation_body_hash", HEX_D)),
            nonce: digest("challenge_nonce", HEX_E),
            issued_at_ms: 1_000_000,
            expires_at_ms: 1_060_000,
        }
    }

    fn signed_challenge(&self) -> PassportChallengeV1 {
        let payload = self.payload();

        let transcript =
            canonical_passport_challenge_v1_transcript(&payload).expect("challenge transcript");

        let signature = self.signing_key.sign(&transcript);

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
            service_signature: Ed25519SignatureV1::from_bytes(signature.to_bytes()),
        }
    }

    fn context(&self, now_ms: u64) -> PassportChallengeVerificationContextV1<'_> {
        PassportChallengeVerificationContextV1 {
            trusted_service_public_key: &self.service_public_key,
            expected_network_id: &self.network,
            expected_environment: &self.environment,
            expected_audience: &self.audience,
            expected_issuing_service_id: &self.issuing_service,
            expected_service_key_id: &self.service_key_id,
            now_ms,
            max_clock_skew_ms: 10_000,
        }
    }
}

fn context(value: &str) -> NativePassportContextLabelV1 {
    NativePassportContextLabelV1::parse(value).expect("context")
}

fn scope(value: &str) -> NativePassportScopeV1 {
    NativePassportScopeV1::parse(value).expect("scope")
}

fn service_key_id(value: &str) -> ServiceKeyIdV1 {
    ServiceKeyIdV1::parse(value).expect("service key id")
}

fn digest(label: &'static str, value: &str) -> B3DigestHex {
    B3DigestHex::parse(label, value).expect("digest")
}

fn challenge_id(hex: &str) -> ChallengeIdV1 {
    ChallengeIdV1::parse(format!("challenge:v1:b3:{hex}")).expect("challenge id")
}

fn passport_id(hex: &str) -> PassportIdV1 {
    PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{hex}")).expect("passport id")
}

fn device_id(hex: &str) -> DeviceIdV1 {
    DeviceIdV1::parse(format!("device:v1:ed25519:b3:{hex}")).expect("device id")
}

fn lower_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("hex write");
    }

    output
}

fn assert_rejected(fixture: &Fixture, challenge: &PassportChallengeV1) {
    assert!(verify_passport_challenge_v1_strict(challenge, fixture.context(1_030_000)).is_err());
}

#[test]
fn canonical_signed_challenge_cross_verifies_with_trusted_service_key() {
    let fixture = Fixture::new();
    let challenge = fixture.signed_challenge();

    verify_passport_challenge_v1_strict(&challenge, fixture.context(1_030_000))
        .expect("strict challenge verification");

    let first = canonical_passport_challenge_v1_transcript(&challenge.signing_payload())
        .expect("first transcript");

    let second = canonical_passport_challenge_v1_transcript(&challenge.signing_payload())
        .expect("second transcript");

    assert_eq!(first, second);

    let first_hash =
        passport_challenge_v1_transcript_b3_hex(&challenge.signing_payload()).expect("first hash");

    let second_hash =
        passport_challenge_v1_transcript_b3_hex(&challenge.signing_payload()).expect("second hash");

    assert_eq!(first_hash, second_hash);
    assert_eq!(
        PASSPORT_CHALLENGE_V1_TRANSCRIPT_DOMAIN,
        "rustyonions.native-passport.service-challenge.v1"
    );
    assert_eq!(
        PASSPORT_CHALLENGE_V1_CANONICAL_TRANSCRIPT_ENCODING,
        "length-prefixed-binary-v1"
    );
    const {
        assert!(!PASSPORT_CHALLENGE_V1_JSON_TRANSCRIPT_SIGNABLE);
    }
}

#[test]
fn every_signed_challenge_field_mutation_is_rejected() {
    let fixture = Fixture::new();
    let original = fixture.signed_challenge();

    let mut changed = original.clone();
    changed.version = 2;
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.challenge_id = challenge_id(HEX_B);
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.network_id = context("rustyonions-other");
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.environment = context("other-beta");
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.audience = context("other-service");
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.issuing_service_id = context("other-passport");
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.service_key_id = service_key_id("ed25519/default/v2");
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.purpose = PassportChallengePurposeV1::RefreshCapability;
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.requested_scopes = vec![
        scope("catalog.read"),
        scope("entitlement.read"),
        scope("identity.read"),
    ];
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.passport_id = Some(passport_id(HEX_C));
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.device_id = Some(device_id(HEX_D));
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.operation_body_hash = Some(digest("operation_body_hash", HEX_E));
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.nonce = digest("challenge_nonce", HEX_A);
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.issued_at_ms += 1;
    assert_rejected(&fixture, &changed);

    let mut changed = original.clone();
    changed.expires_at_ms += 1;
    assert_rejected(&fixture, &changed);

    let mut changed = original;
    changed.service_signature = Ed25519SignatureV1::from_bytes([0x99; 64]);
    assert_rejected(&fixture, &changed);
}

#[test]
fn trusted_context_and_time_fail_closed_before_client_signing() {
    let fixture = Fixture::new();
    let challenge = fixture.signed_challenge();

    let mut not_yet_valid = fixture.context(900_000);
    not_yet_valid.max_clock_skew_ms = 0;

    assert!(verify_passport_challenge_v1_strict(&challenge, not_yet_valid).is_err());

    let mut expired = fixture.context(1_070_001);
    expired.max_clock_skew_ms = 10_000;

    assert!(verify_passport_challenge_v1_strict(&challenge, expired).is_err());

    let wrong_signing_key = SigningKey::from_bytes(&[0x66; 32]);

    let wrong_public_key =
        Ed25519PublicKeyHex::parse(lower_hex(&wrong_signing_key.verifying_key().to_bytes()))
            .expect("wrong public key");

    let mut wrong_key_context = fixture.context(1_030_000);
    wrong_key_context.trusted_service_public_key = &wrong_public_key;

    assert!(verify_passport_challenge_v1_strict(&challenge, wrong_key_context).is_err());

    let mut excessive_skew = fixture.context(1_030_000);
    excessive_skew.max_clock_skew_ms = 30_001;

    assert!(verify_passport_challenge_v1_strict(&challenge, excessive_skew).is_err());
}
