#[cfg(not(feature = "native-passport"))]
#[test]
fn phase9b_proof_signing_adapter_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native proof-signing adapter surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        execute_native_proof_signing_adapter, native_proof_signing_adapter_posture,
        review_native_passport_challenge_contract_draft,
        review_native_passport_proof_contract_draft,
        review_native_passport_request_proof_contract_draft,
        review_native_proof_signing_contract_boundary, B3DigestHex, ChallengeIdV1, DeviceIdV1,
        NativeLocalProofSigningAdapter, NativePassportChallengeContractDescriptorV1,
        NativePassportChallengeContractDraftV1, NativePassportChallengePurpose,
        NativePassportProofAuthority, NativePassportProofContractDescriptorV1,
        NativePassportProofContractDraftV1, NativePassportProofKind, NativePassportRequestMethod,
        NativePassportRequestProofContractDescriptorV1, NativePassportRequestProofContractDraftV1,
        NativePassportScope, NativePassportSurface, NativeProofSignatureAlgorithm,
        NativeProofSignedPayloadHex, NativeProofSigningAdapterDraftV1,
        NativeProofSigningAdapterRequestV1, NativeProofSigningAdapterReviewError,
        NativeProofSigningContractBoundaryDecisionV1, NativeProofSigningContractBoundaryDraftV1,
        NativeProofSigningOperationKind, NativeProofSigningUnlockState, PassportIdV1,
        NATIVE_PASSPORT_PHASE9B_LABEL, PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
        PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION, PHASE8A_REQUIRED_SERVICE_SIGNATURE_ALGORITHM,
        PHASE8A_REQUIRED_TRANSCRIPT_CODEC, PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
        PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
        PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC, PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
        PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN, PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
        PHASE8C_REQUIRED_REQUEST_TRANSCRIPT_CODEC, PHASE9A_ENABLED_SURFACES,
        PHASE9A_PROOF_SIGNING_BOUNDARY_DOMAIN, PHASE9A_PROOF_SIGNING_BOUNDARY_VERSION,
        PHASE9B_ENABLED_SURFACES, PHASE9B_FORBIDDEN_SIGNING_ADAPTER_AUTHORITY_FLAGS,
        PHASE9B_PROOF_SIGNING_ADAPTER_DOMAIN, PHASE9B_PROOF_SIGNING_ADAPTER_VERSION,
    };

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const HEX_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    struct FixedLocalSigner {
        reject: bool,
    }

    impl NativeLocalProofSigningAdapter for FixedLocalSigner {
        fn produce_signed_payload(
            &self,
            request: &NativeProofSigningAdapterRequestV1,
        ) -> Result<NativeProofSignedPayloadHex, NativeProofSigningAdapterReviewError> {
            if self.reject {
                return Err(NativeProofSigningAdapterReviewError::SignerRejected);
            }

            assert!(request.contract_only);
            assert!(!request.signer_public_key_label.is_empty());

            NativeProofSignedPayloadHex::parse("test_signed_payload_hex", "11".repeat(64))
        }
    }

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn digest(label: &'static str, hex: &'static str) -> B3DigestHex {
        B3DigestHex::parse(label, hex).expect(label)
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

    fn boundary_draft(
        proof: &NativePassportProofContractDescriptorV1,
        operation_kind: NativeProofSigningOperationKind,
        unlock: NativeProofSigningUnlockState,
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
            unlocked_authority: unlock,
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

    fn root_boundary_decision() -> NativeProofSigningContractBoundaryDecisionV1 {
        let challenge = challenge(NativePassportChallengePurpose::RegisterRoot);
        let proof = root_proof(&challenge);

        review_native_proof_signing_contract_boundary(
            &challenge,
            &proof,
            None,
            boundary_draft(
                &proof,
                NativeProofSigningOperationKind::RootProof,
                NativeProofSigningUnlockState::RootUnlocked,
            ),
        )
        .expect("root boundary decision")
    }

    fn request_boundary_decision() -> NativeProofSigningContractBoundaryDecisionV1 {
        let challenge = challenge(NativePassportChallengePurpose::ProveSession);
        let proof = device_proof(&challenge);
        let request = request_proof(&challenge, &proof);

        review_native_proof_signing_contract_boundary(
            &challenge,
            &proof,
            Some(&request),
            request_boundary_draft(&request),
        )
        .expect("request boundary decision")
    }

    fn adapter_draft(
        decision: &NativeProofSigningContractBoundaryDecisionV1,
    ) -> NativeProofSigningAdapterDraftV1 {
        NativeProofSigningAdapterDraftV1 {
            contract_domain: PHASE9B_PROOF_SIGNING_ADAPTER_DOMAIN,
            contract_version: PHASE9B_PROOF_SIGNING_ADAPTER_VERSION,
            operation_kind: decision.operation_kind,
            signer_authority: decision.accepted_authority,
            local_signer_unlocked: true,
            challenge_id: decision.challenge_id.clone(),
            passport_id: decision.passport_id.clone(),
            device_id: decision.device_id.clone(),
            proof_contract_domain: decision.proof_contract_domain,
            proof_contract_version: decision.proof_contract_version,
            request_contract_domain: decision.request_contract_domain,
            request_contract_version: decision.request_contract_version,
            proof_kind: decision.proof_kind,
            transcript_hash: decision.transcript_hash.clone(),
            signer_public_key_label: decision.signer_public_key_label,
            requested_signature_algorithm: decision.requested_signature_algorithm,
            signing_requested_at_ms: decision.signing_requested_at_ms,
            allows_injected_local_signer: true,
            requests_secret_key_access: false,
            requests_key_loading: false,
            requests_key_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            requests_platform_sealer_unseal: false,
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
    fn phase9b_label_and_posture_are_locked() {
        let posture = native_proof_signing_adapter_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE9B_LABEL,
            "NATIVE_PASSPORT_PHASE9B_NATIVE_PROOF_SIGNING_ADAPTER"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE9B_LABEL);
        assert!(posture.proof_signing_adapter_added);
        assert!(posture.root_proof_signing_adapter_added);
        assert!(posture.device_proof_signing_adapter_added);
        assert!(posture.request_proof_signing_adapter_added);
        assert!(!posture.secret_key_access_added);
        assert!(!posture.key_loading_added);
        assert!(!posture.vault_unlock_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "secret_key_access",
            "key_loading",
            "vault_unlock",
            "runtime_io",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE9B_FORBIDDEN_SIGNING_ADAPTER_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase9b_surfaces_extend_phase9a_without_back_mutating_it() {
        assert_eq!(
            PHASE9B_ENABLED_SURFACES.len(),
            PHASE9A_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE9B_ENABLED_SURFACES[..PHASE9A_ENABLED_SURFACES.len()],
            PHASE9A_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE9B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ProofSigningAdapterDto)
        );
        assert_eq!(
            PHASE9A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ProofSigningContractBoundaryDto)
        );
    }

    #[test]
    fn phase9b_executes_root_adapter_from_accepted_boundary_decision() {
        let decision = root_boundary_decision();
        let envelope = execute_native_proof_signing_adapter(
            &decision,
            adapter_draft(&decision),
            &FixedLocalSigner { reject: false },
        )
        .expect("root adapter should produce signed payload envelope");

        assert_eq!(
            envelope.contract_domain,
            PHASE9B_PROOF_SIGNING_ADAPTER_DOMAIN
        );
        assert_eq!(
            envelope.contract_version,
            PHASE9B_PROOF_SIGNING_ADAPTER_VERSION
        );
        assert_eq!(
            envelope.operation_kind,
            NativeProofSigningOperationKind::RootProof
        );
        assert_eq!(
            envelope.signer_authority,
            NativePassportProofAuthority::RootAuthority
        );
        assert_eq!(
            envelope.requested_signature_algorithm,
            NativeProofSignatureAlgorithm::RootEd25519V1
        );
        assert_eq!(envelope.transcript_hash, decision.transcript_hash);
        assert!(envelope.signed_payload_present);
        assert_eq!(envelope.signed_payload_hex.as_str().len(), 128);
        assert!(!envelope.secret_material_exposed);
        assert!(!envelope.runtime_io_performed);
        assert!(!envelope.wallet_or_ledger_mutated);
    }

    #[test]
    fn phase9b_executes_request_adapter_from_accepted_boundary_decision() {
        let decision = request_boundary_decision();
        let envelope = execute_native_proof_signing_adapter(
            &decision,
            adapter_draft(&decision),
            &FixedLocalSigner { reject: false },
        )
        .expect("request adapter should produce signed payload envelope");

        assert_eq!(
            envelope.operation_kind,
            NativeProofSigningOperationKind::RequestProof
        );
        assert_eq!(
            envelope.signer_authority,
            NativePassportProofAuthority::DeviceAuthority
        );
        assert_eq!(
            envelope.request_contract_domain,
            Some(PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN)
        );
        assert_eq!(
            envelope.requested_signature_algorithm,
            NativeProofSignatureAlgorithm::DeviceEd25519V1
        );
        assert_eq!(envelope.transcript_hash, decision.transcript_hash);
        assert!(envelope.signed_payload_present);
        assert!(!envelope.secret_material_exposed);
        assert!(!envelope.runtime_io_performed);
        assert!(!envelope.wallet_or_ledger_mutated);
    }

    #[test]
    fn phase9b_rejects_decision_drift_locked_signer_mismatch_and_bad_payload() {
        let decision = root_boundary_decision();

        let mut locked = adapter_draft(&decision);
        locked.local_signer_unlocked = false;
        assert_eq!(
            execute_native_proof_signing_adapter(
                &decision,
                locked,
                &FixedLocalSigner { reject: false },
            ),
            Err(NativeProofSigningAdapterReviewError::LocalSignerLocked)
        );

        let mut wrong_domain = adapter_draft(&decision);
        wrong_domain.contract_domain = "native-passport/proof-signing-adapter/v0";
        assert_eq!(
            execute_native_proof_signing_adapter(
                &decision,
                wrong_domain,
                &FixedLocalSigner { reject: false },
            ),
            Err(NativeProofSigningAdapterReviewError::ContractDomainMismatch)
        );

        let mut wrong_hash = adapter_draft(&decision);
        wrong_hash.transcript_hash = digest("wrong_transcript_hash", HEX_D);
        assert_eq!(
            execute_native_proof_signing_adapter(
                &decision,
                wrong_hash,
                &FixedLocalSigner { reject: false },
            ),
            Err(NativeProofSigningAdapterReviewError::TranscriptHashMismatch)
        );

        let mut wrong_label = adapter_draft(&decision);
        wrong_label.signer_public_key_label = "public-key:v1:wrong";
        assert_eq!(
            execute_native_proof_signing_adapter(
                &decision,
                wrong_label,
                &FixedLocalSigner { reject: false },
            ),
            Err(NativeProofSigningAdapterReviewError::SignerPublicKeyLabelMismatch)
        );

        assert_eq!(
            execute_native_proof_signing_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedLocalSigner { reject: true },
            ),
            Err(NativeProofSigningAdapterReviewError::SignerRejected)
        );

        assert_eq!(
            NativeProofSignedPayloadHex::parse("bad_payload", "not-hex"),
            Err(NativeProofSigningAdapterReviewError::InvalidSignedPayload(
                "bad_payload"
            ))
        );
    }

    #[test]
    fn phase9b_rejects_all_unsafe_adapter_authority_flags() {
        let decision = request_boundary_decision();

        let mutators: &[fn(&mut NativeProofSigningAdapterDraftV1)] = &[
            |draft| draft.requests_secret_key_access = true,
            |draft| draft.requests_key_loading = true,
            |draft| draft.requests_key_derivation_runtime = true,
            |draft| draft.requests_vault_unlock = true,
            |draft| draft.includes_vault_runtime = true,
            |draft| draft.requests_platform_sealer_unseal = true,
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
            let mut draft = adapter_draft(&decision);
            mutate(&mut draft);
            assert_eq!(
                execute_native_proof_signing_adapter(
                    &decision,
                    draft,
                    &FixedLocalSigner { reject: false },
                ),
                Err(NativeProofSigningAdapterReviewError::UnsafeAdapterAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase9b_source_remains_adapter_only_without_key_vault_io_verification_or_mutation() {
        let source = fs::read_to_string(repo_file("src/native/proof_signing_adapter.rs"))
            .expect("proof signing adapter source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeLocalProofSigningAdapter"));
        assert!(source.contains("execute_native_proof_signing_adapter"));
        assert!(source.contains("NativeProofSigningAdapterEnvelopeV1"));
        assert!(native_mod.contains("ProofSigningAdapterDto"));
        assert!(!source.contains("serde_json"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "verify_signature(",
            "verify_proof(",
            "verify_request_proof(",
            "load_secret",
            "load_key",
            "secret_key_bytes",
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
                "Phase 9B proof-signing adapter source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
