//! RO:WHAT — Cryptographic acceptance tests for Native Passport V1 protected-request DeviceKey proofs.
//! RO:WHY — CN-4 capability authority must not authorize username/profile mutation unless the exact request is signed by the capability-bound DeviceKey.
//! RO:INTERACTS — ron-auth canonical request-proof transcript/verifier and ron-proto Capability/Device/B3/request-proof DTOs.
//! RO:INVARIANTS — exact canonical bytes verify; every request binding mutation rejects; stale/future proofs reject; wrong DeviceKey rejects; JSON bytes are never accepted as signing input.
//! RO:METRICS — none.
//! RO:CONFIG — deterministic test values and the 30-second V1 verifier clock-skew ceiling.
//! RO:SECURITY — deterministic test-only Ed25519 key; no physical DeviceKey, persistence, replay mutation, capability issuance, username mutation, wallet, or ledger authority.
//! RO:TEST — cargo test -p ron-auth --test native_passport_request_proof.

use ed25519_dalek::{Signer as _, SigningKey};
use ron_auth::native_passport::{
    canonical_passport_request_proof_v1_transcript, passport_request_proof_v1_transcript_b3_hex,
    verify_passport_request_proof_v1_strict, PassportRequestProofVerificationContextV1,
    PassportRequestProofVerificationError, PASSPORT_REQUEST_PROOF_V1_CANONICAL_TRANSCRIPT_ENCODING,
    PASSPORT_REQUEST_PROOF_V1_JSON_TRANSCRIPT_SIGNABLE,
    PASSPORT_REQUEST_PROOF_V1_MAX_CLOCK_SKEW_MS, PASSPORT_REQUEST_PROOF_V1_TRANSCRIPT_DOMAIN,
};
use ron_proto::{
    B3DigestHex, CapabilityIdV1, DeviceIdV1, Ed25519PublicKeyHex, Ed25519SignatureV1,
    PassportRequestProofV1, PASSPORT_REQUEST_PROOF_V1_VERSION,
};

const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

struct Fixture {
    signing_key: SigningKey,
    public_key: Ed25519PublicKeyHex,
    capability_id: CapabilityIdV1,
    device_id: DeviceIdV1,
    query_hash: B3DigestHex,
    body_hash: B3DigestHex,
}

impl Fixture {
    fn new() -> Self {
        let signing_key = SigningKey::from_bytes(&[0x63; 32]);

        let public_key =
            Ed25519PublicKeyHex::parse(lower_hex(signing_key.verifying_key().as_bytes()))
                .expect("device public key");

        Self {
            signing_key,
            public_key,
            capability_id: CapabilityIdV1::parse(format!("capability:v1:b3:{HEX_A}"))
                .expect("capability ID"),
            device_id: DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_B}"))
                .expect("Device ID"),
            query_hash: digest("canonical_query_hash", HEX_C),
            body_hash: digest("body_hash", HEX_D),
        }
    }

    fn unsigned_proof(&self) -> PassportRequestProofV1 {
        PassportRequestProofV1 {
            version: PASSPORT_REQUEST_PROOF_V1_VERSION,
            capability_id: self.capability_id.clone(),
            request_method: "POST".to_owned(),
            canonical_path: "/identity/passport/username/claim".to_owned(),
            canonical_query_hash: self.query_hash.clone(),
            body_hash: self.body_hash.clone(),
            timestamp_ms: 1_000_000,
            request_nonce: digest("request_nonce", HEX_E),
            device_id: self.device_id.clone(),
            device_signature: Ed25519SignatureV1::from_bytes([0_u8; 64]),
        }
    }

    fn signed_proof(&self) -> PassportRequestProofV1 {
        let mut proof = self.unsigned_proof();

        let transcript = canonical_passport_request_proof_v1_transcript(&proof)
            .expect("canonical request-proof transcript");

        proof.device_signature =
            Ed25519SignatureV1::from_bytes(self.signing_key.sign(&transcript).to_bytes());

        proof
    }

    fn context(&self, now_ms: u64) -> PassportRequestProofVerificationContextV1<'_> {
        PassportRequestProofVerificationContextV1 {
            trusted_device_public_key: &self.public_key,
            expected_capability_id: &self.capability_id,
            expected_device_id: &self.device_id,
            expected_request_method: "POST",
            expected_canonical_path: "/identity/passport/username/claim",
            expected_canonical_query_hash: &self.query_hash,
            expected_body_hash: &self.body_hash,
            now_ms,
            max_clock_skew_ms: 30_000,
        }
    }
}

#[test]
fn canonical_request_proof_bytes_sign_and_verify() {
    let fixture = Fixture::new();
    let proof = fixture.signed_proof();

    verify_passport_request_proof_v1_strict(&proof, fixture.context(1_020_000))
        .expect("strict request-proof verification");

    let first = canonical_passport_request_proof_v1_transcript(&proof).expect("first transcript");

    let second = canonical_passport_request_proof_v1_transcript(&proof).expect("second transcript");

    assert_eq!(first, second);

    let first_hash = passport_request_proof_v1_transcript_b3_hex(&proof).expect("first audit hash");

    let second_hash =
        passport_request_proof_v1_transcript_b3_hex(&proof).expect("second audit hash");

    assert_eq!(first_hash, second_hash);
    assert_eq!(first_hash.len(), 64);

    assert_eq!(
        PASSPORT_REQUEST_PROOF_V1_TRANSCRIPT_DOMAIN,
        "rustyonions.native-passport.request-proof.v1",
    );

    assert_eq!(
        PASSPORT_REQUEST_PROOF_V1_CANONICAL_TRANSCRIPT_ENCODING,
        "length-prefixed-binary-v1",
    );

    assert_eq!(PASSPORT_REQUEST_PROOF_V1_MAX_CLOCK_SKEW_MS, 30_000,);

    assert!(!PASSPORT_REQUEST_PROOF_V1_JSON_TRANSCRIPT_SIGNABLE);
}

