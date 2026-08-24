//! RO:WHAT — Physical M1 tests for recovery-factor-backed Native Passport RegisterRoot proof signing.
//! RO:WHY — The real client root proof must use existing recovery custody without exporting the mnemonic, BIP-39 seed, or generic root signing authority.
//! RO:INTERACTS — recovery_identity, purpose-specific root_registration_proof_signing, ron-auth canonical RegisterRoot transcript/strict verifier, and ron-proto public IDs.
//! RO:INVARIANTS — valid recovery factor signs and cross-verifies; identical input is deterministic; wrong Passport/root bindings reject; malformed recovery factor produces no proof evidence.
//! RO:METRICS — none.
//! RO:CONFIG — native-passport feature only.
//! RO:SECURITY — deterministic fixture recovery factors only; no physical vault, PIN, platform sealer, filesystem mutation, network, registry, replay, capability, username, wallet, or ledger mutation.
//! RO:TEST — this file.

#[cfg(not(feature = "native-passport"))]
#[test]
fn recovery_root_registration_signer_is_feature_gated() {
    assert!(!cfg!(feature = "native-passport"));
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use ron_auth::native_passport::{
        verify_root_registration_proof_v1_strict, RootRegistrationProofTranscriptV1,
    };

    use ron_proto::{
        B3DigestHex, ChallengeIdV1, Ed25519PublicKeyHex as ProtoEd25519PublicKeyHex,
        PassportIdV1 as ProtoPassportIdV1,
    };

    use svc_passport::native::{
        derive_native_recovery_public_identity_v1, sign_native_recovery_root_registration_proof_v1,
        NativeRecoveryRootRegistrationProofSigningError, NativeRootRegistrationProofSigningError,
        NativeSecretBytes, PHYSICAL_M1_RECOVERY_ROOT_REGISTRATION_PROOF_SIGNING_LABEL,
    };

    struct Fixture {
        recovery_factor: NativeSecretBytes,
        passport_id: ProtoPassportIdV1,
        root_public_key: ProtoEd25519PublicKeyHex,
        challenge_id: ChallengeIdV1,
        operation_body_hash: B3DigestHex,
        challenge_transcript_hash: B3DigestHex,
    }

    impl Fixture {
        fn new(byte: u8) -> Self {
            let recovery_factor =
                NativeSecretBytes::new(vec![byte; 32]).expect("nonphysical recovery factor");

            let identity = derive_native_recovery_public_identity_v1(&recovery_factor)
                .expect("recovery-derived public root");

            let passport_id =
                ProtoPassportIdV1::parse(identity.passport_id.as_str()).expect("proto Passport ID");

            let root_public_key =
                ProtoEd25519PublicKeyHex::parse(identity.root_public_key.as_str())
                    .expect("proto root public key");

            Self {
                recovery_factor,
                passport_id,
                root_public_key,

                challenge_id: ChallengeIdV1::parse(format!("challenge:v1:b3:{}", "11".repeat(32),))
                    .expect("challenge ID"),

                operation_body_hash: B3DigestHex::parse("operation_body_hash", "22".repeat(32))
                    .expect("operation body hash"),

                challenge_transcript_hash: B3DigestHex::parse(
                    "challenge_transcript_hash",
                    "33".repeat(32),
                )
                .expect("challenge transcript hash"),
            }
        }

        fn transcript<'a>(
            &'a self,
            scopes: &'a [&'a str],
        ) -> RootRegistrationProofTranscriptV1<'a> {
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

                device_id: None,

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
    fn recovery_factor_root_proof_cross_verifies_through_ron_auth() {
        let fixture = Fixture::new(0x51);
        let scopes = ["catalog.read", "identity.read"];

        let transcript = fixture.transcript(&scopes);

        let signed =
            sign_native_recovery_root_registration_proof_v1(&fixture.recovery_factor, &transcript)
                .expect("recovery-backed RegisterRoot proof");

        assert_eq!(
            PHYSICAL_M1_RECOVERY_ROOT_REGISTRATION_PROOF_SIGNING_LABEL,
            "PHYSICAL_M1_NATIVE_RECOVERY_ROOT_REGISTRATION_PROOF_SIGNING_V1",
        );

        assert_eq!(
            signed.root_identity.passport_id.as_str(),
            fixture.passport_id.as_str(),
        );

        assert_eq!(
            signed.root_identity.root_public_key.as_str(),
            fixture.root_public_key.as_str(),
        );

        let signature = decode_signature(signed.signed_payload_hex.as_str());

        verify_root_registration_proof_v1_strict(&transcript, &signature)
            .expect("strict ron-auth verification");
    }

    #[test]
    fn recovery_factor_root_proof_is_deterministic() {
        let first = Fixture::new(0x52);
        let scopes = ["catalog.read", "identity.read"];

        let first_transcript = first.transcript(&scopes);

        let first_signed = sign_native_recovery_root_registration_proof_v1(
            &first.recovery_factor,
            &first_transcript,
        )
        .expect("first proof");

        let second = Fixture::new(0x52);
        let second_transcript = second.transcript(&scopes);

        let second_signed = sign_native_recovery_root_registration_proof_v1(
            &second.recovery_factor,
            &second_transcript,
        )
        .expect("second proof");

        assert_eq!(
            first_signed.signed_payload_hex.as_str(),
            second_signed.signed_payload_hex.as_str(),
        );
    }

    #[test]
    fn recovery_factor_wrong_passport_binding_rejects() {
        let fixture = Fixture::new(0x53);
        let scopes = ["identity.read"];

        let wrong_passport =
            ProtoPassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{}", "aa".repeat(32),))
                .expect("wrong Passport ID");

        let mut transcript = fixture.transcript(&scopes);
        transcript.passport_id = &wrong_passport;

        assert!(matches!(
            sign_native_recovery_root_registration_proof_v1(&fixture.recovery_factor, &transcript,),
            Err(
                NativeRecoveryRootRegistrationProofSigningError::RootRegistrationProofSigning(
                    NativeRootRegistrationProofSigningError::PassportBindingMismatch
                )
            )
        ));
    }

    #[test]
    fn recovery_factor_wrong_root_public_key_rejects() {
        let fixture = Fixture::new(0x54);
        let other = Fixture::new(0x55);
        let scopes = ["identity.read"];

        let mut transcript = fixture.transcript(&scopes);

        transcript.root_public_key = &other.root_public_key;

        assert!(matches!(
            sign_native_recovery_root_registration_proof_v1(&fixture.recovery_factor, &transcript,),
            Err(
                NativeRecoveryRootRegistrationProofSigningError::RootRegistrationProofSigning(
                    NativeRootRegistrationProofSigningError::RootPublicKeyBindingMismatch
                )
            )
        ));
    }

    #[test]
    fn malformed_recovery_factor_produces_no_root_proof() {
        let fixture = Fixture::new(0x56);
        let scopes = ["identity.read"];
        let transcript = fixture.transcript(&scopes);

        let malformed =
            NativeSecretBytes::new(vec![0x57; 31]).expect("bounded malformed recovery factor");

        assert!(matches!(
            sign_native_recovery_root_registration_proof_v1(&malformed, &transcript,),
            Err(NativeRecoveryRootRegistrationProofSigningError::RecoveryFactorInvalid)
        ));
    }

    fn decode_signature(value: &str) -> [u8; 64] {
        assert_eq!(value.len(), 128);

        let bytes = value.as_bytes();
        let mut output = [0_u8; 64];

        for index in 0..64 {
            output[index] = (nibble(bytes[index * 2]) << 4) | nibble(bytes[index * 2 + 1]);
        }

        output
    }

    fn nibble(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("signature must be canonical lower hex"),
        }
    }
}
