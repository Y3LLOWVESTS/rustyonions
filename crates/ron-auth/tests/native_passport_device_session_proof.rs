//! RO:WHAT — Cryptographic acceptance tests for the canonical Native Passport V1 device-session possession proof.
//! RO:WHY — CN-4 requires real device-key possession evidence before capability issuance or username/profile mutation.
//! RO:INTERACTS — `ron-auth` device-session transcript/verifier, `ron-proto` canonical Passport/Device/scope/signature DTOs, and deterministic Ed25519 test keys.
//! RO:INVARIANTS — canonical bytes verify; scope order canonicalizes; duplicate scopes reject; device/challenge binding mutation rejects; JSON and legacy pipe signatures are never accepted.
//! RO:METRICS — none.
//! RO:CONFIG — deterministic nonphysical test material only.
//! RO:SECURITY — test-only signing keys; no physical device key, vault, PIN, HTTP, replay state, capability issuance, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — `cargo test -p ron-auth --test native_passport_device_session_proof`.

use ed25519_dalek::{Signer as _, SigningKey};
use ron_auth::native_passport::{
    canonical_device_session_proof_v1_transcript, device_session_proof_v1_transcript_b3_hex,
    verify_device_session_proof_v1_strict, DeviceSessionProofError, DeviceSessionProofTranscriptV1,
    DEVICE_SESSION_PROOF_V1_CANONICAL_TRANSCRIPT_ENCODING,
    DEVICE_SESSION_PROOF_V1_JSON_TRANSCRIPT_SIGNABLE,
    DEVICE_SESSION_PROOF_V1_LEGACY_PHASE0_TRANSCRIPT_SIGNABLE,
    DEVICE_SESSION_PROOF_V1_TRANSCRIPT_DOMAIN, ROOT_REGISTRATION_PROOF_V1_TRANSCRIPT_DOMAIN,
};
use ron_proto::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, Ed25519PublicKeyHex, Ed25519SignatureV1,
    NativePassportContextLabelV1, NativePassportScopeV1, PassportIdV1,
};

const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

struct Fixture {
    signing_key: SigningKey,
    public_key: Ed25519PublicKeyHex,
    challenge_id: ChallengeIdV1,
    passport_id: PassportIdV1,
    device_id: DeviceIdV1,
    challenge_hash: B3DigestHex,
    network: NativePassportContextLabelV1,
    environment: NativePassportContextLabelV1,
    audience: NativePassportContextLabelV1,
    scopes: Vec<NativePassportScopeV1>,
}

impl Fixture {
    fn new() -> Self {
        let signing_key = SigningKey::from_bytes(&[0x42; 32]);

        let public_key =
            Ed25519PublicKeyHex::parse(lower_hex(&signing_key.verifying_key().to_bytes()))
                .expect("device public key");

        Self {
            signing_key,
            public_key,
            challenge_id: ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_A}"))
                .expect("challenge ID"),
            passport_id: PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_B}"))
                .expect("Passport ID"),
            device_id: DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}"))
                .expect("Device ID"),
            challenge_hash: B3DigestHex::parse("challenge_transcript_hash", HEX_D)
                .expect("challenge transcript hash"),
            network: NativePassportContextLabelV1::parse("rustyonions-devnet").expect("network"),
            environment: NativePassportContextLabelV1::parse("private-beta").expect("environment"),
            audience: NativePassportContextLabelV1::parse("svc-passport").expect("audience"),
            scopes: vec![scope("catalog.read"), scope("identity.read")],
        }
    }

    fn transcript<'a>(
        &'a self,
        scopes: &'a [NativePassportScopeV1],
    ) -> DeviceSessionProofTranscriptV1<'a> {
        DeviceSessionProofTranscriptV1 {
            challenge_contract_domain: "native-passport/proof-challenge-contract/v1",
            challenge_contract_version: 1,
            proof_contract_domain: "native-passport/proof-contract/v1",
            proof_contract_version: 1,
            challenge_id: &self.challenge_id,
            network_id: &self.network,
            environment: &self.environment,
            audience: &self.audience,
            passport_id: &self.passport_id,
            device_id: &self.device_id,
            device_public_key: &self.public_key,
            challenge_transcript_hash: &self.challenge_hash,
            requested_scopes: scopes,
            challenge_issued_at_ms: 1_000_000,
            challenge_expires_at_ms: 1_060_000,
            proof_created_at_ms: 1_030_000,
        }
    }

    fn sign(&self, input: &DeviceSessionProofTranscriptV1<'_>) -> Ed25519SignatureV1 {
        let transcript = canonical_device_session_proof_v1_transcript(input)
            .expect("canonical device-session transcript");

        Ed25519SignatureV1::from_bytes(self.signing_key.sign(&transcript).to_bytes())
    }
}

