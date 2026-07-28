#[cfg(not(feature = "native-passport"))]
#[test]
fn phase8b_proof_contract_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native proof contract DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_proof_contract_posture, review_native_passport_challenge_contract_draft,
        review_native_passport_proof_contract_draft,
        validate_native_passport_proof_contract_descriptor, B3DigestHex, ChallengeIdV1, DeviceIdV1,
        NativeChallengeTranscriptCodec, NativePassportChallengeContractDescriptorV1,
        NativePassportChallengeContractDraftV1, NativePassportChallengePurpose,
        NativePassportProofAuthority, NativePassportProofContractDraftV1,
        NativePassportProofContractReviewError, NativePassportProofKind, NativePassportScope,
        NativePassportSurface, NativeProofSignatureAlgorithm, PassportIdV1,
        NATIVE_PASSPORT_PHASE8B_LABEL, PHASE8A_ENABLED_SURFACES,
        PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN, PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
        PHASE8A_REQUIRED_SERVICE_SIGNATURE_ALGORITHM, PHASE8A_REQUIRED_TRANSCRIPT_CODEC,
        PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_ENABLED_SURFACES,
        PHASE8B_FORBIDDEN_PROOF_AUTHORITY_FLAGS, PHASE8B_PROOF_CONTRACT_DOMAIN,
        PHASE8B_PROOF_CONTRACT_VERSION, PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC,
        PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
    };

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const HEX_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn challenge_id() -> ChallengeIdV1 {
        ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_A}")).expect("challenge id")
    }

    fn other_challenge_id() -> ChallengeIdV1 {
        ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_F}")).expect("other challenge id")
    }

    fn passport_id() -> PassportIdV1 {
        PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_B}")).expect("passport id")
    }

    fn device_id() -> DeviceIdV1 {
        DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}")).expect("device id")
    }

    fn body_hash() -> B3DigestHex {
        B3DigestHex::parse("operation_body_hash", HEX_D).expect("operation body hash")
    }

    fn nonce() -> B3DigestHex {
        B3DigestHex::parse("challenge_nonce_hex", HEX_A).expect("nonce")
    }

    fn challenge_transcript_hash() -> B3DigestHex {
        B3DigestHex::parse("challenge_transcript_hash", HEX_E).expect("challenge transcript hash")
    }

    fn proof_transcript_hash() -> B3DigestHex {
        B3DigestHex::parse("proof_transcript_hash", HEX_F).expect("proof transcript hash")
    }

    fn challenge_draft(
        purpose: NativePassportChallengePurpose,
    ) -> NativePassportChallengeContractDraftV1 {
        NativePassportChallengeContractDraftV1 {
            contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
            contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
            challenge_id: challenge_id(),
            network_id: "rustyonions-main",
            environment: "private-beta",
            audience: "svc-passport",
            issuing_service_id: "svc-passport",
            service_key_id: "service-key:v1:challenge",
            purpose,
            requested_scopes: vec![
                NativePassportScope::IdentityRead,
                NativePassportScope::CatalogRead,
            ],
            passport_id: Some(passport_id()),
            device_id: Some(device_id()),
            operation_body_hash: Some(body_hash()),
            nonce_hex: nonce(),
            issued_at_ms: 1_000_000,
            expires_at_ms: 1_060_000,
            max_clock_skew_ms: 10_000,
            transcript_codec: PHASE8A_REQUIRED_TRANSCRIPT_CODEC,
            service_signature_algorithm: PHASE8A_REQUIRED_SERVICE_SIGNATURE_ALGORITHM,
            service_signature_placeholder_present: true,
            requests_service_challenge_signing: false,
            requests_service_challenge_verification: false,
            requests_root_proof_signing: false,
            requests_device_proof_signing: false,
            requests_proof_signature_verification: false,
            requests_request_proof_signing: false,
            requests_request_proof_verification: false,
            requests_replay_store_mutation: false,
            requests_challenge_consumption: false,
            requests_capability_issuance: false,
            requests_capability_lifecycle_runtime: false,
            requests_device_authority_execution: false,
            requests_namespace_execution: false,
            requests_json_byte_signing: false,
            requests_device_key_generation: false,
            requests_key_derivation_runtime: false,
            requests_pin_validation_runtime: false,
            requests_pin_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            includes_platform_sealer_implementation: false,
            stores_secret_material: false,
            exports_secret_or_material: false,
            requests_encryption_or_decryption: false,
            requests_live_rpc: false,
            requests_runtime_io: false,
            adds_routes: false,
            requests_storage_mutation: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    fn challenge(
        purpose: NativePassportChallengePurpose,
    ) -> NativePassportChallengeContractDescriptorV1 {
        review_native_passport_challenge_contract_draft(challenge_draft(purpose))
            .expect("valid challenge")
    }

    fn proof_draft(
        challenge: &NativePassportChallengeContractDescriptorV1,
        proof_kind: NativePassportProofKind,
        proof_authority: NativePassportProofAuthority,
        proof_signature_algorithm: NativeProofSignatureAlgorithm,
    ) -> NativePassportProofContractDraftV1 {
        NativePassportProofContractDraftV1 {
            contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            challenge_contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
            challenge_contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
            challenge_id: challenge.challenge_id.clone(),
            purpose: challenge.purpose,
            proof_kind,
            proof_authority,
            requested_scopes: challenge.requested_scopes.clone(),
            passport_id: challenge.passport_id.clone(),
            device_id: challenge.device_id.clone(),
            operation_body_hash: challenge.operation_body_hash.clone(),
            challenge_transcript_hash: challenge_transcript_hash(),
            proof_transcript_hash: proof_transcript_hash(),
            signer_public_key_label: "public-key:v1:test-signer",
            proof_transcript_codec: PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC,
            proof_signature_algorithm,
            service_challenge_placeholder_present: true,
            proof_signature_placeholder_present: true,
            proof_created_at_ms: 1_030_000,
            requests_root_proof_signing: false,
            requests_device_proof_signing: false,
            requests_proof_signature_verification: false,
            requests_request_proof_signing: false,
            requests_request_proof_verification: false,
            requests_service_challenge_signing: false,
            requests_service_challenge_verification: false,
            requests_replay_store_mutation: false,
            requests_challenge_consumption: false,
            requests_capability_issuance: false,
            requests_capability_lifecycle_runtime: false,
            requests_device_authority_execution: false,
            requests_namespace_execution: false,
            requests_json_byte_signing: false,
            requests_device_key_generation: false,
            requests_key_derivation_runtime: false,
            requests_pin_validation_runtime: false,
            requests_pin_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            includes_platform_sealer_implementation: false,
            stores_secret_material: false,
            exports_secret_or_material: false,
            requests_encryption_or_decryption: false,
            requests_live_rpc: false,
            requests_runtime_io: false,
            adds_routes: false,
            requests_storage_mutation: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    fn root_registration_proof_draft(
        challenge: &NativePassportChallengeContractDescriptorV1,
    ) -> NativePassportProofContractDraftV1 {
        proof_draft(
            challenge,
            NativePassportProofKind::RootRegistration,
            NativePassportProofAuthority::RootAuthority,
            PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
        )
    }

    fn device_session_proof_draft(
        challenge: &NativePassportChallengeContractDescriptorV1,
    ) -> NativePassportProofContractDraftV1 {
        proof_draft(
            challenge,
            NativePassportProofKind::DeviceSession,
            NativePassportProofAuthority::DeviceAuthority,
            PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
        )
    }

    #[test]
    fn phase8b_label_and_posture_are_locked() {
        let posture = native_passport_proof_contract_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE8B_LABEL,
            "NATIVE_PASSPORT_PHASE8B_PROOF_CONTRACT_DTO"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE8B_LABEL);
        assert!(posture.proof_contract_dtos_added);

        for authority_added in [
            posture.root_proof_signing_added,
            posture.device_proof_signing_added,
            posture.proof_signature_verification_added,
            posture.request_proof_runtime_added,
            posture.service_challenge_signing_added,
            posture.service_challenge_verification_added,
            posture.replay_store_mutation_added,
            posture.challenge_consumption_added,
            posture.capability_issuance_added,
            posture.capability_lifecycle_runtime_added,
            posture.device_authority_execution_added,
            posture.namespace_execution_added,
            posture.json_byte_signing_added,
            posture.device_key_generation_added,
            posture.key_derivation_runtime_added,
            posture.pin_validation_runtime_added,
            posture.pin_derivation_runtime_added,
            posture.vault_unlock_added,
            posture.vault_runtime_added,
            posture.platform_sealer_implementation_added,
            posture.secret_storage_added,
            posture.material_export_added,
            posture.encryption_runtime_added,
            posture.decryption_runtime_added,
            posture.live_rpc_added,
            posture.runtime_io_added,
            posture.routes_added,
            posture.storage_mutation_added,
            posture.wallet_or_ledger_mutation_added,
            posture.runtime_authority_changed,
            posture.native_secret_implementation_added,
        ] {
            assert!(!authority_added);
        }

        for forbidden in [
            "root_proof_signing",
            "device_proof_signing",
            "proof_signature_verification",
            "request_proof_signing",
            "request_proof_verification",
            "service_challenge_verification",
            "replay_store_mutation",
            "challenge_consumption",
            "capability_issuance",
            "json_byte_signing",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE8B_FORBIDDEN_PROOF_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase8b_surfaces_extend_phase8a_without_back_mutating_it() {
        assert_eq!(
            PHASE8B_ENABLED_SURFACES.len(),
            PHASE8A_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE8B_ENABLED_SURFACES[..PHASE8A_ENABLED_SURFACES.len()],
            PHASE8A_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE8B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ProofContractDto)
        );
        assert_eq!(
            PHASE8A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::PurposeBoundChallengeContractDto)
        );
    }

    #[test]
    fn phase8b_reviews_root_registration_proof_contract_against_challenge() {
        let challenge = challenge(NativePassportChallengePurpose::RegisterRoot);
        let descriptor = review_native_passport_proof_contract_draft(
            &challenge,
            root_registration_proof_draft(&challenge),
        )
        .expect("root registration proof contract should review");

        assert_eq!(descriptor.contract_domain, PHASE8B_PROOF_CONTRACT_DOMAIN);
        assert_eq!(descriptor.contract_version, PHASE8B_PROOF_CONTRACT_VERSION);
        assert_eq!(
            descriptor.challenge_contract_domain,
            PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN
        );
        assert_eq!(descriptor.challenge_id, challenge.challenge_id);
        assert_eq!(
            descriptor.purpose,
            NativePassportChallengePurpose::RegisterRoot
        );
        assert_eq!(
            descriptor.proof_kind,
            NativePassportProofKind::RootRegistration
        );
        assert_eq!(
            descriptor.proof_authority,
            NativePassportProofAuthority::RootAuthority
        );
        assert_eq!(
            descriptor.proof_signature_algorithm,
            NativeProofSignatureAlgorithm::RootEd25519V1
        );
        assert!(descriptor.passport_id.is_some());
        assert!(descriptor.device_id.is_some());
        assert!(descriptor.operation_body_hash.is_some());
        assert!(descriptor.service_challenge_placeholder_present);
        assert!(descriptor.proof_signature_placeholder_present);
        assert!(descriptor.contract_only);

        validate_native_passport_proof_contract_descriptor(&challenge, &descriptor)
            .expect("reviewed root proof descriptor should validate");
    }

    #[test]
    fn phase8b_reviews_device_session_proof_contract_against_challenge() {
        let mut challenge_draft = challenge_draft(NativePassportChallengePurpose::ProveSession);
        challenge_draft.operation_body_hash = None;
        let challenge = review_native_passport_challenge_contract_draft(challenge_draft)
            .expect("session challenge");
        let descriptor = review_native_passport_proof_contract_draft(
            &challenge,
            device_session_proof_draft(&challenge),
        )
        .expect("device session proof contract should review");

        assert_eq!(
            descriptor.purpose,
            NativePassportChallengePurpose::ProveSession
        );
        assert_eq!(
            descriptor.proof_kind,
            NativePassportProofKind::DeviceSession
        );
        assert_eq!(
            descriptor.proof_authority,
            NativePassportProofAuthority::DeviceAuthority
        );
        assert_eq!(
            descriptor.proof_signature_algorithm,
            NativeProofSignatureAlgorithm::DeviceEd25519V1
        );
        assert!(descriptor.passport_id.is_some());
        assert!(descriptor.device_id.is_some());
        assert!(descriptor.operation_body_hash.is_none());
        assert!(descriptor.contract_only);
    }

    #[test]
    fn phase8b_constants_are_locked() {
        assert_eq!(
            PHASE8B_PROOF_CONTRACT_DOMAIN,
            "native-passport/proof-contract/v1"
        );
        assert_eq!(PHASE8B_PROOF_CONTRACT_VERSION, 1);
        assert_eq!(
            PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC,
            NativeChallengeTranscriptCodec::CanonicalB3V1
        );
        assert_eq!(
            PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
            NativeProofSignatureAlgorithm::RootEd25519V1
        );
        assert_eq!(
            PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
            NativeProofSignatureAlgorithm::DeviceEd25519V1
        );
    }

    #[test]
    fn phase8b_rejects_domain_version_challenge_and_binding_drift() {
        let challenge = challenge(NativePassportChallengePurpose::RegisterRoot);

        let mut bad_domain = root_registration_proof_draft(&challenge);
        bad_domain.contract_domain = "native-passport/proof-contract/v0";
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_domain),
            Err(NativePassportProofContractReviewError::ContractDomainMismatch)
        );

        let mut bad_version = root_registration_proof_draft(&challenge);
        bad_version.contract_version = 2;
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_version),
            Err(NativePassportProofContractReviewError::ContractVersionMismatch)
        );

        let mut bad_challenge_domain = root_registration_proof_draft(&challenge);
        bad_challenge_domain.challenge_contract_domain =
            "native-passport/proof-challenge-contract/v0";
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_challenge_domain),
            Err(NativePassportProofContractReviewError::ChallengeContractDomainMismatch)
        );

        let mut bad_challenge_id = root_registration_proof_draft(&challenge);
        bad_challenge_id.challenge_id = other_challenge_id();
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_challenge_id),
            Err(NativePassportProofContractReviewError::ChallengeIdMismatch)
        );

        let mut bad_purpose = root_registration_proof_draft(&challenge);
        bad_purpose.purpose = NativePassportChallengePurpose::AuthorizeDevice;
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_purpose),
            Err(NativePassportProofContractReviewError::PurposeMismatch)
        );

        let mut bad_scopes = root_registration_proof_draft(&challenge);
        bad_scopes
            .requested_scopes
            .push(NativePassportScope::ContentRead);
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_scopes),
            Err(NativePassportProofContractReviewError::RequestedScopeMismatch)
        );

        let mut bad_passport = root_registration_proof_draft(&challenge);
        bad_passport.passport_id = None;
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_passport),
            Err(NativePassportProofContractReviewError::PassportBindingMismatch)
        );

        let mut bad_device = root_registration_proof_draft(&challenge);
        bad_device.device_id = None;
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_device),
            Err(NativePassportProofContractReviewError::DeviceBindingMismatch)
        );

        let mut bad_body = root_registration_proof_draft(&challenge);
        bad_body.operation_body_hash = None;
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_body),
            Err(NativePassportProofContractReviewError::OperationBodyHashMismatch)
        );
    }

    #[test]
    fn phase8b_rejects_wrong_kind_authority_codec_algorithm_placeholders_time_and_non_contract() {
        let challenge = challenge(NativePassportChallengePurpose::RegisterRoot);

        let mut bad_kind = root_registration_proof_draft(&challenge);
        bad_kind.proof_kind = NativePassportProofKind::DeviceSession;
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_kind),
            Err(NativePassportProofContractReviewError::InvalidProofKindForPurpose)
        );

        let mut bad_authority = root_registration_proof_draft(&challenge);
        bad_authority.proof_authority = NativePassportProofAuthority::DeviceAuthority;
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_authority),
            Err(NativePassportProofContractReviewError::InvalidProofAuthorityForPurpose)
        );

        let mut empty_signer = root_registration_proof_draft(&challenge);
        empty_signer.signer_public_key_label = "";
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, empty_signer),
            Err(NativePassportProofContractReviewError::MissingSignerPublicKeyLabel)
        );

        let mut bad_codec = root_registration_proof_draft(&challenge);
        bad_codec.proof_transcript_codec = NativeChallengeTranscriptCodec::JsonBytesRejected;
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_codec),
            Err(NativePassportProofContractReviewError::InvalidProofTranscriptCodec)
        );

        let mut bad_algorithm = root_registration_proof_draft(&challenge);
        bad_algorithm.proof_signature_algorithm = NativeProofSignatureAlgorithm::MissingOrUnknown;
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_algorithm),
            Err(NativePassportProofContractReviewError::InvalidProofSignatureAlgorithm)
        );

        let mut missing_service = root_registration_proof_draft(&challenge);
        missing_service.service_challenge_placeholder_present = false;
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, missing_service),
            Err(NativePassportProofContractReviewError::MissingServiceChallengePlaceholder)
        );

        let mut missing_proof = root_registration_proof_draft(&challenge);
        missing_proof.proof_signature_placeholder_present = false;
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, missing_proof),
            Err(NativePassportProofContractReviewError::MissingProofSignaturePlaceholder)
        );

        let mut bad_time = root_registration_proof_draft(&challenge);
        bad_time.proof_created_at_ms = challenge.expires_at_ms + 1;
        assert_eq!(
            review_native_passport_proof_contract_draft(&challenge, bad_time),
            Err(NativePassportProofContractReviewError::InvalidProofCreatedAt)
        );

        let mut descriptor = review_native_passport_proof_contract_draft(
            &challenge,
            root_registration_proof_draft(&challenge),
        )
        .expect("descriptor");
        descriptor.contract_only = false;
        assert_eq!(
            validate_native_passport_proof_contract_descriptor(&challenge, &descriptor),
            Err(NativePassportProofContractReviewError::ProofContractNotContractOnly)
        );
    }

    #[test]
    fn phase8b_rejects_all_unsafe_runtime_authority_flags() {
        let challenge = challenge(NativePassportChallengePurpose::RegisterRoot);

        let mutators: &[fn(&mut NativePassportProofContractDraftV1)] = &[
            |draft| draft.requests_root_proof_signing = true,
            |draft| draft.requests_device_proof_signing = true,
            |draft| draft.requests_proof_signature_verification = true,
            |draft| draft.requests_request_proof_signing = true,
            |draft| draft.requests_request_proof_verification = true,
            |draft| draft.requests_service_challenge_signing = true,
            |draft| draft.requests_service_challenge_verification = true,
            |draft| draft.requests_replay_store_mutation = true,
            |draft| draft.requests_challenge_consumption = true,
            |draft| draft.requests_capability_issuance = true,
            |draft| draft.requests_capability_lifecycle_runtime = true,
            |draft| draft.requests_device_authority_execution = true,
            |draft| draft.requests_namespace_execution = true,
            |draft| draft.requests_json_byte_signing = true,
            |draft| draft.requests_device_key_generation = true,
            |draft| draft.requests_key_derivation_runtime = true,
            |draft| draft.requests_pin_validation_runtime = true,
            |draft| draft.requests_pin_derivation_runtime = true,
            |draft| draft.requests_vault_unlock = true,
            |draft| draft.includes_vault_runtime = true,
            |draft| draft.includes_platform_sealer_implementation = true,
            |draft| draft.stores_secret_material = true,
            |draft| draft.exports_secret_or_material = true,
            |draft| draft.requests_encryption_or_decryption = true,
            |draft| draft.requests_live_rpc = true,
            |draft| draft.requests_runtime_io = true,
            |draft| draft.adds_routes = true,
            |draft| draft.requests_storage_mutation = true,
            |draft| draft.requests_wallet_or_ledger_mutation = true,
        ];

        for mutate in mutators {
            let mut draft = root_registration_proof_draft(&challenge);
            mutate(&mut draft);
            assert_eq!(
                review_native_passport_proof_contract_draft(&challenge, draft),
                Err(NativePassportProofContractReviewError::UnsafeProofAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase8b_source_remains_contract_only_without_signing_verification_replay_io_or_secrets() {
        let source = fs::read_to_string(repo_file("src/native/proof.rs")).expect("proof source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativePassportProofContractDescriptorV1"));
        assert!(source.contains("NativePassportProofContractDraftV1"));
        assert!(source.contains("review_native_passport_proof_contract_draft"));
        assert!(native_mod.contains("ProofContractDto"));
        assert!(!source.contains("serde_json"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "sign(",
            "verify_signature(",
            "verify_proof(",
            "verify_request_proof(",
            "consume_challenge(",
            "replay_store.write(",
            "issue_capability(",
            "refresh_capability(",
            "revoke_capability(",
            "execute_device_authorization(",
            "execute_device_revocation(",
            "execute_namespace(",
            "generate_device_key(",
            "derive_root_key(",
            "derive_device_key(",
            "validate_pin(",
            "derive_pin(",
            "unlock_vault(",
            "platform_seal(",
            "platform_unseal(",
            "encrypt(",
            "decrypt(",
            "rpc.submit(",
            "rpc.send(",
            "runtime_read(",
            "runtime_write(",
            "storage.write(",
            "wallet.spend(",
            "ledger.write(",
            "mint_roc(",
            "burn_roc(",
            "mnemonic:",
            "seed_phrase:",
            "raw_pin:",
            "pin_hash:",
            "root_private_key:",
            "device_private_key:",
            "vault_master_key:",
            "ciphertext:",
            "plaintext:",
            "sealed_bytes:",
            "service_signature_bytes:",
            "root_signature_bytes:",
            "device_signature_bytes:",
            "capability_token:",
        ] {
            assert!(
                !source.contains(forbidden_runtime_pattern),
                "Phase 8B proof source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
