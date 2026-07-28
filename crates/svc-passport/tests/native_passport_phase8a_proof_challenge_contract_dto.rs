#[cfg(not(feature = "native-passport"))]
#[test]
fn phase8a_proof_challenge_contract_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native proof challenge DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_challenge_contract_posture,
        review_native_passport_challenge_contract_draft,
        validate_native_passport_challenge_contract_descriptor, B3DigestHex, ChallengeIdV1,
        DeviceIdV1, NativeChallengeSignatureAlgorithm, NativeChallengeTranscriptCodec,
        NativePassportChallengeContractDraftV1, NativePassportChallengeContractReviewError,
        NativePassportChallengePurpose, NativePassportScope, NativePassportSurface, PassportIdV1,
        NATIVE_PASSPORT_PHASE8A_LABEL, PHASE7C_ENABLED_SURFACES, PHASE8A_ALLOWED_CHALLENGE_SCOPES,
        PHASE8A_ENABLED_SURFACES, PHASE8A_FORBIDDEN_CHALLENGE_AUTHORITY_FLAGS,
        PHASE8A_MAX_CHALLENGE_TTL_MS, PHASE8A_MAX_CLOCK_SKEW_MS,
        PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN, PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
        PHASE8A_REQUIRED_SERVICE_SIGNATURE_ALGORITHM, PHASE8A_REQUIRED_TRANSCRIPT_CODEC,
    };

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn challenge_id() -> ChallengeIdV1 {
        ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_A}")).expect("challenge id")
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

    fn valid_draft() -> NativePassportChallengeContractDraftV1 {
        NativePassportChallengeContractDraftV1 {
            contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
            contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
            challenge_id: challenge_id(),
            network_id: "rustyonions-main",
            environment: "private-beta",
            audience: "svc-passport",
            issuing_service_id: "svc-passport",
            service_key_id: "service-key:v1:challenge",
            purpose: NativePassportChallengePurpose::IssueCapability,
            requested_scopes: vec![
                NativePassportScope::IdentityRead,
                NativePassportScope::CatalogRead,
                NativePassportScope::ContentRead,
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

    #[test]
    fn phase8a_label_and_posture_are_locked() {
        let posture = native_passport_challenge_contract_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE8A_LABEL,
            "NATIVE_PASSPORT_PHASE8A_PROOF_CHALLENGE_CONTRACT_DTO"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE8A_LABEL);
        assert!(posture.proof_challenge_contract_dtos_added);

        for authority_added in [
            posture.service_challenge_signing_added,
            posture.service_challenge_verification_added,
            posture.root_proof_signing_added,
            posture.device_proof_signing_added,
            posture.proof_signature_verification_added,
            posture.request_proof_runtime_added,
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
            "service_challenge_signing",
            "service_challenge_verification",
            "root_proof_signing",
            "device_proof_signing",
            "proof_signature_verification",
            "request_proof_signing",
            "request_proof_verification",
            "replay_store_mutation",
            "challenge_consumption",
            "capability_issuance",
            "json_byte_signing",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE8A_FORBIDDEN_CHALLENGE_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase8a_surfaces_extend_phase7c_without_back_mutating_it() {
        assert_eq!(
            PHASE8A_ENABLED_SURFACES.len(),
            PHASE7C_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE8A_ENABLED_SURFACES[..PHASE7C_ENABLED_SURFACES.len()],
            PHASE7C_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE8A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::PurposeBoundChallengeContractDto)
        );
        assert_eq!(
            PHASE7C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::DelegatedEnrollmentContractDto)
        );
    }

    #[test]
    fn phase8a_reviews_purpose_bound_challenge_draft_into_contract_only_descriptor() {
        let descriptor = review_native_passport_challenge_contract_draft(valid_draft())
            .expect("valid challenge contract should review");

        assert_eq!(
            descriptor.contract_domain,
            PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN
        );
        assert_eq!(
            descriptor.contract_version,
            PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION
        );
        assert_eq!(descriptor.network_id, "rustyonions-main");
        assert_eq!(descriptor.environment, "private-beta");
        assert_eq!(descriptor.audience, "svc-passport");
        assert_eq!(descriptor.issuing_service_id, "svc-passport");
        assert_eq!(descriptor.service_key_id, "service-key:v1:challenge");
        assert_eq!(
            descriptor.purpose,
            NativePassportChallengePurpose::IssueCapability
        );
        assert_eq!(
            descriptor.requested_scopes,
            vec![
                NativePassportScope::IdentityRead,
                NativePassportScope::CatalogRead,
                NativePassportScope::ContentRead,
            ]
        );
        assert!(descriptor.passport_id.is_some());
        assert!(descriptor.device_id.is_some());
        assert!(descriptor.operation_body_hash.is_some());
        assert_eq!(
            descriptor.transcript_codec,
            NativeChallengeTranscriptCodec::CanonicalB3V1
        );
        assert_eq!(
            descriptor.service_signature_algorithm,
            NativeChallengeSignatureAlgorithm::ServiceEd25519V1
        );
        assert!(descriptor.service_signature_placeholder_present);
        assert!(descriptor.contract_only);

        validate_native_passport_challenge_contract_descriptor(&descriptor)
            .expect("reviewed challenge descriptor should validate");
    }

    #[test]
    fn phase8a_purpose_scope_time_and_codec_constants_are_locked() {
        assert_eq!(PHASE8A_MAX_CHALLENGE_TTL_MS, 300_000);
        assert_eq!(PHASE8A_MAX_CLOCK_SKEW_MS, 30_000);
        assert_eq!(
            PHASE8A_REQUIRED_TRANSCRIPT_CODEC,
            NativeChallengeTranscriptCodec::CanonicalB3V1
        );
        assert_eq!(
            PHASE8A_REQUIRED_SERVICE_SIGNATURE_ALGORITHM,
            NativeChallengeSignatureAlgorithm::ServiceEd25519V1
        );
        assert_eq!(
            PHASE8A_ALLOWED_CHALLENGE_SCOPES,
            &[
                NativePassportScope::IdentityRead,
                NativePassportScope::CatalogRead,
                NativePassportScope::ContentRead,
                NativePassportScope::EntitlementRead,
                NativePassportScope::ReceiptsRead,
                NativePassportScope::ConfirmedRocRead,
                NativePassportScope::CapabilityRevokeSelf,
            ]
        );
    }

    #[test]
    fn phase8a_rejects_domain_version_empty_fields_reserved_wallet_and_binding_drift() {
        let mut bad_domain = valid_draft();
        bad_domain.contract_domain = "native-passport/proof-challenge-contract/v0";
        assert_eq!(
            review_native_passport_challenge_contract_draft(bad_domain),
            Err(NativePassportChallengeContractReviewError::ContractDomainMismatch)
        );

        let mut bad_version = valid_draft();
        bad_version.contract_version = 2;
        assert_eq!(
            review_native_passport_challenge_contract_draft(bad_version),
            Err(NativePassportChallengeContractReviewError::ContractVersionMismatch)
        );

        let mut empty_network = valid_draft();
        empty_network.network_id = "";
        assert_eq!(
            review_native_passport_challenge_contract_draft(empty_network),
            Err(NativePassportChallengeContractReviewError::EmptyField(
                "network_id"
            ))
        );

        let mut reserved_wallet = valid_draft();
        reserved_wallet.purpose =
            NativePassportChallengePurpose::WalletAuthorizationRequestReserved;
        assert_eq!(
            review_native_passport_challenge_contract_draft(reserved_wallet),
            Err(NativePassportChallengeContractReviewError::ReservedWalletPurpose)
        );

        let mut missing_passport = valid_draft();
        missing_passport.passport_id = None;
        assert_eq!(
            review_native_passport_challenge_contract_draft(missing_passport),
            Err(NativePassportChallengeContractReviewError::MissingPassportBinding)
        );

        let mut missing_device = valid_draft();
        missing_device.device_id = None;
        assert_eq!(
            review_native_passport_challenge_contract_draft(missing_device),
            Err(NativePassportChallengeContractReviewError::MissingDeviceBinding)
        );

        let mut missing_body = valid_draft();
        missing_body.operation_body_hash = None;
        assert_eq!(
            review_native_passport_challenge_contract_draft(missing_body),
            Err(NativePassportChallengeContractReviewError::MissingOperationBodyHash)
        );
    }

    #[test]
    fn phase8a_rejects_scope_time_codec_signature_and_non_contract_drift() {
        let mut empty_scopes = valid_draft();
        empty_scopes.requested_scopes.clear();
        assert_eq!(
            review_native_passport_challenge_contract_draft(empty_scopes),
            Err(NativePassportChallengeContractReviewError::EmptyRequestedScopes)
        );

        let mut duplicate_scope = valid_draft();
        duplicate_scope
            .requested_scopes
            .push(NativePassportScope::IdentityRead);
        assert_eq!(
            review_native_passport_challenge_contract_draft(duplicate_scope),
            Err(NativePassportChallengeContractReviewError::DuplicateRequestedScope)
        );

        let mut bad_window = valid_draft();
        bad_window.expires_at_ms = bad_window.issued_at_ms;
        assert_eq!(
            review_native_passport_challenge_contract_draft(bad_window),
            Err(NativePassportChallengeContractReviewError::InvalidExpiryWindow)
        );

        let mut too_long = valid_draft();
        too_long.expires_at_ms = too_long.issued_at_ms + PHASE8A_MAX_CHALLENGE_TTL_MS + 1;
        assert_eq!(
            review_native_passport_challenge_contract_draft(too_long),
            Err(NativePassportChallengeContractReviewError::ChallengeTtlTooLong)
        );

        let mut bad_skew = valid_draft();
        bad_skew.max_clock_skew_ms = PHASE8A_MAX_CLOCK_SKEW_MS + 1;
        assert_eq!(
            review_native_passport_challenge_contract_draft(bad_skew),
            Err(NativePassportChallengeContractReviewError::ClockSkewTooLong)
        );

        let mut json_codec = valid_draft();
        json_codec.transcript_codec = NativeChallengeTranscriptCodec::JsonBytesRejected;
        assert_eq!(
            review_native_passport_challenge_contract_draft(json_codec),
            Err(NativePassportChallengeContractReviewError::InvalidTranscriptCodec)
        );

        let mut missing_algorithm = valid_draft();
        missing_algorithm.service_signature_algorithm =
            NativeChallengeSignatureAlgorithm::MissingOrUnknown;
        assert_eq!(
            review_native_passport_challenge_contract_draft(missing_algorithm),
            Err(NativePassportChallengeContractReviewError::InvalidServiceSignatureAlgorithm)
        );

        let mut missing_signature = valid_draft();
        missing_signature.service_signature_placeholder_present = false;
        assert_eq!(
            review_native_passport_challenge_contract_draft(missing_signature),
            Err(NativePassportChallengeContractReviewError::MissingServiceSignaturePlaceholder)
        );

        let mut descriptor =
            review_native_passport_challenge_contract_draft(valid_draft()).expect("descriptor");
        descriptor.contract_only = false;
        assert_eq!(
            validate_native_passport_challenge_contract_descriptor(&descriptor),
            Err(NativePassportChallengeContractReviewError::ChallengeContractNotContractOnly)
        );
    }

    #[test]
    fn phase8a_accepts_session_purpose_without_operation_body_hash_but_with_passport_and_device() {
        let mut session = valid_draft();
        session.purpose = NativePassportChallengePurpose::ProveSession;
        session.operation_body_hash = None;

        let descriptor = review_native_passport_challenge_contract_draft(session)
            .expect("session proof can omit operation body hash");

        assert_eq!(
            descriptor.purpose,
            NativePassportChallengePurpose::ProveSession
        );
        assert!(descriptor.passport_id.is_some());
        assert!(descriptor.device_id.is_some());
        assert!(descriptor.operation_body_hash.is_none());
    }

    #[test]
    fn phase8a_rejects_all_unsafe_runtime_authority_flags() {
        let mutators: &[fn(&mut NativePassportChallengeContractDraftV1)] = &[
            |draft| draft.requests_service_challenge_signing = true,
            |draft| draft.requests_service_challenge_verification = true,
            |draft| draft.requests_root_proof_signing = true,
            |draft| draft.requests_device_proof_signing = true,
            |draft| draft.requests_proof_signature_verification = true,
            |draft| draft.requests_request_proof_signing = true,
            |draft| draft.requests_request_proof_verification = true,
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
            let mut draft = valid_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_passport_challenge_contract_draft(draft),
                Err(NativePassportChallengeContractReviewError::UnsafeChallengeAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase8a_source_remains_contract_only_without_signing_verification_routes_io_or_secret_material(
    ) {
        let source =
            fs::read_to_string(repo_file("src/native/challenge.rs")).expect("challenge source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativePassportChallengeContractDescriptorV1"));
        assert!(source.contains("NativePassportChallengeContractDraftV1"));
        assert!(source.contains("review_native_passport_challenge_contract_draft"));
        assert!(native_mod.contains("PurposeBoundChallengeContractDto"));
        assert!(!source.contains("serde_json"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "sign(",
            "verify_signature(",
            "verify_proof(",
            "consume_challenge(",
            "replay_store.write(",
            "issue_capability(",
            "refresh_capability(",
            "revoke_capability(",
            "execute_device_authorization(",
            "execute_device_revocation(",
            "execute_username_claim(",
            "execute_site_update(",
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
                "Phase 8A challenge source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
