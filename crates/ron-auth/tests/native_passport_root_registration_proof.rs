//! RO:WHAT — Cryptographic acceptance tests for the canonical Native Passport V1 root-registration proof.
//! RO:WHY — Durable Passport root registration must require real Ed25519 control of the concrete root key, not a descriptive verifier label or injected accepted=true evidence.
//! RO:INTERACTS — ron-auth root-registration transcript/strict verifier, ron-proto canonical IDs, and ed25519-dalek test-only signing keys.
//! RO:INVARIANTS — canonical bytes verify; wrong key or mutated binding rejects; scope ordering canonicalizes; duplicate scopes reject; legacy pipe and JSON signatures never verify as canonical proofs.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — deterministic test keys only; no recovery material, vault, filesystem authority, network I/O, server registry mutation, capability issuance, wallet, or ledger mutation.
//! RO:TEST — cargo test -p ron-auth --test native_passport_root_registration_proof.

use ed25519_dalek::{Signer as _, SigningKey};
use ron_auth::native_passport::{
    canonical_root_registration_proof_v1_transcript, root_registration_proof_v1_transcript_b3_hex,
    verify_root_registration_proof_v1_strict, RootRegistrationProofError,
    RootRegistrationProofTranscriptV1, ROOT_REGISTRATION_PROOF_V1_CANONICAL_TRANSCRIPT_ENCODING,
    ROOT_REGISTRATION_PROOF_V1_JSON_TRANSCRIPT_SIGNABLE,
    ROOT_REGISTRATION_PROOF_V1_LEGACY_PHASE0_TRANSCRIPT_ENCODING,
    ROOT_REGISTRATION_PROOF_V1_LEGACY_PHASE0_TRANSCRIPT_SIGNABLE,
    ROOT_REGISTRATION_PROOF_V1_TRANSCRIPT_DOMAIN,
};
use ron_proto::{B3DigestHex, ChallengeIdV1, DeviceIdV1, Ed25519PublicKeyHex, PassportIdV1};

const _: [(); 0] = [(); ROOT_REGISTRATION_PROOF_V1_LEGACY_PHASE0_TRANSCRIPT_SIGNABLE as usize];

const _: [(); 0] = [(); ROOT_REGISTRATION_PROOF_V1_JSON_TRANSCRIPT_SIGNABLE as usize];

struct Fixture {
    challenge_id: ChallengeIdV1,
    passport_id: PassportIdV1,
    root_public_key: Ed25519PublicKeyHex,
    device_id: DeviceIdV1,
    operation_body_hash: B3DigestHex,
    challenge_transcript_hash: B3DigestHex,
}

impl Fixture {
    fn new() -> Self {
        Self {
            challenge_id: ChallengeIdV1::parse(format!("challenge:v1:b3:{}", "11".repeat(32)))
                .expect("challenge ID"),

            // ron-auth deliberately verifies root control only. svc-passport
            // remains responsible for deriving Passport ID from the verified
            // public key before durable registry mutation.
            passport_id: PassportIdV1::parse(format!(
                "passport:v1:main:ed25519:b3:{}",
                "22".repeat(32)
            ))
            .expect("Passport ID"),

            root_public_key: root_public_key(&root_signing_key()),

            device_id: DeviceIdV1::parse(format!("device:v1:ed25519:b3:{}", "33".repeat(32)))
                .expect("Device ID"),

            operation_body_hash: B3DigestHex::parse("operation_body_hash", "44".repeat(32))
                .expect("operation body hash"),

            challenge_transcript_hash: B3DigestHex::parse(
                "challenge_transcript_hash",
                "55".repeat(32),
            )
            .expect("challenge transcript hash"),
        }
    }

    fn transcript<'a>(&'a self, scopes: &'a [&'a str]) -> RootRegistrationProofTranscriptV1<'a> {
        RootRegistrationProofTranscriptV1 {
            challenge_contract_domain: "native-passport/proof-challenge-contract/v1",
            challenge_contract_version: 1,
            proof_contract_domain: "native-passport/proof-contract/v1",
            proof_contract_version: 1,
            challenge_id: &self.challenge_id,
            network_id: "rustyonions-devnet",
            environment: "private-beta",
            audience: "svc-passport",
            passport_id: &self.passport_id,
            root_public_key: &self.root_public_key,
            root_key_epoch: 0,
            device_id: Some(&self.device_id),
            operation_body_hash: &self.operation_body_hash,
            challenge_transcript_hash: &self.challenge_transcript_hash,
            requested_scopes: scopes,
            challenge_issued_at_ms: 1_000_000,
            challenge_expires_at_ms: 1_300_000,
            proof_created_at_ms: 1_030_000,
        }
    }
}

