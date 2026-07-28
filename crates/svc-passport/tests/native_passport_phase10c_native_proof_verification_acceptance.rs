#[cfg(not(feature = "native-passport"))]
#[test]
fn phase10c_acceptance_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native Passport Phase 10 acceptance surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        execute_native_proof_verification_adapter, native_proof_verification_adapter_posture,
        native_proof_verification_contract_boundary_posture, B3DigestHex, ChallengeIdV1,
        DeviceIdV1, NativeLocalProofVerificationAdapter, NativePassportProofAuthority,
        NativePassportProofKind, NativePassportSurface, NativeProofSignatureAlgorithm,
        NativeProofSignedPayloadHex, NativeProofSigningOperationKind,
        NativeProofVerificationAdapterDraftV1, NativeProofVerificationAdapterEvidenceV1,
        NativeProofVerificationAdapterRequestV1, NativeProofVerificationAdapterReviewError,
        NativeProofVerificationContractBoundaryDecisionV1, PassportIdV1, PHASE10A_ENABLED_SURFACES,
        PHASE10A_PROOF_VERIFICATION_BOUNDARY_DOMAIN, PHASE10A_PROOF_VERIFICATION_BOUNDARY_VERSION,
        PHASE10B_ENABLED_SURFACES, PHASE10B_PROOF_VERIFICATION_ADAPTER_DOMAIN,
        PHASE10B_PROOF_VERIFICATION_ADAPTER_VERSION, PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
        PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
        PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM, PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN,
        PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
    };

    const PHASE10C_ACCEPTANCE_LABEL: &str =
        "NATIVE_PASSPORT_PHASE10C_NATIVE_PROOF_VERIFICATION_ACCEPTANCE";

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const HEX_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    struct AcceptanceVerifier {
        reject: bool,
        unsafe_evidence: bool,
        mismatch: bool,
    }

    impl NativeLocalProofVerificationAdapter for AcceptanceVerifier {
        fn assess_signed_payload(
            &self,
            request: &NativeProofVerificationAdapterRequestV1,
        ) -> Result<
            NativeProofVerificationAdapterEvidenceV1,
            NativeProofVerificationAdapterReviewError,
        > {
            assert!(request.contract_only);
            assert!(!request.signer_public_key_label.is_empty());
            assert!(request.verification_requested_at_ms >= 1_035_001);

            let transcript_hash = if self.mismatch {
                digest("mismatched_transcript_hash", HEX_F)
            } else {
                request.transcript_hash.clone()
            };

            Ok(NativeProofVerificationAdapterEvidenceV1 {
                accepted: !self.reject,
                evidence_label: "phase10c-acceptance-evidence",
                verifier_public_key_label: "public-key:v1:phase10c-verifier",
                transcript_hash,
                signed_payload_hex: request.signed_payload_hex.clone(),
                public_key_only: !self.unsafe_evidence,
                secret_material_exposed: self.unsafe_evidence,
                runtime_io_performed: false,
                wallet_or_ledger_mutated: false,
            })
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

    fn other_challenge_id() -> ChallengeIdV1 {
        ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_F}")).expect("other challenge id")
    }

    fn passport_id() -> PassportIdV1 {
        PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_B}")).expect("passport id")
    }

    fn device_id() -> DeviceIdV1 {
        DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}")).expect("device id")
    }

    fn signed_payload(hex_pair: &'static str) -> NativeProofSignedPayloadHex {
        NativeProofSignedPayloadHex::parse("phase10c_signed_payload_hex", hex_pair.repeat(64))
            .expect("signed payload")
    }

    fn verification_decision(
        operation_kind: NativeProofSigningOperationKind,
        authority: NativePassportProofAuthority,
        proof_kind: NativePassportProofKind,
        transcript_hash: B3DigestHex,
        signer_public_key_label: &'static str,
        algorithm: NativeProofSignatureAlgorithm,
        payload_pair: &'static str,
        request_contract_domain: Option<&'static str>,
        request_contract_version: Option<u16>,
        verification_requested_at_ms: u64,
    ) -> NativeProofVerificationContractBoundaryDecisionV1 {
        NativeProofVerificationContractBoundaryDecisionV1 {
            contract_domain: PHASE10A_PROOF_VERIFICATION_BOUNDARY_DOMAIN,
            contract_version: PHASE10A_PROOF_VERIFICATION_BOUNDARY_VERSION,
            operation_kind,
            reviewed_authority: authority,
            challenge_id: challenge_id(),
            passport_id: Some(passport_id()),
            device_id: Some(device_id()),
            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            request_contract_domain,
            request_contract_version,
            proof_kind,
            transcript_hash,
            signer_public_key_label,
            expected_signature_algorithm: algorithm,
            signed_payload_hex: signed_payload(payload_pair),
            verification_requested_at_ms,
            cryptographic_verification_performed: false,
            replay_state_mutated: false,
            challenge_consumed: false,
            capability_state_changed: false,
            runtime_io_performed: false,
            wallet_or_ledger_mutated: false,
            contract_only: true,
        }
    }

    fn root_decision() -> NativeProofVerificationContractBoundaryDecisionV1 {
        verification_decision(
            NativeProofSigningOperationKind::RootProof,
            NativePassportProofAuthority::RootAuthority,
            NativePassportProofKind::RootRegistration,
            digest("proof_transcript_hash", HEX_D),
            "public-key:v1:phase10c-root",
            PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
            "77",
            None,
            None,
            1_035_001,
        )
    }

    fn device_decision() -> NativeProofVerificationContractBoundaryDecisionV1 {
        verification_decision(
            NativeProofSigningOperationKind::DeviceProof,
            NativePassportProofAuthority::DeviceAuthority,
            NativePassportProofKind::DeviceSession,
            digest("proof_transcript_hash", HEX_E),
            "public-key:v1:phase10c-device",
            PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
            "88",
            None,
            None,
            1_035_001,
        )
    }

    fn request_decision() -> NativeProofVerificationContractBoundaryDecisionV1 {
        verification_decision(
            NativeProofSigningOperationKind::RequestProof,
            NativePassportProofAuthority::DeviceAuthority,
            NativePassportProofKind::DeviceSession,
            digest("request_transcript_hash", HEX_E),
            "public-key:v1:phase10c-request",
            PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
            "99",
            Some(PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN),
            Some(PHASE8C_PROOF_REQUEST_CONTRACT_VERSION),
            1_045_001,
        )
    }

    fn adapter_draft(
        decision: &NativeProofVerificationContractBoundaryDecisionV1,
    ) -> NativeProofVerificationAdapterDraftV1 {
        NativeProofVerificationAdapterDraftV1 {
            contract_domain: PHASE10B_PROOF_VERIFICATION_ADAPTER_DOMAIN,
            contract_version: PHASE10B_PROOF_VERIFICATION_ADAPTER_VERSION,
            operation_kind: decision.operation_kind,
            expected_authority: decision.reviewed_authority,
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
            expected_signature_algorithm: decision.expected_signature_algorithm,
            signed_payload_hex: decision.signed_payload_hex.clone(),
            verification_requested_at_ms: decision.verification_requested_at_ms,
            allows_injected_local_verifier: true,
            requests_secret_key_access: false,
            requests_key_loading: false,
            requests_key_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            requests_platform_sealer_unseal: false,
            requests_replay_store_mutation: false,
            requests_challenge_consumption: false,
            requests_capability_consumption: false,
            requests_capability_issuance: false,
            requests_capability_lifecycle_runtime: false,
            requests_runtime_io: false,
            adds_routes: false,
            requests_storage_mutation: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase10c_acceptance_label_is_locked() {
        assert_eq!(
            PHASE10C_ACCEPTANCE_LABEL,
            "NATIVE_PASSPORT_PHASE10C_NATIVE_PROOF_VERIFICATION_ACCEPTANCE"
        );
    }

    #[test]
    fn phase10c_accepts_phase10_surfaces_and_postures() {
        let boundary_posture = native_proof_verification_contract_boundary_posture();
        let adapter_posture = native_proof_verification_adapter_posture();

        assert_eq!(
            PHASE10B_ENABLED_SURFACES.len(),
            PHASE10A_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE10B_ENABLED_SURFACES[..PHASE10A_ENABLED_SURFACES.len()],
            PHASE10A_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE10B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ProofVerificationAdapterDto)
        );

        assert!(boundary_posture.proof_verification_contract_boundary_added);
        assert!(adapter_posture.proof_verification_adapter_added);
        assert!(adapter_posture.injected_local_verifier_adapter_added);

        assert!(!boundary_posture.replay_store_mutation_added);
        assert!(!boundary_posture.challenge_consumption_added);
        assert!(!boundary_posture.capability_issuance_added);
        assert!(!boundary_posture.runtime_io_added);
        assert!(!boundary_posture.wallet_or_ledger_mutation_added);
        assert!(!adapter_posture.replay_store_mutation_added);
        assert!(!adapter_posture.challenge_consumption_added);
        assert!(!adapter_posture.capability_issuance_added);
        assert!(!adapter_posture.runtime_io_added);
        assert!(!adapter_posture.wallet_or_ledger_mutation_added);
    }

    #[test]
    fn phase10c_accepts_root_device_and_request_verification_flow_through_injected_adapter() {
        for decision in [root_decision(), device_decision(), request_decision()] {
            let envelope = execute_native_proof_verification_adapter(
                &decision,
                adapter_draft(&decision),
                &AcceptanceVerifier {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                },
            )
            .expect("accepted boundary decision should produce verification envelope");

            assert_eq!(envelope.transcript_hash, decision.transcript_hash);
            assert_eq!(
                envelope.signer_public_key_label,
                decision.signer_public_key_label
            );
            assert_eq!(
                envelope.expected_signature_algorithm,
                decision.expected_signature_algorithm
            );
            assert_eq!(envelope.signed_payload_hex, decision.signed_payload_hex);
            assert!(envelope.signed_payload_accepted);
            assert!(!envelope.replay_state_mutated);
            assert!(!envelope.challenge_consumed);
            assert!(!envelope.capability_state_changed);
            assert!(!envelope.runtime_io_performed);
            assert!(!envelope.wallet_or_ledger_mutated);
        }
    }

    #[test]
    fn phase10c_rejects_wrong_challenge_unsafe_draft_and_bad_verifier_evidence() {
        let decision = request_decision();

        let mut wrong_challenge = adapter_draft(&decision);
        wrong_challenge.challenge_id = other_challenge_id();
        assert_eq!(
            execute_native_proof_verification_adapter(
                &decision,
                wrong_challenge,
                &AcceptanceVerifier {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                },
            ),
            Err(NativeProofVerificationAdapterReviewError::ChallengeIdMismatch)
        );

        let mut unsafe_draft = adapter_draft(&decision);
        unsafe_draft.requests_replay_store_mutation = true;
        assert_eq!(
            execute_native_proof_verification_adapter(
                &decision,
                unsafe_draft,
                &AcceptanceVerifier {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                },
            ),
            Err(NativeProofVerificationAdapterReviewError::UnsafeAdapterAuthorityFlag)
        );

        assert_eq!(
            execute_native_proof_verification_adapter(
                &decision,
                adapter_draft(&decision),
                &AcceptanceVerifier {
                    reject: true,
                    unsafe_evidence: false,
                    mismatch: false,
                },
            ),
            Err(NativeProofVerificationAdapterReviewError::VerifierRejected)
        );

        assert_eq!(
            execute_native_proof_verification_adapter(
                &decision,
                adapter_draft(&decision),
                &AcceptanceVerifier {
                    reject: false,
                    unsafe_evidence: true,
                    mismatch: false,
                },
            ),
            Err(NativeProofVerificationAdapterReviewError::VerifierEvidenceUnsafe)
        );

        assert_eq!(
            execute_native_proof_verification_adapter(
                &decision,
                adapter_draft(&decision),
                &AcceptanceVerifier {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: true,
                },
            ),
            Err(NativeProofVerificationAdapterReviewError::VerifierEvidenceMismatch)
        );
    }

    #[test]
    fn phase10c_acceptance_sources_remain_public_verifier_only_without_replay_capability_io_or_mutation(
    ) {
        let boundary_source = fs::read_to_string(repo_file("src/native/proof_verification.rs"))
            .expect("verification boundary source");
        let adapter_source =
            fs::read_to_string(repo_file("src/native/proof_verification_adapter.rs"))
                .expect("verification adapter source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(boundary_source.contains("review_native_proof_verification_contract_boundary"));
        assert!(adapter_source.contains("NativeLocalProofVerificationAdapter"));
        assert!(adapter_source.contains("execute_native_proof_verification_adapter"));
        assert!(native_mod.contains("ProofVerificationContractBoundaryDto"));
        assert!(native_mod.contains("ProofVerificationAdapterDto"));

        let combined = format!("{boundary_source}\n{adapter_source}\n{native_mod}");

        assert!(!combined.contains("serde_json"));

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
                !combined.contains(forbidden_runtime_pattern),
                "Phase 10 acceptance sources must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
