//! RO:WHAT — Purpose-specific native DeviceKey signing for canonical DeviceSessionProofV1 transcripts.
//! RO:WHY — CN-4 physical possession must prove control of the already-authorized operational DeviceKey without exporting the device seed or creating a generic signing capability.
//! RO:INTERACTS — Native device public-identity derivation, NativeSecretBytes, ron-auth canonical DeviceSessionProof transcript/signature verification, and ron-proto public proof DTOs.
//! RO:INVARIANTS — seed is exactly 32 bytes; the supplied transcript Device ID and public key must derive from that seed; only canonical ron-auth transcript bytes are signed; the produced signature self-verifies strictly before return.
//! RO:METRICS — none; pure native signing primitive.
//! RO:CONFIG — compiled only through the native-passport feature.
//! RO:SECURITY — no generic-message signer, seed export, serialization, logging, filesystem, Keychain, HTTP, RecoveryRoot, root PIN, capability issuance, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — focused module tests below prove strict cross-verification and device-binding rejection.

#![forbid(unsafe_code)]

use ed25519_dalek::{Signer as _, SigningKey};
use ron_auth::native_passport::{
    canonical_device_session_proof_v1_transcript, verify_device_session_proof_v1_strict,
    DeviceSessionProofTranscriptV1,
};
use ron_proto::Ed25519SignatureV1;
use zeroize::Zeroizing;

use super::{
    derive_native_device_public_identity_v1, NativeSecretBytes, DEVICE_ID_V1_SIGNING_SEED_BYTES,
};

pub const PHYSICAL_M1_DEVICE_SESSION_PROOF_SIGNING_LABEL: &str =
    "PHYSICAL_M1_NATIVE_DEVICE_SESSION_PROOF_SIGNING_V1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDeviceSessionProofSigningError {
    InvalidDeviceSigningSeedLength { actual: usize, expected: usize },
    DeviceIdentityDerivationFailed,
    DeviceBindingMismatch,
    TranscriptRejected,
    ProducedSignatureRejected,
}