#[test]
fn every_request_binding_mutation_rejects() {
    let fixture = Fixture::new();
    let original = fixture.signed_proof();

    let mut changed = original.clone();
    changed.capability_id =
        CapabilityIdV1::parse(format!("capability:v1:b3:{HEX_B}")).expect("other capability ID");

    assert_eq!(
        verify_passport_request_proof_v1_strict(&changed, fixture.context(1_020_000),),
        Err(PassportRequestProofVerificationError::CapabilityIdMismatch),
    );

    let mut changed = original.clone();
    changed.device_id =
        DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}")).expect("other Device ID");

    assert_eq!(
        verify_passport_request_proof_v1_strict(&changed, fixture.context(1_020_000),),
        Err(PassportRequestProofVerificationError::DeviceIdMismatch),
    );

    let mut changed = original.clone();
    changed.request_method = "GET".to_owned();

    assert_eq!(
        verify_passport_request_proof_v1_strict(&changed, fixture.context(1_020_000),),
        Err(PassportRequestProofVerificationError::RequestMethodMismatch),
    );

    let mut changed = original.clone();
    changed.canonical_path = "/identity/passport/profile".to_owned();

    assert_eq!(
        verify_passport_request_proof_v1_strict(&changed, fixture.context(1_020_000),),
        Err(PassportRequestProofVerificationError::CanonicalPathMismatch),
    );

    let mut changed = original.clone();
    changed.canonical_query_hash = digest("canonical_query_hash", HEX_E);

    assert_eq!(
        verify_passport_request_proof_v1_strict(&changed, fixture.context(1_020_000),),
        Err(PassportRequestProofVerificationError::CanonicalQueryHashMismatch),
    );

    let mut changed = original.clone();
    changed.body_hash = digest("body_hash", HEX_A);

    assert_eq!(
        verify_passport_request_proof_v1_strict(&changed, fixture.context(1_020_000),),
        Err(PassportRequestProofVerificationError::BodyHashMismatch),
    );

    let mut changed = original.clone();
    changed.timestamp_ms += 1;

    assert_eq!(
        verify_passport_request_proof_v1_strict(&changed, fixture.context(1_020_000),),
        Err(PassportRequestProofVerificationError::InvalidDeviceSignature),
    );

    let mut changed = original;
    changed.request_nonce = digest("request_nonce", HEX_A);

    assert_eq!(
        verify_passport_request_proof_v1_strict(&changed, fixture.context(1_020_000),),
        Err(PassportRequestProofVerificationError::InvalidDeviceSignature),
    );
}

#[test]
fn stale_future_wrong_key_and_excessive_skew_fail_closed() {
    let fixture = Fixture::new();
    let proof = fixture.signed_proof();

    assert_eq!(
        verify_passport_request_proof_v1_strict(&proof, fixture.context(1_030_001),),
        Err(PassportRequestProofVerificationError::Stale),
    );

    assert_eq!(
        verify_passport_request_proof_v1_strict(&proof, fixture.context(969_999),),
        Err(PassportRequestProofVerificationError::NotYetValid),
    );

    let wrong_signing_key = SigningKey::from_bytes(&[0x64; 32]);

    let wrong_public_key =
        Ed25519PublicKeyHex::parse(lower_hex(wrong_signing_key.verifying_key().as_bytes()))
            .expect("wrong public key");

    let mut wrong_key_context = fixture.context(1_020_000);
    wrong_key_context.trusted_device_public_key = &wrong_public_key;

    assert_eq!(
        verify_passport_request_proof_v1_strict(&proof, wrong_key_context,),
        Err(PassportRequestProofVerificationError::InvalidDeviceSignature),
    );

    let mut excessive_skew = fixture.context(1_020_000);
    excessive_skew.max_clock_skew_ms = 30_001;

    assert_eq!(
        verify_passport_request_proof_v1_strict(&proof, excessive_skew,),
        Err(PassportRequestProofVerificationError::ClockSkewPolicyExceeded),
    );
}

#[test]
fn json_bytes_are_never_accepted_as_request_proof_signature() {
    let fixture = Fixture::new();
    let mut proof = fixture.unsigned_proof();

    proof.device_signature = Ed25519SignatureV1::from_bytes(
        fixture
            .signing_key
            .sign(br#"{"capability_id":"not-the-canonical-transcript"}"#)
            .to_bytes(),
    );

    assert_eq!(
        verify_passport_request_proof_v1_strict(&proof, fixture.context(1_020_000),),
        Err(PassportRequestProofVerificationError::InvalidDeviceSignature),
    );
}

fn digest(label: &'static str, value: &str) -> B3DigestHex {
    B3DigestHex::parse(label, value).expect(label)
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }

    output
}
