#[cfg(not(feature = "native-passport"))]
#[test]
fn phase9a_proof_signing_boundary_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native proof-signing boundary DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_proof_signing_contract_boundary_posture,
        review_native_passport_challenge_contract_draft,
        review_native_passport_proof_contract_draft,
        review_native_passport_request_proof_contract_draft,
        review_native_proof_signing_contract_boundary, B3DigestHex, ChallengeIdV1, DeviceIdV1,
        NativePassportChallengeContractDescriptorV1, NativePassportChallengeContractDraftV1,
        NativePassportChallengePurpose, NativePassportProofAuthority,
        NativePassportProofContractDescriptorV1, NativePassportProofContractDraftV1,
        NativePassportProofKind, NativePassportRequestMethod,
        NativePassportRequestProofContractDescriptorV1, NativePassportRequestProofContractDraftV1,
        NativePassportScope, NativePassportSurface, NativeProofSignatureAlgorithm,
        NativeProofSigningContractBoundaryDraftV1, NativeProofSigningContractBoundaryReviewError,
        NativeProofSigningOperationKind, NativeProofSigningUnlockState, PassportIdV1,
        NATIVE_PASSPORT_PHASE9A_LABEL, PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
        PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION, PHASE8A_REQUIRED_SERVICE_SIGNATURE_ALGORITHM,
        PHASE8A_REQUIRED_TRANSCRIPT_CODEC, PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
        PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
        PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC, PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
        PHASE8C_ENABLED_SURFACES, PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN,
        PHASE8C_PROOF_REQUEST_CONTRACT_VERSION, PHASE8C_REQUIRED_REQUEST_TRANSCRIPT_CODEC,
        PHASE9A_ENABLED_SURFACES, PHASE9A_FORBIDDEN_PROOF_SIGNING_AUTHORITY_FLAGS,
        PHASE9A_PROOF_SIGNING_BOUNDARY_DOMAIN, PHASE9A_PROOF_SIGNING_BOUNDARY_VERSION,
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

    fn challenge_draft(
        purpose: NativePassportChallengePurpose,
    ) -> NativePassportChallengeContractDraftV1 {
        let operation_body_hash = match purpose {
            NativePassportChallengePurpose::ProveSession => None,
            _ => Some(digest("operation_body_hash", HEX_D)),
        };

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
            operation_body_hash,
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
            challenge_transcript_hash: digest("challenge_transcript_hash", HEX_E),
            proof_transcript_hash: digest("proof_transcript_hash", HEX_F),
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

    fn root_proof(
        challenge: &NativePassportChallengeContractDescriptorV1,
    ) -> NativePassportProofContractDescriptorV1 {
        review_native_passport_proof_contract_draft(
            challenge,
            proof_draft(
                challenge,
                NativePassportProofKind::RootRegistration,
                NativePassportProofAuthority::RootAuthority,
                PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
            ),
        )
        .expect("valid root proof")
    }

    fn device_proof(
        challenge: &NativePassportChallengeContractDescriptorV1,
    ) -> NativePassportProofContractDescriptorV1 {
        review_native_passport_proof_contract_draft(
            challenge,
            proof_draft(
                challenge,
                NativePassportProofKind::DeviceSession,
                NativePassportProofAuthority::DeviceAuthority,
                PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
            ),
        )
        .expect("valid device proof")
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

    fn request_proof(
        challenge: &NativePassportChallengeContractDescriptorV1,
        proof: &NativePassportProofContractDescriptorV1,
    ) -> NativePassportRequestProofContractDescriptorV1 {
        review_native_passport_request_proof_contract_draft(
            challenge,
            proof,
            request_draft(challenge, proof),
        )
        .expect("valid request proof")
    }

    fn proof_boundary_draft(
        proof: &NativePassportProofContractDescriptorV1,
        operation_kind: NativeProofSigningOperationKind,
        unlocked_authority: NativeProofSigningUnlockState,
    ) -> NativeProofSigningContractBoundaryDraftV1 {
        let requested_signature_algorithm = match proof.proof_authority {
            NativePassportProofAuthority::RootAuthority => PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
            NativePassportProofAuthority::DeviceAuthority => {
                PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM
            }
        };

        NativeProofSigningContractBoundaryDraftV1 {
            contract_domain: PHASE9A_PROOF_SIGNING_BOUNDARY_DOMAIN,
            contract_version: PHASE9A_PROOF_SIGNING_BOUNDARY_VERSION,
            operation_kind,
            unlocked_authority,
            challenge_id: proof.challenge_id.clone(),
            passport_id: proof.passport_id.clone(),
            device_id: proof.device_id.clone(),
            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            request_contract_domain: None,
            request_contract_version: None,
            proof_kind: proof.proof_kind,
            proof_authority: proof.proof_authority,
            transcript_hash: proof.proof_transcript_hash.clone(),
            signer_public_key_label: "public-key:v1:test-signer",
            requested_signature_algorithm,
            signing_requested_at_ms: 1_035_000,
            requests_root_proof_signing_runtime: false,
            requests_device_proof_signing_runtime: false,
            requests_request_proof_signing_runtime: false,
            emits_signed_payload: false,
            requests_secret_key_access: false,
            requests_key_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            includes_platform_sealer_implementation: false,
            requests_proof_signature_verification: false,
            requests_request_proof_verification: false,
            requests_service_challenge_verification: false,
            requests_replay_store_mutation: false,
            requests_challenge_consumption: false,
            requests_capability_consumption: false,
            requests_capability_issuance: false,
            requests_runtime_io: false,
            adds_routes: false,
            requests_storage_mutation: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    fn request_boundary_draft(
        request: &NativePassportRequestProofContractDescriptorV1,
    ) -> NativeProofSigningContractBoundaryDraftV1 {
        NativeProofSigningContractBoundaryDraftV1 {
            contract_domain: PHASE9A_PROOF_SIGNING_BOUNDARY_DOMAIN,
            contract_version: PHASE9A_PROOF_SIGNING_BOUNDARY_VERSION,
            operation_kind: NativeProofSigningOperationKind::RequestProof,
            unlocked_authority: NativeProofSigningUnlockState::DeviceUnlocked,
            challenge_id: request.challenge_id.clone(),
            passport_id: request.passport_id.clone(),
            device_id: request.device_id.clone(),
            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            request_contract_domain: Some(PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN),
            request_contract_version: Some(PHASE8C_PROOF_REQUEST_CONTRACT_VERSION),
            proof_kind: request.proof_kind,
            proof_authority: request.proof_authority,
            transcript_hash: request.request_transcript_hash.clone(),
            signer_public_key_label: "public-key:v1:test-device",
            requested_signature_algorithm: PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
            signing_requested_at_ms: 1_045_000,
            requests_root_proof_signing_runtime: false,
            requests_device_proof_signing_runtime: false,
            requests_request_proof_signing_runtime: false,
            emits_signed_payload: false,
            requests_secret_key_access: false,
            requests_key_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            includes_platform_sealer_implementation: false,
            requests_proof_signature_verification: false,
            requests_request_proof_verification: false,
            requests_service_challenge_verification: false,
            requests_replay_store_mutation: false,
            requests_challenge_consumption: false,
            requests_capability_consumption: false,
            requests_capability_issuance: false,
            requests_runtime_io: false,
            adds_routes: false,
            requests_storage_mutation: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase9a_label_and_posture_are_locked() {
        let posture = native_proof_signing_contract_boundary_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE9A_LABEL,
            "NATIVE_PASSPORT_PHASE9A_NATIVE_PROOF_SIGNING_CONTRACT_BOUNDARY"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE9A_LABEL);
        assert!(posture.proof_signing_contract_boundary_added);
        assert!(!posture.root_proof_signing_runtime_added);
        assert!(!posture.device_proof_signing_runtime_added);
        assert!(!posture.request_proof_signing_runtime_added);
        assert!(!posture.signed_payload_export_added);
        assert!(!posture.secret_key_access_added);
        assert!(!posture.vault_unlock_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "root_proof_signing_runtime",
            "device_proof_signing_runtime",
            "request_proof_signing_runtime",
            "signature_bytes_export",
            "secret_key_access",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE9A_FORBIDDEN_PROOF_SIGNING_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase9a_surfaces_extend_phase8c_without_back_mutating_it() {
        assert_eq!(
            PHASE9A_ENABLED_SURFACES.len(),
            PHASE8C_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE9A_ENABLED_SURFACES[..PHASE8C_ENABLED_SURFACES.len()],
            PHASE8C_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE9A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ProofSigningContractBoundaryDto)
        );
        assert_eq!(
            PHASE8C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::RequestProofContractDto)
        );
    }

    #[test]
    fn phase9a_accepts_root_proof_boundary_without_signature_runtime() {
        let challenge = challenge(NativePassportChallengePurpose::RegisterRoot);
        let proof = root_proof(&challenge);
        let decision = review_native_proof_signing_contract_boundary(
            &challenge,
            &proof,
            None,
            proof_boundary_draft(
                &proof,
                NativeProofSigningOperationKind::RootProof,
                NativeProofSigningUnlockState::RootUnlocked,
            ),
        )
        .expect("root proof signing boundary should review");

        assert_eq!(
            decision.operation_kind,
            NativeProofSigningOperationKind::RootProof
        );
        assert_eq!(
            decision.accepted_authority,
            NativePassportProofAuthority::RootAuthority
        );
        assert_eq!(decision.transcript_hash, proof.proof_transcript_hash);
        assert_eq!(
            decision.requested_signature_algorithm,
            NativeProofSignatureAlgorithm::RootEd25519V1
        );
        assert!(!decision.signing_runtime_enabled);
        assert!(!decision.signed_payload_present);
        assert!(decision.contract_only);
    }

    #[test]
    fn phase9a_accepts_request_proof_boundary_without_signature_runtime() {
        let challenge = challenge(NativePassportChallengePurpose::ProveSession);
        let proof = device_proof(&challenge);
        let request = request_proof(&challenge, &proof);

        let decision = review_native_proof_signing_contract_boundary(
            &challenge,
            &proof,
            Some(&request),
            request_boundary_draft(&request),
        )
        .expect("request proof signing boundary should review");

        assert_eq!(
            decision.operation_kind,
            NativeProofSigningOperationKind::RequestProof
        );
        assert_eq!(
            decision.accepted_authority,
            NativePassportProofAuthority::DeviceAuthority
        );
        assert_eq!(decision.transcript_hash, request.request_transcript_hash);
        assert_eq!(
            decision.request_contract_domain,
            Some(PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN)
        );
        assert_eq!(
            decision.requested_signature_algorithm,
            NativeProofSignatureAlgorithm::DeviceEd25519V1
        );
        assert!(!decision.signing_runtime_enabled);
        assert!(!decision.signed_payload_present);
        assert!(decision.contract_only);
    }

    #[test]
    fn phase9a_rejects_locked_wrong_unlock_algorithm_and_reference_drift() {
        let challenge = challenge(NativePassportChallengePurpose::RegisterRoot);
        let proof = root_proof(&challenge);

        let locked = proof_boundary_draft(
            &proof,
            NativeProofSigningOperationKind::RootProof,
            NativeProofSigningUnlockState::Locked,
        );
        assert_eq!(
            review_native_proof_signing_contract_boundary(&challenge, &proof, None, locked),
            Err(NativeProofSigningContractBoundaryReviewError::LockedStateDenied)
        );

        let wrong_unlock = proof_boundary_draft(
            &proof,
            NativeProofSigningOperationKind::RootProof,
            NativeProofSigningUnlockState::DeviceUnlocked,
        );
        assert_eq!(
            review_native_proof_signing_contract_boundary(&challenge, &proof, None, wrong_unlock),
            Err(NativeProofSigningContractBoundaryReviewError::RootUnlockRequired)
        );

        let mut wrong_algorithm = proof_boundary_draft(
            &proof,
            NativeProofSigningOperationKind::RootProof,
            NativeProofSigningUnlockState::RootUnlocked,
        );
        wrong_algorithm.requested_signature_algorithm =
            NativeProofSignatureAlgorithm::DeviceEd25519V1;
        assert_eq!(
            review_native_proof_signing_contract_boundary(
                &challenge,
                &proof,
                None,
                wrong_algorithm,
            ),
            Err(NativeProofSigningContractBoundaryReviewError::SignatureAlgorithmMismatch)
        );

        let mut wrong_challenge = proof_boundary_draft(
            &proof,
            NativeProofSigningOperationKind::RootProof,
            NativeProofSigningUnlockState::RootUnlocked,
        );
        wrong_challenge.challenge_id = other_challenge_id();
        assert_eq!(
            review_native_proof_signing_contract_boundary(
                &challenge,
                &proof,
                None,
                wrong_challenge,
            ),
            Err(NativeProofSigningContractBoundaryReviewError::ChallengeIdMismatch)
        );

        let mut wrong_hash = proof_boundary_draft(
            &proof,
            NativeProofSigningOperationKind::RootProof,
            NativeProofSigningUnlockState::RootUnlocked,
        );
        wrong_hash.transcript_hash = digest("wrong_transcript_hash", HEX_D);
        assert_eq!(
            review_native_proof_signing_contract_boundary(&challenge, &proof, None, wrong_hash),
            Err(NativeProofSigningContractBoundaryReviewError::TranscriptHashMismatch)
        );
    }

    #[test]
    fn phase9a_rejects_request_reference_and_time_drift() {
        let challenge = challenge(NativePassportChallengePurpose::ProveSession);
        let proof = device_proof(&challenge);
        let request = request_proof(&challenge, &proof);

        let missing_request = proof_boundary_draft(
            &proof,
            NativeProofSigningOperationKind::RequestProof,
            NativeProofSigningUnlockState::DeviceUnlocked,
        );
        assert_eq!(
            review_native_proof_signing_contract_boundary(
                &challenge,
                &proof,
                None,
                missing_request,
            ),
            Err(NativeProofSigningContractBoundaryReviewError::RequestProofReferenceRequired)
        );

        let unexpected_request = proof_boundary_draft(
            &proof,
            NativeProofSigningOperationKind::DeviceProof,
            NativeProofSigningUnlockState::DeviceUnlocked,
        );
        assert_eq!(
            review_native_proof_signing_contract_boundary(
                &challenge,
                &proof,
                Some(&request),
                unexpected_request,
            ),
            Err(NativeProofSigningContractBoundaryReviewError::RequestProofReferenceUnexpected)
        );

        let mut bad_request_domain = request_boundary_draft(&request);
        bad_request_domain.request_contract_domain =
            Some("native-passport/proof-request-contract/v0");
        assert_eq!(
            review_native_proof_signing_contract_boundary(
                &challenge,
                &proof,
                Some(&request),
                bad_request_domain,
            ),
            Err(NativeProofSigningContractBoundaryReviewError::RequestProofContractDomainMismatch)
        );

        let mut bad_time = request_boundary_draft(&request);
        bad_time.signing_requested_at_ms = request.request_expires_at_ms + 1;
        assert_eq!(
            review_native_proof_signing_contract_boundary(
                &challenge,
                &proof,
                Some(&request),
                bad_time,
            ),
            Err(NativeProofSigningContractBoundaryReviewError::InvalidSigningRequestedAt)
        );
    }

    #[test]
    fn phase9a_rejects_all_unsafe_runtime_authority_flags() {
        let challenge = challenge(NativePassportChallengePurpose::ProveSession);
        let proof = device_proof(&challenge);

        let mutators: &[fn(&mut NativeProofSigningContractBoundaryDraftV1)] = &[
            |draft| draft.requests_root_proof_signing_runtime = true,
            |draft| draft.requests_device_proof_signing_runtime = true,
            |draft| draft.requests_request_proof_signing_runtime = true,
            |draft| draft.emits_signed_payload = true,
            |draft| draft.requests_secret_key_access = true,
            |draft| draft.requests_key_derivation_runtime = true,
            |draft| draft.requests_vault_unlock = true,
            |draft| draft.includes_vault_runtime = true,
            |draft| draft.includes_platform_sealer_implementation = true,
            |draft| draft.requests_proof_signature_verification = true,
            |draft| draft.requests_request_proof_verification = true,
            |draft| draft.requests_service_challenge_verification = true,
            |draft| draft.requests_replay_store_mutation = true,
            |draft| draft.requests_challenge_consumption = true,
            |draft| draft.requests_capability_consumption = true,
            |draft| draft.requests_capability_issuance = true,
            |draft| draft.requests_runtime_io = true,
            |draft| draft.adds_routes = true,
            |draft| draft.requests_storage_mutation = true,
            |draft| draft.requests_wallet_or_ledger_mutation = true,
        ];

        for mutate in mutators {
            let mut draft = proof_boundary_draft(
                &proof,
                NativeProofSigningOperationKind::DeviceProof,
                NativeProofSigningUnlockState::DeviceUnlocked,
            );
            mutate(&mut draft);
            assert_eq!(
                review_native_proof_signing_contract_boundary(&challenge, &proof, None, draft),
                Err(NativeProofSigningContractBoundaryReviewError::UnsafeSigningAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase9a_source_remains_contract_boundary_without_signature_key_vault_io_or_mutation() {
        let source = fs::read_to_string(repo_file("src/native/proof_signing.rs"))
            .expect("proof signing boundary source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeProofSigningContractBoundaryDraftV1"));
        assert!(source.contains("NativeProofSigningContractBoundaryDecisionV1"));
        assert!(source.contains("review_native_proof_signing_contract_boundary"));
        assert!(native_mod.contains("ProofSigningContractBoundaryDto"));
        assert!(!source.contains("serde_json"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "sign(",
            "verify_signature(",
            "verify_proof(",
            "verify_request_proof(",
            "load_secret",
            "load_key",
            "secret_key_bytes",
            "signature_bytes:",
            "consume_challenge(",
            "consume_capability(",
            "replay_store.write(",
            "issue_capability(",
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
                "Phase 9A proof-signing boundary source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