#[test]
fn root_registration_exact_canonical_bytes_sign_and_verify() {
    let fixture = Fixture::new();
    let scopes = ["identity.read", "catalog.read"];

    let input = fixture.transcript(&scopes);
    let transcript =
        canonical_root_registration_proof_v1_transcript(&input).expect("canonical transcript");

    let signature = root_signing_key().sign(&transcript).to_bytes();

    verify_root_registration_proof_v1_strict(&input, &signature).expect("root proof should verify");

    let digest_one = root_registration_proof_v1_transcript_b3_hex(&input).expect("audit digest");

    let digest_two =
        root_registration_proof_v1_transcript_b3_hex(&input).expect("repeat audit digest");

    assert_eq!(digest_one, digest_two);
    assert_eq!(digest_one.len(), 64);

    assert_eq!(
        ROOT_REGISTRATION_PROOF_V1_TRANSCRIPT_DOMAIN,
        "rustyonions.native-passport.challenge-proof.v1"
    );

    assert_eq!(
        ROOT_REGISTRATION_PROOF_V1_CANONICAL_TRANSCRIPT_ENCODING,
        "length-prefixed-binary-v1"
    );
}

#[test]
fn root_registration_mutated_binding_rejects_original_signature() {
    let fixture = Fixture::new();
    let scopes = ["identity.read", "catalog.read"];

    let input = fixture.transcript(&scopes);

    let transcript =
        canonical_root_registration_proof_v1_transcript(&input).expect("canonical transcript");

    let signature = root_signing_key().sign(&transcript).to_bytes();

    let mut changed = input;
    changed.network_id = "rustyonions-other";

    assert_eq!(
        verify_root_registration_proof_v1_strict(&changed, &signature),
        Err(RootRegistrationProofError::InvalidSignature)
    );
}

#[test]
fn root_registration_wrong_concrete_root_key_rejects() {
    let fixture = Fixture::new();
    let scopes = ["identity.read", "catalog.read"];

    let input = fixture.transcript(&scopes);

    let transcript =
        canonical_root_registration_proof_v1_transcript(&input).expect("canonical transcript");

    let signature = root_signing_key().sign(&transcript).to_bytes();

    let other_root = root_public_key(&other_root_signing_key());

    let mut wrong_root_input = input;
    wrong_root_input.root_public_key = &other_root;

    assert_eq!(
        verify_root_registration_proof_v1_strict(&wrong_root_input, &signature,),
        Err(RootRegistrationProofError::InvalidSignature)
    );
}

#[test]
fn root_registration_scope_set_is_canonical_and_duplicate_fails_closed() {
    let fixture = Fixture::new();

    let scopes_one = ["identity.read", "catalog.read"];
    let scopes_two = ["catalog.read", "identity.read"];

    let one = fixture.transcript(&scopes_one);
    let two = fixture.transcript(&scopes_two);

    let transcript_one =
        canonical_root_registration_proof_v1_transcript(&one).expect("first transcript");

    let transcript_two =
        canonical_root_registration_proof_v1_transcript(&two).expect("second transcript");

    assert_eq!(transcript_one, transcript_two);

    let duplicate_scopes = ["identity.read", "identity.read"];
    let duplicate = fixture.transcript(&duplicate_scopes);

    assert_eq!(
        canonical_root_registration_proof_v1_transcript(&duplicate),
        Err(RootRegistrationProofError::DuplicateRequestedScope)
    );
}

#[test]
fn root_registration_pipe_and_json_bytes_are_never_accepted_as_canonical_proofs() {
    let fixture = Fixture::new();
    let scopes = ["identity.read", "catalog.read"];

    let input = fixture.transcript(&scopes);

    assert_eq!(
        ROOT_REGISTRATION_PROOF_V1_LEGACY_PHASE0_TRANSCRIPT_ENCODING,
        "pipe-delimited-canonical-v1"
    );

    let pipe_signature = root_signing_key()
        .sign(b"rustyonions.native-passport.challenge-proof.v1|challenge|register_root")
        .to_bytes();

    assert_eq!(
        verify_root_registration_proof_v1_strict(&input, &pipe_signature),
        Err(RootRegistrationProofError::InvalidSignature)
    );

    let json_signature = root_signing_key()
        .sign(br#"{"purpose":"register_root"}"#)
        .to_bytes();

    assert_eq!(
        verify_root_registration_proof_v1_strict(&input, &json_signature),
        Err(RootRegistrationProofError::InvalidSignature)
    );
}

fn root_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[0x71; 32])
}

fn other_root_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[0x72; 32])
}

fn root_public_key(signing_key: &SigningKey) -> Ed25519PublicKeyHex {
    Ed25519PublicKeyHex::parse(lower_hex(signing_key.verifying_key().as_bytes()))
        .expect("root public key")
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
