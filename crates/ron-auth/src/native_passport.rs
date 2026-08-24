//! RO:WHAT — Native Passport authentication transcript ownership boundary for ron-auth.
//! RO:WHY — Physical M1 must explicitly supersede the unsigned Phase-0 pipe-delimited authorization digest before any real Passport-root signature is created.
//! RO:INTERACTS — canonical DeviceAuthorizationV1 transcript construction, strict Ed25519 verification, frozen Passport/Device ID binding, and preserved Phase-0 svc-passport evidence.
//! RO:INVARIANTS — canonical V1 signing uses deterministic length-prefixed binary bytes; ordinary JSON and Phase-0 pipe-delimited text are never root-signing inputs.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — pure public-data transcript and verification only; no private-key loading, signing, persistence, HTTP, capability issuance, wallet mutation, or ledger mutation.
//! RO:TEST — tests/physical_m1_device_authorization_transcript_posture.rs.

/// Domain bound into canonical Native Passport V1 device authorization.
pub const DEVICE_AUTHORIZATION_V1_TRANSCRIPT_DOMAIN: &str =
    "rustyonions.native-passport.device-authorization.v1";

/// Canonical encoding selected for the real V1 root-signing transcript.
pub const DEVICE_AUTHORIZATION_V1_CANONICAL_TRANSCRIPT_ENCODING: &str = "length-prefixed-binary-v1";

/// Historical Phase-0 fixture encoding retained only as unsigned evidence.
pub const DEVICE_AUTHORIZATION_V1_LEGACY_PHASE0_TRANSCRIPT_ENCODING: &str =
    "pipe-delimited-canonical-v1";

/// The historical Phase-0 pipe transcript must never be signed by a Passport root.
pub const DEVICE_AUTHORIZATION_V1_LEGACY_PHASE0_TRANSCRIPT_SIGNABLE: bool = false;

/// Ordinary serialized JSON must never be signed as DeviceAuthorizationV1.
pub const DEVICE_AUTHORIZATION_V1_JSON_TRANSCRIPT_SIGNABLE: bool = false;

pub mod device_authorization;
pub use device_authorization::{
    canonical_device_authorization_v1_transcript, device_authorization_v1_transcript_b3_hex,
    DeviceAuthorizationTranscriptError,
};

pub mod device_authorization_verify;
pub use device_authorization_verify::{
    verify_device_authorization_v1_strict, DeviceAuthorizationVerificationContextV1,
    DeviceAuthorizationVerificationError,
};

pub mod challenge;
pub use challenge::{
    canonical_passport_challenge_v1_transcript, passport_challenge_v1_transcript_b3_hex,
    verify_passport_challenge_v1_strict, PassportChallengeTranscriptError,
    PassportChallengeVerificationContextV1, PassportChallengeVerificationError,
    PASSPORT_CHALLENGE_V1_CANONICAL_TRANSCRIPT_ENCODING,
    PASSPORT_CHALLENGE_V1_JSON_TRANSCRIPT_SIGNABLE, PASSPORT_CHALLENGE_V1_TRANSCRIPT_DOMAIN,
};

pub mod device_session_proof;
pub use device_session_proof::{
    canonical_device_session_proof_v1_transcript, device_session_proof_v1_transcript_b3_hex,
    verify_device_session_proof_v1_strict, DeviceSessionProofError, DeviceSessionProofTranscriptV1,
    DEVICE_SESSION_PROOF_V1_CANONICAL_TRANSCRIPT_ENCODING,
    DEVICE_SESSION_PROOF_V1_JSON_TRANSCRIPT_SIGNABLE,
    DEVICE_SESSION_PROOF_V1_LEGACY_PHASE0_TRANSCRIPT_SIGNABLE,
    DEVICE_SESSION_PROOF_V1_TRANSCRIPT_DOMAIN,
};

pub mod root_registration_proof;
pub use root_registration_proof::{
    canonical_root_registration_proof_v1_transcript, root_registration_proof_v1_transcript_b3_hex,
    verify_root_registration_proof_v1_strict, RootRegistrationProofError,
    RootRegistrationProofTranscriptV1, ROOT_REGISTRATION_PROOF_V1_CANONICAL_TRANSCRIPT_ENCODING,
    ROOT_REGISTRATION_PROOF_V1_JSON_TRANSCRIPT_SIGNABLE,
    ROOT_REGISTRATION_PROOF_V1_LEGACY_PHASE0_TRANSCRIPT_ENCODING,
    ROOT_REGISTRATION_PROOF_V1_LEGACY_PHASE0_TRANSCRIPT_SIGNABLE,
    ROOT_REGISTRATION_PROOF_V1_TRANSCRIPT_DOMAIN,
};
