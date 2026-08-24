//! RO:WHAT — Canonical Native Passport V1 root-registration proof transcript construction and strict Ed25519 verification.
//! RO:WHY — A server must prove control of the concrete Passport root public key before that key can become durable registry authority; Phase-10 contract labels alone are not cryptographic evidence.
//! RO:INTERACTS — ron-proto canonical Passport/Device/Challenge/B3/Ed25519 DTOs, the frozen challenge-proof domain, future svc-passport purpose-specific root proof signing, and the private server registry state machine.
//! RO:INVARIANTS — only deterministic length-prefixed binary V1 bytes are signable; the historical Phase-0 pipe transcript and ordinary JSON are never signable; verification covers the canonical bytes themselves; no Passport-ID derivation, key custody, persistence, network I/O, replay mutation, capability issuance, wallet, or ledger authority lives here.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — public-data transcript construction and public-key verification only; no private keys, recovery material, PINs, vault access, signing-key loading, or ambient authority.
//! RO:TEST — tests/native_passport_root_registration_proof.rs.

use ed25519_dalek::{Signature, VerifyingKey};
use ron_proto::{B3DigestHex, ChallengeIdV1, DeviceIdV1, Ed25519PublicKeyHex, PassportIdV1};
use thiserror::Error;

/// Frozen Native Passport domain for challenge proofs.
pub const ROOT_REGISTRATION_PROOF_V1_TRANSCRIPT_DOMAIN: &str =
    "rustyonions.native-passport.challenge-proof.v1";

/// Canonical encoding used by the real V1 root-registration proof.
pub const ROOT_REGISTRATION_PROOF_V1_CANONICAL_TRANSCRIPT_ENCODING: &str =
    "length-prefixed-binary-v1";

/// Historical Phase-0 challenge-proof encoding retained only as unsigned evidence.
pub const ROOT_REGISTRATION_PROOF_V1_LEGACY_PHASE0_TRANSCRIPT_ENCODING: &str =
    "pipe-delimited-canonical-v1";

/// Historical Phase-0 pipe bytes must never be signed as a real proof.
pub const ROOT_REGISTRATION_PROOF_V1_LEGACY_PHASE0_TRANSCRIPT_SIGNABLE: bool = false;

/// Ordinary JSON bytes must never be signed as a root-registration proof.
pub const ROOT_REGISTRATION_PROOF_V1_JSON_TRANSCRIPT_SIGNABLE: bool = false;

const ROOT_REGISTRATION_PROOF_V1_TRANSCRIPT_VERSION: u16 = 1;
const ROOT_REGISTRATION_PROOF_V1_KIND: &str = "root-registration";
const MAX_TRANSCRIPT_TEXT_BYTES: usize = 256;
const MAX_ROOT_REGISTRATION_SCOPES: usize = 32;
const MAX_ROOT_REGISTRATION_SCOPE_BYTES: usize = 96;

/// Public inputs covered by one Native Passport V1 root-registration proof.
///
/// `root_public_key` is deliberately concrete authority-bearing key material,
/// not the Phase-10 descriptive signer label. `passport_id` remains a typed
/// input here; svc-passport remains responsible for deriving and enforcing the
/// Passport-ID/root-public-key relationship before durable registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootRegistrationProofTranscriptV1<'a> {
    pub challenge_contract_domain: &'a str,
    pub challenge_contract_version: u16,
    pub proof_contract_domain: &'a str,
    pub proof_contract_version: u16,
    pub challenge_id: &'a ChallengeIdV1,
    pub network_id: &'a str,
    pub environment: &'a str,
    pub audience: &'a str,
    pub passport_id: &'a PassportIdV1,
    pub root_public_key: &'a Ed25519PublicKeyHex,
    pub root_key_epoch: u64,
    pub device_id: Option<&'a DeviceIdV1>,
    pub operation_body_hash: &'a B3DigestHex,
    pub challenge_transcript_hash: &'a B3DigestHex,
    pub requested_scopes: &'a [&'a str],
    pub challenge_issued_at_ms: u64,
    pub challenge_expires_at_ms: u64,
    pub proof_created_at_ms: u64,
}