#[test]
fn canonical_device_session_bytes_sign_and_verify() {
    let fixture = Fixture::new();
    let input = fixture.transcript(&fixture.scopes);

    let transcript =
        canonical_device_session_proof_v1_transcript(&input).expect("canonical transcript");

    assert!(!transcript.is_empty());

    let signature =
        Ed25519SignatureV1::from_bytes(fixture.signing_key.sign(&transcript).to_bytes());

    verify_device_session_proof_v1_strict(&input, &signature)
        .expect("device possession proof should verify");

    let digest_one = device_session_proof_v1_transcript_b3_hex(&input).expect("audit digest");

    let digest_two =
        device_session_proof_v1_transcript_b3_hex(&input).expect("repeat audit digest");

    assert_eq!(digest_one, digest_two);
    assert_eq!(digest_one.len(), 64);

    assert_eq!(
        DEVICE_SESSION_PROOF_V1_TRANSCRIPT_DOMAIN,
        "rustyonions.native-passport.challenge-proof.v1",
    );

    assert_eq!(
        DEVICE_SESSION_PROOF_V1_TRANSCRIPT_DOMAIN,
        ROOT_REGISTRATION_PROOF_V1_TRANSCRIPT_DOMAIN,
    );

    assert_eq!(
        DEVICE_SESSION_PROOF_V1_CANONICAL_TRANSCRIPT_ENCODING,
        "length-prefixed-binary-v1",
    );

    assert!(!DEVICE_SESSION_PROOF_V1_LEGACY_PHASE0_TRANSCRIPT_SIGNABLE);
    assert!(!DEVICE_SESSION_PROOF_V1_JSON_TRANSCRIPT_SIGNABLE);
}

#[test]
fn scope_order_canonicalizes_and_duplicates_reject() {
    let fixture = Fixture::new();

    let one = vec![scope("identity.read"), scope("catalog.read")];

    let two = vec![scope("catalog.read"), scope("identity.read")];

    let transcript_one = canonical_device_session_proof_v1_transcript(&fixture.transcript(&one))
        .expect("first scope ordering");

    let transcript_two = canonical_device_session_proof_v1_transcript(&fixture.transcript(&two))
        .expect("second scope ordering");

    assert_eq!(transcript_one, transcript_two);

    let duplicate = vec![scope("identity.read"), scope("identity.read")];

    assert_eq!(
        canonical_device_session_proof_v1_transcript(&fixture.transcript(&duplicate),),
        Err(DeviceSessionProofError::DuplicateScope),
    );
}

#[test]
fn mutated_device_and_challenge_bindings_reject_original_signature() {
    let fixture = Fixture::new();
    let baseline = fixture.transcript(&fixture.scopes);
    let signature = fixture.sign(&baseline);

    let other_device_id =
        DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_D}")).expect("other Device ID");

    let mut changed_device_id = baseline;
    changed_device_id.device_id = &other_device_id;

    assert_eq!(
        verify_device_session_proof_v1_strict(&changed_device_id, &signature,),
        Err(DeviceSessionProofError::InvalidSignature),
    );

    let other_signing_key = SigningKey::from_bytes(&[0x55; 32]);

    let other_public_key =
        Ed25519PublicKeyHex::parse(lower_hex(&other_signing_key.verifying_key().to_bytes()))
            .expect("other public key");

    let mut changed_public_key = baseline;
    changed_public_key.device_public_key = &other_public_key;

    assert_eq!(
        verify_device_session_proof_v1_strict(&changed_public_key, &signature,),
        Err(DeviceSessionProofError::InvalidSignature),
    );

    let other_hash =
        B3DigestHex::parse("challenge_transcript_hash", HEX_A).expect("other challenge hash");

    let mut changed_challenge = baseline;
    changed_challenge.challenge_transcript_hash = &other_hash;

    assert_eq!(
        verify_device_session_proof_v1_strict(&changed_challenge, &signature,),
        Err(DeviceSessionProofError::InvalidSignature),
    );
}

#[test]
fn proof_time_must_be_inside_challenge_window() {
    let fixture = Fixture::new();

    let mut early = fixture.transcript(&fixture.scopes);
    early.proof_created_at_ms = early.challenge_issued_at_ms - 1;

    assert_eq!(
        canonical_device_session_proof_v1_transcript(&early),
        Err(DeviceSessionProofError::InvalidProofCreatedAt),
    );

    let mut late = fixture.transcript(&fixture.scopes);
    late.proof_created_at_ms = late.challenge_expires_at_ms + 1;

    assert_eq!(
        canonical_device_session_proof_v1_transcript(&late),
        Err(DeviceSessionProofError::InvalidProofCreatedAt),
    );
}

#[test]
fn json_and_legacy_pipe_signatures_never_verify() {
    let fixture = Fixture::new();
    let input = fixture.transcript(&fixture.scopes);

    let pipe_signature = Ed25519SignatureV1::from_bytes(
        fixture
            .signing_key
            .sign(b"rustyonions.native-passport.challenge-proof.v1|prove_session")
            .to_bytes(),
    );

    assert_eq!(
        verify_device_session_proof_v1_strict(&input, &pipe_signature,),
        Err(DeviceSessionProofError::InvalidSignature),
    );

    let json_signature = Ed25519SignatureV1::from_bytes(
        fixture
            .signing_key
            .sign(br#"{"purpose":"prove_session"}"#)
            .to_bytes(),
    );

    assert_eq!(
        verify_device_session_proof_v1_strict(&input, &json_signature,),
        Err(DeviceSessionProofError::InvalidSignature),
    );
}

fn scope(value: &str) -> NativePassportScopeV1 {
    NativePassportScopeV1::parse(value).expect("scope")
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