/// Sign exactly one canonical DeviceSessionProofV1 transcript with the
/// operational DeviceKey.
///
/// The caller supplies a fully bound canonical transcript. This primitive
/// independently derives the public device identity from the secret seed and
/// refuses to sign if either the Device ID or public key differs.
///
/// Only the public signature leaves this function.
pub fn sign_native_device_session_proof_v1(
    device_signing_seed: &NativeSecretBytes,
    transcript: &DeviceSessionProofTranscriptV1<'_>,
) -> Result<Ed25519SignatureV1, NativeDeviceSessionProofSigningError> {
    if device_signing_seed.len() != DEVICE_ID_V1_SIGNING_SEED_BYTES {
        return Err(
            NativeDeviceSessionProofSigningError::InvalidDeviceSigningSeedLength {
                actual: device_signing_seed.len(),
                expected: DEVICE_ID_V1_SIGNING_SEED_BYTES,
            },
        );
    }

    let native_identity = derive_native_device_public_identity_v1(device_signing_seed)
        .map_err(|_| NativeDeviceSessionProofSigningError::DeviceIdentityDerivationFailed)?;

    if native_identity.device_id.as_str() != transcript.device_id.as_str()
        || native_identity.device_public_key.as_str() != transcript.device_public_key.as_str()
    {
        return Err(NativeDeviceSessionProofSigningError::DeviceBindingMismatch);
    }

    let canonical = canonical_device_session_proof_v1_transcript(transcript)
        .map_err(|_| NativeDeviceSessionProofSigningError::TranscriptRejected)?;

    let mut seed = Zeroizing::new([0u8; DEVICE_ID_V1_SIGNING_SEED_BYTES]);
    seed.copy_from_slice(device_signing_seed.as_slice());

    let signing_key = SigningKey::from_bytes(&seed);

    let signature = Ed25519SignatureV1::from_bytes(signing_key.sign(&canonical).to_bytes());

    verify_device_session_proof_v1_strict(transcript, &signature)
        .map_err(|_| NativeDeviceSessionProofSigningError::ProducedSignatureRejected)?;

    Ok(signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    use ron_auth::native_passport::verify_device_session_proof_v1_strict;
    use ron_proto::{
        B3DigestHex, ChallengeIdV1, DeviceIdV1 as ProtoDeviceIdV1,
        Ed25519PublicKeyHex as ProtoEd25519PublicKeyHex, NativePassportContextLabelV1,
        NativePassportScopeV1, PassportIdV1,
    };

    use super::super::{
        PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN, PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
        PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
    };

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    const CHALLENGE_ISSUED_AT_MS: u64 = 1_720_000_000_000;
    const CHALLENGE_EXPIRES_AT_MS: u64 = 1_720_000_060_000;
    const PROOF_CREATED_AT_MS: u64 = 1_720_000_001_000;

    struct Fixture {
        challenge_id: ChallengeIdV1,
        network_id: NativePassportContextLabelV1,
        environment: NativePassportContextLabelV1,
        audience: NativePassportContextLabelV1,
        passport_id: PassportIdV1,
        device_id: ProtoDeviceIdV1,
        device_public_key: ProtoEd25519PublicKeyHex,
        challenge_transcript_hash: B3DigestHex,
        requested_scopes: Vec<NativePassportScopeV1>,
    }

    impl Fixture {
        fn from_seed(device_seed: &NativeSecretBytes) -> Self {
            let native_identity =
                derive_native_device_public_identity_v1(device_seed).expect("device identity");

            Self {
                challenge_id: ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_A}"))
                    .expect("challenge ID"),
                network_id: NativePassportContextLabelV1::parse("rustyonions-devnet")
                    .expect("network"),
                environment: NativePassportContextLabelV1::parse("private-beta")
                    .expect("environment"),
                audience: NativePassportContextLabelV1::parse("svc-passport").expect("audience"),
                passport_id: PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_B}"))
                    .expect("Passport ID"),
                device_id: ProtoDeviceIdV1::parse(native_identity.device_id.as_str())
                    .expect("proto device ID"),
                device_public_key: ProtoEd25519PublicKeyHex::parse(
                    native_identity.device_public_key.as_str(),
                )
                .expect("proto device public key"),
                challenge_transcript_hash: B3DigestHex::parse("challenge_transcript_hash", HEX_C)
                    .expect("challenge hash"),
                requested_scopes: vec![
                    NativePassportScopeV1::parse("identity.read").expect("scope")
                ],
            }
        }

        fn transcript(&self) -> DeviceSessionProofTranscriptV1<'_> {
            DeviceSessionProofTranscriptV1 {
                challenge_contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
                challenge_contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
                proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
                proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
                challenge_id: &self.challenge_id,
                network_id: &self.network_id,
                environment: &self.environment,
                audience: &self.audience,
                passport_id: &self.passport_id,
                device_id: &self.device_id,
                device_public_key: &self.device_public_key,
                challenge_transcript_hash: &self.challenge_transcript_hash,
                requested_scopes: &self.requested_scopes,
                challenge_issued_at_ms: CHALLENGE_ISSUED_AT_MS,
                challenge_expires_at_ms: CHALLENGE_EXPIRES_AT_MS,
                proof_created_at_ms: PROOF_CREATED_AT_MS,
            }
        }
    }

    fn device_seed(byte: u8) -> NativeSecretBytes {
        NativeSecretBytes::new(vec![byte; DEVICE_ID_V1_SIGNING_SEED_BYTES]).expect("device seed")
    }

    #[test]
    fn device_session_signature_cross_verifies_through_ron_auth() {
        assert_eq!(
            PHYSICAL_M1_DEVICE_SESSION_PROOF_SIGNING_LABEL,
            "PHYSICAL_M1_NATIVE_DEVICE_SESSION_PROOF_SIGNING_V1",
        );

        let seed = device_seed(0x42);
        let fixture = Fixture::from_seed(&seed);
        let transcript = fixture.transcript();

        let signature =
            sign_native_device_session_proof_v1(&seed, &transcript).expect("device signature");

        verify_device_session_proof_v1_strict(&transcript, &signature)
            .expect("strict cross-verification");
    }

    #[test]
    fn device_session_signer_rejects_transcript_bound_to_another_device() {
        let seed = device_seed(0x42);
        let fixture = Fixture::from_seed(&seed);

        let other_seed = device_seed(0x55);
        let other_fixture = Fixture::from_seed(&other_seed);

        let mut transcript = fixture.transcript();
        transcript.device_id = &other_fixture.device_id;
        transcript.device_public_key = &other_fixture.device_public_key;

        assert_eq!(
            sign_native_device_session_proof_v1(&seed, &transcript),
            Err(NativeDeviceSessionProofSigningError::DeviceBindingMismatch),
        );
    }
}