/// Canonical transcript construction or strict verification failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum RootRegistrationProofError {
    #[error("root-registration proof field {0} is empty")]
    EmptyField(&'static str),

    #[error("root-registration proof field {0} exceeds its bound")]
    FieldTooLong(&'static str),

    #[error("root-registration proof field {0} contains unsupported text")]
    InvalidTextField(&'static str),

    #[error("root-registration proof requested scope set is empty")]
    EmptyRequestedScopes,

    #[error("root-registration proof requested scope count exceeds its bound")]
    TooManyRequestedScopes,

    #[error("root-registration proof requested scope is invalid")]
    InvalidRequestedScope,

    #[error("root-registration proof requested scopes contain a duplicate")]
    DuplicateRequestedScope,

    #[error("root-registration proof challenge time window is invalid")]
    InvalidChallengeTimeWindow,

    #[error("root-registration proof creation time is outside the challenge window")]
    ProofCreatedOutsideChallengeWindow,

    #[error("root-registration proof root public key encoding is invalid")]
    InvalidRootPublicKey,

    #[error("root-registration proof digest field {0} is invalid")]
    InvalidDigest(&'static str),

    #[error("root-registration proof root public key is not a valid Ed25519 key")]
    InvalidEd25519RootPublicKey,

    #[error("root-registration proof Ed25519 signature mismatch")]
    InvalidSignature,
}

/// Construct the exact bytes covered by a root-registration Ed25519 signature.
///
/// V1 layout:
///
/// ```text
/// lp(domain)
/// u16_be(transcript_version)
/// lp(proof_kind)
/// lp(challenge_contract_domain)
/// u16_be(challenge_contract_version)
/// lp(proof_contract_domain)
/// u16_be(proof_contract_version)
/// lp(challenge_id)
/// lp(network_id)
/// lp(environment)
/// lp(audience)
/// lp(passport_id)
/// lp(raw_32_byte_root_public_key)
/// u64_be(root_key_epoch)
/// u8(device_id_present)
/// [lp(device_id)]
/// lp(raw_32_byte_operation_body_hash)
/// lp(raw_32_byte_challenge_transcript_hash)
/// u16_be(scope_count)
/// repeated lp(scope), lexicographically canonicalized
/// u64_be(challenge_issued_at_ms)
/// u64_be(challenge_expires_at_ms)
/// u64_be(proof_created_at_ms)
/// ```
///
/// `lp(x)` means `u16_be(len(x)) || x`.
pub fn canonical_root_registration_proof_v1_transcript(
    input: &RootRegistrationProofTranscriptV1<'_>,
) -> Result<Vec<u8>, RootRegistrationProofError> {
    validate_input(input)?;

    let root_public_key = decode_lower_hex_32(
        "root_public_key",
        input.root_public_key.as_str(),
        RootRegistrationProofError::InvalidRootPublicKey,
    )?;

    let operation_body_hash = decode_lower_hex_32(
        "operation_body_hash",
        input.operation_body_hash.as_str(),
        RootRegistrationProofError::InvalidDigest("operation_body_hash"),
    )?;

    let challenge_transcript_hash = decode_lower_hex_32(
        "challenge_transcript_hash",
        input.challenge_transcript_hash.as_str(),
        RootRegistrationProofError::InvalidDigest("challenge_transcript_hash"),
    )?;

    let scopes = canonical_scopes(input.requested_scopes)?;

    let scope_count = u16::try_from(scopes.len())
        .map_err(|_| RootRegistrationProofError::TooManyRequestedScopes)?;

    let mut transcript = Vec::with_capacity(768);

    push_len_prefixed(
        &mut transcript,
        ROOT_REGISTRATION_PROOF_V1_TRANSCRIPT_DOMAIN.as_bytes(),
    )?;

    transcript.extend_from_slice(&ROOT_REGISTRATION_PROOF_V1_TRANSCRIPT_VERSION.to_be_bytes());

    push_len_prefixed(&mut transcript, ROOT_REGISTRATION_PROOF_V1_KIND.as_bytes())?;

    push_len_prefixed(&mut transcript, input.challenge_contract_domain.as_bytes())?;

    transcript.extend_from_slice(&input.challenge_contract_version.to_be_bytes());

    push_len_prefixed(&mut transcript, input.proof_contract_domain.as_bytes())?;

    transcript.extend_from_slice(&input.proof_contract_version.to_be_bytes());

    push_len_prefixed(&mut transcript, input.challenge_id.as_str().as_bytes())?;
    push_len_prefixed(&mut transcript, input.network_id.as_bytes())?;
    push_len_prefixed(&mut transcript, input.environment.as_bytes())?;
    push_len_prefixed(&mut transcript, input.audience.as_bytes())?;
    push_len_prefixed(&mut transcript, input.passport_id.as_str().as_bytes())?;
    push_len_prefixed(&mut transcript, &root_public_key)?;

    transcript.extend_from_slice(&input.root_key_epoch.to_be_bytes());

    match input.device_id {
        None => transcript.push(0),
        Some(device_id) => {
            transcript.push(1);
            push_len_prefixed(&mut transcript, device_id.as_str().as_bytes())?;
        }
    }

    push_len_prefixed(&mut transcript, &operation_body_hash)?;
    push_len_prefixed(&mut transcript, &challenge_transcript_hash)?;

    transcript.extend_from_slice(&scope_count.to_be_bytes());

    for scope in scopes {
        push_len_prefixed(&mut transcript, scope.as_bytes())?;
    }

    transcript.extend_from_slice(&input.challenge_issued_at_ms.to_be_bytes());
    transcript.extend_from_slice(&input.challenge_expires_at_ms.to_be_bytes());
    transcript.extend_from_slice(&input.proof_created_at_ms.to_be_bytes());

    Ok(transcript)
}

/// BLAKE3 audit digest of the canonical root-registration proof transcript.
///
/// Ed25519 signs and verifies the transcript bytes themselves. This digest is
/// deterministic evidence for interop and later proof-envelope binding only.
pub fn root_registration_proof_v1_transcript_b3_hex(
    input: &RootRegistrationProofTranscriptV1<'_>,
) -> Result<String, RootRegistrationProofError> {
    let transcript = canonical_root_registration_proof_v1_transcript(input)?;
    Ok(blake3::hash(&transcript).to_hex().to_string())
}

/// Strictly verify one root-registration proof against its concrete root key.
///
/// Successful verification proves control of `input.root_public_key` over the
/// exact canonical registration transcript. It does not itself make that key
/// durable authority; svc-passport must still enforce Passport-ID/key binding,
/// challenge/replay state, registry conflict rules, and persistence.
pub fn verify_root_registration_proof_v1_strict(
    input: &RootRegistrationProofTranscriptV1<'_>,
    signature_bytes: &[u8; 64],
) -> Result<(), RootRegistrationProofError> {
    let transcript = canonical_root_registration_proof_v1_transcript(input)?;

    let root_public_key = decode_lower_hex_32(
        "root_public_key",
        input.root_public_key.as_str(),
        RootRegistrationProofError::InvalidRootPublicKey,
    )?;

    let verifying_key = VerifyingKey::from_bytes(&root_public_key)
        .map_err(|_| RootRegistrationProofError::InvalidEd25519RootPublicKey)?;

    let signature = Signature::from_bytes(signature_bytes);

    verifying_key
        .verify_strict(&transcript, &signature)
        .map_err(|_| RootRegistrationProofError::InvalidSignature)
}

fn validate_input(
    input: &RootRegistrationProofTranscriptV1<'_>,
) -> Result<(), RootRegistrationProofError> {
    validate_text("challenge_contract_domain", input.challenge_contract_domain)?;

    validate_text("proof_contract_domain", input.proof_contract_domain)?;
    validate_text("network_id", input.network_id)?;
    validate_text("environment", input.environment)?;
    validate_text("audience", input.audience)?;

    if input.challenge_issued_at_ms == 0
        || input.challenge_expires_at_ms <= input.challenge_issued_at_ms
    {
        return Err(RootRegistrationProofError::InvalidChallengeTimeWindow);
    }

    if input.proof_created_at_ms < input.challenge_issued_at_ms
        || input.proof_created_at_ms > input.challenge_expires_at_ms
    {
        return Err(RootRegistrationProofError::ProofCreatedOutsideChallengeWindow);
    }

    canonical_scopes(input.requested_scopes)?;

    Ok(())
}

fn validate_text(field: &'static str, value: &str) -> Result<(), RootRegistrationProofError> {
    if value.is_empty() {
        return Err(RootRegistrationProofError::EmptyField(field));
    }

    if value.len() > MAX_TRANSCRIPT_TEXT_BYTES {
        return Err(RootRegistrationProofError::FieldTooLong(field));
    }

    if value.trim() != value
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte >= 0x80)
    {
        return Err(RootRegistrationProofError::InvalidTextField(field));
    }

    Ok(())
}

fn canonical_scopes<'a>(scopes: &'a [&'a str]) -> Result<Vec<&'a str>, RootRegistrationProofError> {
    if scopes.is_empty() {
        return Err(RootRegistrationProofError::EmptyRequestedScopes);
    }

    if scopes.len() > MAX_ROOT_REGISTRATION_SCOPES {
        return Err(RootRegistrationProofError::TooManyRequestedScopes);
    }

    let mut canonical = Vec::with_capacity(scopes.len());

    for scope in scopes {
        if scope.is_empty()
            || scope.len() > MAX_ROOT_REGISTRATION_SCOPE_BYTES
            || scope.bytes().any(|byte| {
                byte.is_ascii_uppercase()
                    || !(byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'.' | b'_' | b'-' | b':'))
            })
        {
            return Err(RootRegistrationProofError::InvalidRequestedScope);
        }

        canonical.push(*scope);
    }

    canonical.sort_unstable();

    if canonical.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(RootRegistrationProofError::DuplicateRequestedScope);
    }

    Ok(canonical)
}

fn push_len_prefixed(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), RootRegistrationProofError> {
    let length = u16::try_from(bytes.len())
        .map_err(|_| RootRegistrationProofError::FieldTooLong("length_prefixed_field"))?;

    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);

    Ok(())
}

fn decode_lower_hex_32(
    _field: &'static str,
    value: &str,
    error: RootRegistrationProofError,
) -> Result<[u8; 32], RootRegistrationProofError> {
    if value.len() != 64 {
        return Err(error);
    }

    let source = value.as_bytes();
    let mut output = [0_u8; 32];

    for index in 0..32 {
        let high = lower_hex_nibble(source[index * 2]).ok_or_else(|| error.clone())?;
        let low = lower_hex_nibble(source[index * 2 + 1]).ok_or_else(|| error.clone())?;

        output[index] = (high << 4) | low;
    }

    Ok(output)
}

fn lower_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
