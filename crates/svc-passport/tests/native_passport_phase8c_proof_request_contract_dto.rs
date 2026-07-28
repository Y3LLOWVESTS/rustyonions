#[cfg(not(feature = "native-passport"))]
#[test]
fn phase8c_request_proof_contract_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native request-proof contract DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_request_proof_contract_posture,
        review_native_passport_challenge_contract_draft,
        review_native_passport_proof_contract_draft,
        review_native_passport_request_proof_contract_draft,
        validate_native_passport_request_proof_contract_descriptor, B3DigestHex, ChallengeIdV1,
        DeviceIdV1, NativeChallengeTranscriptCodec, NativePassportChallengeContractDescriptorV1,
        NativePassportChallengeContractDraftV1, NativePassportChallengePurpose,
        NativePassportProofAuthority, NativePassportProofContractDescriptorV1,
        NativePassportProofContractDraftV1, NativePassportProofKind, NativePassportRequestMethod,
        NativePassportRequestProofContractDraftV1, NativePassportRequestProofContractReviewError,
        NativePassportScope, NativePassportSurface, PassportIdV1, NATIVE_PASSPORT_PHASE8C_LABEL,
        PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN, PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
        PHASE8A_REQUIRED_SERVICE_SIGNATURE_ALGORITHM, PHASE8A_REQUIRED_TRANSCRIPT_CODEC,
        PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_ENABLED_SURFACES,
        PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
        PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC, PHASE8C_ALLOWED_REQUEST_METHODS,
        PHASE8C_ENABLED_SURFACES, PHASE8C_FORBIDDEN_REQUEST_PROOF_AUTHORITY_FLAGS,
        PHASE8C_MAX_CLOCK_SKEW_MS, PHASE8C_MAX_REQUEST_TTL_MS,
        PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN, PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
        PHASE8C_REQUIRED_REQUEST_TRANSCRIPT_CODEC,
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
        review_native_passport_challenge_contract_draft(challenge_draft()).expect("valid challenge")
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
            signer_public_key_label: "public-key:v1:test-device",
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
            .expect("valid proof")
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
    fn phase8c_label_and_posture_are_locked() {
        let posture = native_passport_request_proof_contract_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE8C_LABEL,
            "NATIVE_PASSPORT_PHASE8C_PROOF_REQUEST_CONTRACT_DTO"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE8C_LABEL);
        assert!(posture.request_proof_contract_dtos_added);

        for authority_added in [
            posture.request_proof_signing_added,
            posture.request_proof_verification_added,
            posture.proof_signature_verification_added,
            posture.replay_store_mutation_added,
            posture.challenge_consumption_added,
            posture.capability_consumption_added,
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
            "request_proof_signing",
            "request_proof_verification",
            "proof_signature_verification",
            "replay_store_mutation",
            "challenge_consumption",
            "capability_consumption",
            "json_byte_signing",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE8C_FORBIDDEN_REQUEST_PROOF_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase8c_surfaces_extend_phase8b_without_back_mutating_it() {
        assert_eq!(
            PHASE8C_ENABLED_SURFACES.len(),
            PHASE8B_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE8C_ENABLED_SURFACES[..PHASE8B_ENABLED_SURFACES.len()],
            PHASE8B_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE8C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::RequestProofContractDto)
        );
        assert_eq!(
            PHASE8B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ProofContractDto)
        );
    }

    #[test]
    fn phase8c_reviews_request_proof_draft_against_challenge_and_proof() {
        let challenge = challenge();
        let proof = proof(&challenge);

        let descriptor = review_native_passport_request_proof_contract_draft(
            &challenge,
            &proof,
            request_draft(&challenge, &proof),
        )
        .expect("valid request proof contract should review");

        assert_eq!(
            descriptor.contract_domain,
            PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN
        );
        assert_eq!(
            descriptor.contract_version,
            PHASE8C_PROOF_REQUEST_CONTRACT_VERSION
        );
        assert_eq!(
            descriptor.proof_contract_domain,
            PHASE8B_PROOF_CONTRACT_DOMAIN
        );
        assert_eq!(descriptor.challenge_id, proof.challenge_id);
        assert_eq!(descriptor.network_id, challenge.network_id);
        assert_eq!(descriptor.environment, challenge.environment);
        assert_eq!(descriptor.audience, challenge.audience);
        assert_eq!(descriptor.request_target_origin, "crab://svc-passport");
        assert_eq!(descriptor.request_method, NativePassportRequestMethod::Get);
        assert_eq!(descriptor.request_path, "/v1/passport/profile");
        assert_eq!(
            descriptor.requested_scopes,
            vec![NativePassportScope::IdentityRead]
        );
        assert!(descriptor.passport_id.is_some());
        assert!(descriptor.device_id.is_some());
        assert_eq!(
            descriptor.proof_kind,
            NativePassportProofKind::DeviceSession
        );
        assert_eq!(
            descriptor.proof_authority,
            NativePassportProofAuthority::DeviceAuthority
        );
        assert_eq!(
            descriptor.proof_transcript_hash,
            proof.proof_transcript_hash
        );
        assert_eq!(
            descriptor.request_transcript_codec,
            NativeChallengeTranscriptCodec::CanonicalB3V1
        );
        assert!(descriptor.proof_signature_placeholder_present);
        assert!(descriptor.contract_only);

        validate_native_passport_request_proof_contract_descriptor(&challenge, &proof, &descriptor)
            .expect("reviewed request-proof descriptor should validate");
    }

    #[test]
    fn phase8c_accepts_mutation_like_methods_only_with_body_hash() {
        let challenge = challenge();
        let proof = proof(&challenge);

        let mut post = request_draft(&challenge, &proof);
        post.request_method = NativePassportRequestMethod::Post;
        post.request_path = "/v1/passport/profile";
        post.request_body_hash = Some(digest("request_body_hash", HEX_D));

        let descriptor =
            review_native_passport_request_proof_contract_draft(&challenge, &proof, post)
                .expect("POST request proof requires and accepts body hash");

        assert_eq!(descriptor.request_method, NativePassportRequestMethod::Post);
        assert!(descriptor.request_body_hash.is_some());
    }

    #[test]
    fn phase8c_constants_are_locked() {
        assert_eq!(
            PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN,
            "native-passport/proof-request-contract/v1"
        );
        assert_eq!(PHASE8C_PROOF_REQUEST_CONTRACT_VERSION, 1);
        assert_eq!(PHASE8C_MAX_REQUEST_TTL_MS, 120_000);
        assert_eq!(PHASE8C_MAX_CLOCK_SKEW_MS, 30_000);
        assert_eq!(
            PHASE8C_REQUIRED_REQUEST_TRANSCRIPT_CODEC,
            NativeChallengeTranscriptCodec::CanonicalB3V1
        );
        assert_eq!(
            PHASE8C_ALLOWED_REQUEST_METHODS,
            &[
                NativePassportRequestMethod::Get,
                NativePassportRequestMethod::Head,
                NativePassportRequestMethod::Post,
                NativePassportRequestMethod::Put,
                NativePassportRequestMethod::Patch,
                NativePassportRequestMethod::Delete,
            ]
        );
    }

    #[test]
    fn phase8c_rejects_domain_version_reference_audience_and_binding_drift() {
        let challenge = challenge();
        let proof = proof(&challenge);

        let mut bad_domain = request_draft(&challenge, &proof);
        bad_domain.contract_domain = "native-passport/proof-request-contract/v0";
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, bad_domain),
            Err(NativePassportRequestProofContractReviewError::ContractDomainMismatch)
        );

        let mut bad_version = request_draft(&challenge, &proof);
        bad_version.contract_version = 2;
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, bad_version),
            Err(NativePassportRequestProofContractReviewError::ContractVersionMismatch)
        );

        let mut bad_proof_domain = request_draft(&challenge, &proof);
        bad_proof_domain.proof_contract_domain = "native-passport/proof-contract/v0";
        assert_eq!(
            review_native_passport_request_proof_contract_draft(
                &challenge,
                &proof,
                bad_proof_domain,
            ),
            Err(NativePassportRequestProofContractReviewError::ProofContractDomainMismatch)
        );

        let mut bad_challenge = request_draft(&challenge, &proof);
        bad_challenge.challenge_id = other_challenge_id();
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, bad_challenge),
            Err(NativePassportRequestProofContractReviewError::ChallengeIdMismatch)
        );

        let mut bad_network = request_draft(&challenge, &proof);
        bad_network.network_id = "wrong-network";
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, bad_network),
            Err(NativePassportRequestProofContractReviewError::NetworkMismatch)
        );

        let mut bad_environment = request_draft(&challenge, &proof);
        bad_environment.environment = "prod";
        assert_eq!(
            review_native_passport_request_proof_contract_draft(
                &challenge,
                &proof,
                bad_environment,
            ),
            Err(NativePassportRequestProofContractReviewError::EnvironmentMismatch)
        );

        let mut bad_audience = request_draft(&challenge, &proof);
        bad_audience.audience = "other-service";
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, bad_audience),
            Err(NativePassportRequestProofContractReviewError::AudienceMismatch)
        );

        let mut bad_passport = request_draft(&challenge, &proof);
        bad_passport.passport_id = None;
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, bad_passport),
            Err(NativePassportRequestProofContractReviewError::PassportBindingMismatch)
        );

        let mut bad_device = request_draft(&challenge, &proof);
        bad_device.device_id = None;
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, bad_device),
            Err(NativePassportRequestProofContractReviewError::DeviceBindingMismatch)
        );
    }

    #[test]
    fn phase8c_rejects_request_target_scope_body_time_codec_placeholder_and_non_contract_drift() {
        let challenge = challenge();
        let proof = proof(&challenge);

        let mut empty_origin = request_draft(&challenge, &proof);
        empty_origin.request_target_origin = "";
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, empty_origin),
            Err(NativePassportRequestProofContractReviewError::EmptyRequestTargetOrigin)
        );

        let mut bad_path = request_draft(&challenge, &proof);
        bad_path.request_path = "relative/path";
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, bad_path),
            Err(NativePassportRequestProofContractReviewError::InvalidRequestPath)
        );

        let mut empty_scopes = request_draft(&challenge, &proof);
        empty_scopes.requested_scopes.clear();
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, empty_scopes),
            Err(NativePassportRequestProofContractReviewError::EmptyRequestedScopes)
        );

        let mut duplicate_scope = request_draft(&challenge, &proof);
        duplicate_scope
            .requested_scopes
            .push(NativePassportScope::IdentityRead);
        assert_eq!(
            review_native_passport_request_proof_contract_draft(
                &challenge,
                &proof,
                duplicate_scope,
            ),
            Err(NativePassportRequestProofContractReviewError::DuplicateRequestedScope)
        );

        let mut ungranted_scope = request_draft(&challenge, &proof);
        ungranted_scope.requested_scopes = vec![NativePassportScope::ReceiptsRead];
        assert_eq!(
            review_native_passport_request_proof_contract_draft(
                &challenge,
                &proof,
                ungranted_scope,
            ),
            Err(NativePassportRequestProofContractReviewError::ScopeNotGrantedByProof)
        );

        let mut unexpected_body = request_draft(&challenge, &proof);
        unexpected_body.request_body_hash = Some(digest("request_body_hash", HEX_D));
        assert_eq!(
            review_native_passport_request_proof_contract_draft(
                &challenge,
                &proof,
                unexpected_body,
            ),
            Err(NativePassportRequestProofContractReviewError::UnexpectedRequestBodyHash)
        );

        let mut missing_body = request_draft(&challenge, &proof);
        missing_body.request_method = NativePassportRequestMethod::Post;
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, missing_body),
            Err(NativePassportRequestProofContractReviewError::MissingRequestBodyHash)
        );

        let mut bad_time = request_draft(&challenge, &proof);
        bad_time.request_created_at_ms = proof.proof_created_at_ms - 1;
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, bad_time),
            Err(NativePassportRequestProofContractReviewError::RequestOutsideProofWindow)
        );

        let mut too_long = request_draft(&challenge, &proof);
        too_long.request_expires_at_ms =
            too_long.request_created_at_ms + PHASE8C_MAX_REQUEST_TTL_MS + 1;
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, too_long),
            Err(NativePassportRequestProofContractReviewError::RequestTtlTooLong)
        );

        let mut bad_skew = request_draft(&challenge, &proof);
        bad_skew.max_clock_skew_ms = PHASE8C_MAX_CLOCK_SKEW_MS + 1;
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, bad_skew),
            Err(NativePassportRequestProofContractReviewError::ClockSkewTooLong)
        );

        let mut bad_codec = request_draft(&challenge, &proof);
        bad_codec.request_transcript_codec = NativeChallengeTranscriptCodec::JsonBytesRejected;
        assert_eq!(
            review_native_passport_request_proof_contract_draft(&challenge, &proof, bad_codec),
            Err(NativePassportRequestProofContractReviewError::InvalidRequestTranscriptCodec)
        );

        let mut missing_placeholder = request_draft(&challenge, &proof);
        missing_placeholder.proof_signature_placeholder_present = false;
        assert_eq!(
            review_native_passport_request_proof_contract_draft(
                &challenge,
                &proof,
                missing_placeholder,
            ),
            Err(NativePassportRequestProofContractReviewError::MissingProofSignaturePlaceholder)
        );

        let mut descriptor = review_native_passport_request_proof_contract_draft(
            &challenge,
            &proof,
            request_draft(&challenge, &proof),
        )
        .expect("descriptor");
        descriptor.contract_only = false;
        assert_eq!(
            validate_native_passport_request_proof_contract_descriptor(
                &challenge,
                &proof,
                &descriptor,
            ),
            Err(NativePassportRequestProofContractReviewError::RequestProofContractNotContractOnly)
        );
    }

    #[test]
    fn phase8c_rejects_all_unsafe_runtime_authority_flags() {
        let challenge = challenge();
        let proof = proof(&challenge);

        let mutators: &[fn(&mut NativePassportRequestProofContractDraftV1)] = &[
            |draft| draft.requests_request_proof_signing = true,
            |draft| draft.requests_request_proof_verification = true,
            |draft| draft.requests_proof_signature_verification = true,
            |draft| draft.requests_service_challenge_verification = true,
            |draft| draft.requests_replay_store_mutation = true,
            |draft| draft.requests_challenge_consumption = true,
            |draft| draft.requests_capability_consumption = true,
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
            let mut draft = request_draft(&challenge, &proof);
            mutate(&mut draft);
            assert_eq!(
                review_native_passport_request_proof_contract_draft(&challenge, &proof, draft),
                Err(NativePassportRequestProofContractReviewError::UnsafeRequestProofAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase8c_source_remains_contract_only_without_signing_verification_replay_io_or_secrets() {
        let source = fs::read_to_string(repo_file("src/native/request_proof.rs"))
            .expect("request proof source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativePassportRequestProofContractDescriptorV1"));
        assert!(source.contains("NativePassportRequestProofContractDraftV1"));
        assert!(source.contains("review_native_passport_request_proof_contract_draft"));
        assert!(native_mod.contains("RequestProofContractDto"));
        assert!(!source.contains("serde_json"));

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
            "request_signature_bytes:",
            "proof_signature_bytes:",
            "capability_token:",
        ] {
            assert!(
                !source.contains(forbidden_runtime_pattern),
                "Phase 8C request-proof source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
