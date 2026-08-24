//! RO:WHAT — Physical M1 tests for the real purpose-specific Native Passport root-registration proof signer.
//! RO:WHY — Server root registration must eventually consume a real root signature bound to the concrete derived Passport root, never synthetic accepted evidence.
//! RO:INTERACTS — svc-passport root identity custody/signer, ron-auth canonical RegisterRoot transcript/strict verifier, and NativeProofSignedPayloadHex.
//! RO:INVARIANTS — exact root/passport binding signs and cross-verifies; wrong Passport/root reject before usable evidence; malformed seed rejects; identical inputs sign deterministically.
//! RO:METRICS — none.
//! RO:CONFIG — native-passport feature only.
//! RO:SECURITY — deterministic nonphysical test secrets only; no vault, PIN, recovery phrase, filesystem authority, route, replay, capability, username, wallet, or ledger mutation.
//! RO:TEST — this file.

#[cfg(not(feature = "native-passport"))]
#[test]
fn root_registration_signer_is_feature_gated_in_default_build() {
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
        derive_native_root_public_identity_v1, sign_native_root_registration_proof_v1,
        NativeRootRegistrationProofSigningError, NativeSecretBytes,
        PHYSICAL_M1_ROOT_REGISTRATION_PROOF_SIGNING_LABEL,
    };

    struct RootFixture {
        seed: NativeSecretBytes,
        passport_id: ProtoPassportIdV1,
        root_public_key: ProtoEd25519PublicKeyHex,
        challenge_id: ChallengeIdV1,
        operation_body_hash: B3DigestHex,
        challenge_transcript_hash: B3DigestHex,
    }

    impl RootFixture {
        fn new(seed_byte: u8) -> Self {
            let seed =
                NativeSecretBytes::new(vec![seed_byte; 64]).expect("nonphysical BIP-39 seed");

            let identity =
                derive_native_root_public_identity_v1(&seed).expect("derived root identity");

            let passport_id =
                ProtoPassportIdV1::parse(identity.passport_id.as_str()).expect("proto Passport ID");

            let root_public_key =
                ProtoEd25519PublicKeyHex::parse(identity.root_public_key.as_str())
                    .expect("proto root public key");

            let challenge_id =
                ChallengeIdV1::parse(format!("challenge:v1:b3:{}", "11".repeat(32),))
                    .expect("challenge ID");

            let operation_body_hash = B3DigestHex::parse("operation_body_hash", "22".repeat(32))
                .expect("operation body hash");

            let challenge_transcript_hash =
                B3DigestHex::parse("challenge_transcript_hash", "33".repeat(32))
                    .expect("challenge transcript hash");

            Self {
                seed,
                passport_id,
                root_public_key,
                challenge_id,
                operation_body_hash,
                challenge_transcript_hash,
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
    fn real_root_registration_signer_cross_verifies_through_ron_auth() {
        let fixture = RootFixture::new(0x41);

        let scopes = ["catalog.read", "identity.read"];

        let transcript = fixture.transcript(&scopes);

        let signed = sign_native_root_registration_proof_v1(&fixture.seed, &transcript)
            .expect("root-registration proof signing");

        assert_eq!(
            PHYSICAL_M1_ROOT_REGISTRATION_PROOF_SIGNING_LABEL,
            "PHYSICAL_M1_NATIVE_ROOT_REGISTRATION_PROOF_SIGNING_V1"
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
            .expect("strict ron-auth cross-verification");
    }

    #[test]
    fn identical_root_registration_inputs_sign_deterministically() {
        let fixture = RootFixture::new(0x42);

        let scopes = ["catalog.read", "identity.read"];

        let transcript = fixture.transcript(&scopes);

        let first = sign_native_root_registration_proof_v1(&fixture.seed, &transcript)
            .expect("first signature");

        let second = sign_native_root_registration_proof_v1(&fixture.seed, &transcript)
            .expect("second signature");

        assert_eq!(
            first.signed_payload_hex.as_str(),
            second.signed_payload_hex.as_str(),
        );
    }

    #[test]
    fn wrong_passport_binding_rejects_before_signing_evidence() {
        let fixture = RootFixture::new(0x43);

        let scopes = ["identity.read"];

        let wrong_passport =
            ProtoPassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{}", "aa".repeat(32),))
                .expect("wrong Passport ID");

        let mut transcript = fixture.transcript(&scopes);

        transcript.passport_id = &wrong_passport;

        assert!(matches!(
            sign_native_root_registration_proof_v1(&fixture.seed, &transcript,),
            Err(NativeRootRegistrationProofSigningError::PassportBindingMismatch),
        ));
    }

    #[test]
    fn wrong_root_public_key_rejects_before_signing_evidence() {
        let fixture = RootFixture::new(0x44);
        let other = RootFixture::new(0x45);

        let scopes = ["identity.read"];

        let mut transcript = fixture.transcript(&scopes);

        transcript.root_public_key = &other.root_public_key;

        assert!(matches!(
            sign_native_root_registration_proof_v1(&fixture.seed, &transcript,),
            Err(NativeRootRegistrationProofSigningError::RootPublicKeyBindingMismatch),
        ));
    }

    #[test]
    fn malformed_root_seed_rejects_without_signed_payload() {
        let fixture = RootFixture::new(0x46);

        let malformed =
            NativeSecretBytes::new(vec![0x47; 31]).expect("bounded malformed seed fixture");

        let scopes = ["identity.read"];

        let transcript = fixture.transcript(&scopes);

        assert!(matches!(
            sign_native_root_registration_proof_v1(&malformed, &transcript,),
            Err(NativeRootRegistrationProofSigningError::RootIdentityDerivationFailed),
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
