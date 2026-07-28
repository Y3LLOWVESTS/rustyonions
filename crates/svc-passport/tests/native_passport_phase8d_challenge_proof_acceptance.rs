#[cfg(not(feature = "native-passport"))]
#[test]
fn phase8d_acceptance_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native Passport Phase 8 acceptance surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_challenge_contract_posture, native_passport_proof_contract_posture,
        native_passport_request_proof_contract_posture,
        review_native_passport_challenge_contract_draft,
        review_native_passport_proof_contract_draft,
        review_native_passport_request_proof_contract_draft,
        validate_native_passport_challenge_contract_descriptor,
        validate_native_passport_proof_contract_descriptor,
        validate_native_passport_request_proof_contract_descriptor, B3DigestHex, ChallengeIdV1,
        DeviceIdV1, NativeChallengeSignatureAlgorithm, NativeChallengeTranscriptCodec,
        NativePassportChallengeContractDescriptorV1, NativePassportChallengeContractDraftV1,
        NativePassportChallengePurpose, NativePassportProofAuthority,
        NativePassportProofContractDescriptorV1, NativePassportProofContractDraftV1,
        NativePassportProofKind, NativePassportRequestMethod,
        NativePassportRequestProofContractDraftV1, NativePassportRequestProofContractReviewError,
        NativePassportScope, NativePassportSurface, NativeProofSignatureAlgorithm, PassportIdV1,
        PHASE7C_ENABLED_SURFACES, PHASE8A_ENABLED_SURFACES,
        PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN, PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
        PHASE8A_REQUIRED_SERVICE_SIGNATURE_ALGORITHM, PHASE8A_REQUIRED_TRANSCRIPT_CODEC,
        PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_ENABLED_SURFACES,
        PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
        PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC, PHASE8C_ENABLED_SURFACES,
        PHASE8C_MAX_CLOCK_SKEW_MS, PHASE8C_MAX_REQUEST_TTL_MS,
        PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN, PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
        PHASE8C_REQUIRED_REQUEST_TRANSCRIPT_CODEC,
    };

    const PHASE8D_ACCEPTANCE_LABEL: &str = "NATIVE_PASSPORT_PHASE8D_CHALLENGE_PROOF_ACCEPTANCE";

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const HEX_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn digest(label: &'static str, hex: &'static str) -> B3DigestHex {
        B3DigestHex::parse(label, hex).expect(label)
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

    fn challenge_draft() -> NativePassportChallengeContractDraftV1 {
        NativePassportChallengeContractDraftV1 {
            contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
            contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
            challenge_id: challenge_id(),
            network_id: "rustyonions-main",
            environment: "private-beta",
            audience: "svc-passport",
            issuing_service_id: "svc-passport",
            service_key_id: "service-key:v1:challenge",
            purpose: NativePassportChallengePurpose::ProveSession,
            requested_scopes: vec![
                NativePassportScope::IdentityRead,
                NativePassportScope::CatalogRead,
                NativePassportScope::ContentRead,
            ],
            passport_id: Some(passport_id()),
            device_id: Some(device_id()),
            operation_body_hash: None,
            nonce_hex: digest("challenge_nonce_hex", HEX_A),
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

    fn challenge() -> NativePassportChallengeContractDescriptorV1 {
        review_native_passport_challenge_contract_draft(challenge_draft())
            .expect("valid challenge contract")
    }

    fn proof_draft(
        challenge: &NativePassportChallengeContractDescriptorV1,
    ) -> NativePassportProofContractDraftV1 {
        NativePassportProofContractDraftV1 {
            contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            challenge_contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
            challenge_contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
            challenge_id: challenge.challenge_id.clone(),
            purpose: challenge.purpose,
            proof_kind: NativePassportProofKind::DeviceSession,
            proof_authority: NativePassportProofAuthority::DeviceAuthority,
            requested_scopes: challenge.requested_scopes.clone(),
            passport_id: challenge.passport_id.clone(),
            device_id: challenge.device_id.clone(),
            operation_body_hash: challenge.operation_body_hash.clone(),
            challenge_transcript_hash: digest("challenge_transcript_hash", HEX_E),
            proof_transcript_hash: digest("proof_transcript_hash", HEX_F),
            signer_public_key_label: "public-key:v1:phase8d-device",
            proof_transcript_codec: PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC,
            proof_signature_algorithm: PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
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

    fn proof(
        challenge: &NativePassportChallengeContractDescriptorV1,
    ) -> NativePassportProofContractDescriptorV1 {
        review_native_passport_proof_contract_draft(challenge, proof_draft(challenge))
            .expect("valid proof contract")
    }

    fn request_draft(
        challenge: &NativePassportChallengeContractDescriptorV1,
        proof: &NativePassportProofContractDescriptorV1,
    ) -> NativePassportRequestProofContractDraftV1 {
        NativePassportRequestProofContractDraftV1 {
            contract_domain: PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN,
            contract_version: PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            challenge_id: proof.challenge_id.clone(),
            network_id: challenge.network_id,
            environment: challenge.environment,
            audience: challenge.audience,
            request_target_origin: "crab://svc-passport",
            request_method: NativePassportRequestMethod::Get,
            request_path: "/v1/passport/profile",
            request_query_hash: Some(digest("request_query_hash", HEX_D)),
            request_body_hash: None,
            requested_scopes: vec![NativePassportScope::IdentityRead],
            passport_id: proof.passport_id.clone(),
            device_id: proof.device_id.clone(),
            proof_kind: proof.proof_kind,
            proof_authority: proof.proof_authority,
            proof_transcript_hash: proof.proof_transcript_hash.clone(),
            request_transcript_hash: digest("request_transcript_hash", HEX_E),
            request_nonce_hash: digest("request_nonce_hash", HEX_A),
            request_transcript_codec: PHASE8C_REQUIRED_REQUEST_TRANSCRIPT_CODEC,
            proof_signature_placeholder_present: true,
            request_created_at_ms: 1_040_000,
            request_expires_at_ms: 1_050_000,
            max_clock_skew_ms: 10_000,
            requests_request_proof_signing: false,
            requests_request_proof_verification: false,
            requests_proof_signature_verification: false,
            requests_service_challenge_verification: false,
            requests_replay_store_mutation: false,
            requests_challenge_consumption: false,
            requests_capability_consumption: false,
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

    #[test]
    fn phase8d_acceptance_label_is_locked() {
        assert_eq!(
            PHASE8D_ACCEPTANCE_LABEL,
            "NATIVE_PASSPORT_PHASE8D_CHALLENGE_PROOF_ACCEPTANCE"
        );
    }

    #[test]
    fn phase8d_accepts_challenge_proof_request_surface_chain() {
        assert_eq!(
            PHASE8A_ENABLED_SURFACES.len(),
            PHASE7C_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            PHASE8B_ENABLED_SURFACES.len(),
            PHASE8A_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            PHASE8C_ENABLED_SURFACES.len(),
            PHASE8B_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            PHASE8A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::PurposeBoundChallengeContractDto)
        );
        assert_eq!(
            PHASE8B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ProofContractDto)
        );
        assert_eq!(
            PHASE8C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::RequestProofContractDto)
        );
    }

    #[test]
    fn phase8d_accepts_phase8_postures_without_runtime_authority() {
        let challenge_posture = native_passport_challenge_contract_posture();
        let proof_posture = native_passport_proof_contract_posture();
        let request_posture = native_passport_request_proof_contract_posture();

        assert!(challenge_posture.proof_challenge_contract_dtos_added);
        assert!(proof_posture.proof_contract_dtos_added);
        assert!(request_posture.request_proof_contract_dtos_added);

        for denied in [
            challenge_posture.service_challenge_signing_added,
            challenge_posture.service_challenge_verification_added,
            challenge_posture.root_proof_signing_added,
            challenge_posture.device_proof_signing_added,
            challenge_posture.proof_signature_verification_added,
            challenge_posture.request_proof_runtime_added,
            challenge_posture.replay_store_mutation_added,
            challenge_posture.challenge_consumption_added,
            challenge_posture.capability_issuance_added,
            challenge_posture.runtime_io_added,
            challenge_posture.routes_added,
            challenge_posture.storage_mutation_added,
            challenge_posture.wallet_or_ledger_mutation_added,
            challenge_posture.runtime_authority_changed,
            challenge_posture.native_secret_implementation_added,
            proof_posture.root_proof_signing_added,
            proof_posture.device_proof_signing_added,
            proof_posture.proof_signature_verification_added,
            proof_posture.request_proof_runtime_added,
            proof_posture.service_challenge_verification_added,
            proof_posture.replay_store_mutation_added,
            proof_posture.challenge_consumption_added,
            proof_posture.capability_issuance_added,
            proof_posture.runtime_io_added,
            proof_posture.routes_added,
            proof_posture.storage_mutation_added,
            proof_posture.wallet_or_ledger_mutation_added,
            proof_posture.runtime_authority_changed,
            proof_posture.native_secret_implementation_added,
            request_posture.request_proof_signing_added,
            request_posture.request_proof_verification_added,
            request_posture.proof_signature_verification_added,
            request_posture.replay_store_mutation_added,
            request_posture.challenge_consumption_added,
            request_posture.capability_consumption_added,
            request_posture.capability_issuance_added,
            request_posture.runtime_io_added,
            request_posture.routes_added,
            request_posture.storage_mutation_added,
            request_posture.wallet_or_ledger_mutation_added,
            request_posture.runtime_authority_changed,
            request_posture.native_secret_implementation_added,
        ] {
            assert!(!denied);
        }
    }

    #[test]
    fn phase8d_accepts_challenge_to_proof_to_request_contract_flow() {
        let challenge = challenge();
        validate_native_passport_challenge_contract_descriptor(&challenge)
            .expect("Phase 8A challenge descriptor should validate");

        let proof = proof(&challenge);
        validate_native_passport_proof_contract_descriptor(&challenge, &proof)
            .expect("Phase 8B proof descriptor should validate");

        let request = review_native_passport_request_proof_contract_draft(
            &challenge,
            &proof,
            request_draft(&challenge, &proof),
        )
        .expect("Phase 8C request-proof descriptor should review");

        validate_native_passport_request_proof_contract_descriptor(&challenge, &proof, &request)
            .expect("Phase 8C request-proof descriptor should validate");

        assert_eq!(
            challenge.transcript_codec,
            NativeChallengeTranscriptCodec::CanonicalB3V1
        );
        assert_eq!(
            challenge.service_signature_algorithm,
            NativeChallengeSignatureAlgorithm::ServiceEd25519V1
        );
        assert_eq!(
            proof.proof_transcript_codec,
            NativeChallengeTranscriptCodec::CanonicalB3V1
        );
        assert_eq!(
            proof.proof_signature_algorithm,
            NativeProofSignatureAlgorithm::DeviceEd25519V1
        );
        assert_eq!(
            request.request_transcript_codec,
            NativeChallengeTranscriptCodec::CanonicalB3V1
        );

        assert_eq!(proof.challenge_id, challenge.challenge_id);
        assert_eq!(request.challenge_id, challenge.challenge_id);
        assert_eq!(request.proof_transcript_hash, proof.proof_transcript_hash);
        assert_eq!(request.network_id, challenge.network_id);
        assert_eq!(request.environment, challenge.environment);
        assert_eq!(request.audience, challenge.audience);

        assert!(challenge.contract_only);
        assert!(proof.contract_only);
        assert!(request.contract_only);
    }

    #[test]
    fn phase8d_rejects_cross_layer_drift_before_request_acceptance() {
        let challenge = challenge();
        let proof = proof(&challenge);

        let mut wrong_challenge = request_draft(&challenge, &proof);
        wrong_challenge.challenge_id = other_challenge_id();
        assert_eq!(
            review_native_passport_request_proof_contract_draft(
                &challenge,
                &proof,
                wrong_challenge,
            ),
            Err(NativePassportRequestProofContractReviewError::ChallengeIdMismatch)
        );

        let mut wrong_network = request_draft(&challenge, &proof);
        wrong_network.network_id = "wrong-network";
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, wrong_network,),
            Err(NativePassportRequestProofContractReviewError::NetworkMismatch)
        );

        let mut wrong_scope = request_draft(&challenge, &proof);
        wrong_scope.requested_scopes = vec![NativePassportScope::ReceiptsRead];
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, wrong_scope),
            Err(NativePassportRequestProofContractReviewError::ScopeNotGrantedByProof)
        );

        let mut wrong_proof_hash = request_draft(&challenge, &proof);
        wrong_proof_hash.proof_transcript_hash = digest("proof_transcript_hash", HEX_D);
        assert_eq!(
            review_native_passport_request_proof_contract_draft(
                &challenge,
                &proof,
                wrong_proof_hash,
            ),
            Err(NativePassportRequestProofContractReviewError::ProofTranscriptHashMismatch)
        );

        let mut non_contract_proof = proof.clone();
        non_contract_proof.contract_only = false;
        assert_eq!(
            review_native_passport_request_proof_contract_draft(
                &challenge,
                &non_contract_proof,
                request_draft(&challenge, &non_contract_proof),
            ),
            Err(NativePassportRequestProofContractReviewError::ProofContractReferenceInvalid)
        );
    }

    #[test]
    fn phase8d_accepts_request_method_time_and_canonical_codec_ceiling() {
        assert_eq!(PHASE8C_MAX_REQUEST_TTL_MS, 120_000);
        assert_eq!(PHASE8C_MAX_CLOCK_SKEW_MS, 30_000);
        assert_eq!(
            PHASE8C_REQUIRED_REQUEST_TRANSCRIPT_CODEC,
            NativeChallengeTranscriptCodec::CanonicalB3V1
        );

        let challenge = challenge();
        let proof = proof(&challenge);

        let mut post = request_draft(&challenge, &proof);
        post.request_method = NativePassportRequestMethod::Post;
        post.request_body_hash = Some(digest("request_body_hash", HEX_D));
        post.request_path = "/v1/passport/profile";

        let request = review_native_passport_request_proof_contract_draft(&challenge, &proof, post)
            .expect("mutation-like request proof accepts body hash");

        assert_eq!(request.request_method, NativePassportRequestMethod::Post);
        assert!(request.request_body_hash.is_some());
        assert!(request.request_created_at_ms >= proof.proof_created_at_ms);
        assert!(request.request_expires_at_ms <= proof.challenge_expires_at_ms);
    }

    #[test]
    fn phase8d_acceptance_sources_remain_contract_only_without_signing_verification_replay_routes_or_secrets(
    ) {
        let challenge_source =
            fs::read_to_string(repo_file("src/native/challenge.rs")).expect("challenge source");
        let proof_source =
            fs::read_to_string(repo_file("src/native/proof.rs")).expect("proof source");
        let request_source = fs::read_to_string(repo_file("src/native/request_proof.rs"))
            .expect("request proof source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        for required_token in [
            "PurposeBoundChallengeContractDto",
            "ProofContractDto",
            "RequestProofContractDto",
            "PHASE8A_ENABLED_SURFACES",
            "PHASE8B_ENABLED_SURFACES",
            "PHASE8C_ENABLED_SURFACES",
        ] {
            assert!(
                native_mod.contains(required_token),
                "native module should expose {required_token}"
            );
        }

        let combined =
            format!("{challenge_source}\n{proof_source}\n{request_source}\n{native_mod}");

        assert!(!combined.contains("serde_json"));
        assert!(!combined.contains("JsonBytesAccepted"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "sign(",
            "verify_signature(",
            "verify_proof(",
            "verify_request_proof(",
            "consume_challenge(",
            "consume_capability(",
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
            "request_signature_bytes:",
            "proof_signature_bytes:",
            "capability_token:",
        ] {
            assert!(
                !combined.contains(forbidden_runtime_pattern),
                "Phase 8 acceptance sources must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
