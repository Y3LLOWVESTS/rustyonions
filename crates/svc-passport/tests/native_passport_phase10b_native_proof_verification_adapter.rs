#[cfg(not(feature = "native-passport"))]
#[test]
fn phase10b_proof_verification_adapter_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native proof-verification adapter surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        execute_native_proof_verification_adapter, native_proof_verification_adapter_posture,
        B3DigestHex, ChallengeIdV1, DeviceIdV1, NativeLocalProofVerificationAdapter,
        NativePassportProofAuthority, NativePassportProofKind, NativePassportSurface,
        NativeProofSignatureAlgorithm, NativeProofSignedPayloadHex,
        NativeProofSigningOperationKind, NativeProofVerificationAdapterDraftV1,
        NativeProofVerificationAdapterEvidenceV1, NativeProofVerificationAdapterRequestV1,
        NativeProofVerificationAdapterReviewError,
        NativeProofVerificationContractBoundaryDecisionV1, PassportIdV1,
        NATIVE_PASSPORT_PHASE10B_LABEL, PHASE10A_ENABLED_SURFACES, PHASE10B_ENABLED_SURFACES,
        PHASE10B_FORBIDDEN_VERIFICATION_ADAPTER_AUTHORITY_FLAGS,
        PHASE10B_PROOF_VERIFICATION_ADAPTER_DOMAIN, PHASE10B_PROOF_VERIFICATION_ADAPTER_VERSION,
        PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_PROOF_CONTRACT_DOMAIN,
        PHASE8B_PROOF_CONTRACT_VERSION, PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
        PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN, PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
    };

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const HEX_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    struct FixedVerifier {
        reject: bool,
        unsafe_evidence: bool,
        mismatch: bool,
    }

    impl NativeLocalProofVerificationAdapter for FixedVerifier {
        fn assess_signed_payload(
            &self,
            request: &NativeProofVerificationAdapterRequestV1,
        ) -> Result<
            NativeProofVerificationAdapterEvidenceV1,
            NativeProofVerificationAdapterReviewError,
        > {
            assert!(request.contract_only);
            assert!(!request.signer_public_key_label.is_empty());

            let transcript_hash = if self.mismatch {
                digest("mismatched_transcript_hash", HEX_F)
            } else {
                request.transcript_hash.clone()
            };

            Ok(NativeProofVerificationAdapterEvidenceV1 {
                accepted: !self.reject,
                evidence_label: "phase10b-test-evidence",
                verifier_public_key_label: "public-key:v1:phase10b-verifier",
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

    fn signed_payload() -> NativeProofSignedPayloadHex {
        NativeProofSignedPayloadHex::parse("signed_payload_hex", "66".repeat(64))
            .expect("signed payload")
    }

    fn root_decision() -> NativeProofVerificationContractBoundaryDecisionV1 {
        NativeProofVerificationContractBoundaryDecisionV1 {
            contract_domain: "native-passport/proof-verification-contract-boundary/v1",
            contract_version: 1,
            operation_kind: NativeProofSigningOperationKind::RootProof,
            reviewed_authority: NativePassportProofAuthority::RootAuthority,
            challenge_id: challenge_id(),
            passport_id: Some(passport_id()),
            device_id: Some(device_id()),
            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            request_contract_domain: None,
            request_contract_version: None,
            proof_kind: NativePassportProofKind::RootRegistration,
            transcript_hash: digest("proof_transcript_hash", HEX_D),
            signer_public_key_label: "public-key:v1:phase10b-root",
            expected_signature_algorithm: PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
            signed_payload_hex: signed_payload(),
            verification_requested_at_ms: 1_035_001,
            cryptographic_verification_performed: false,
            replay_state_mutated: false,
            challenge_consumed: false,
            capability_state_changed: false,
            runtime_io_performed: false,
            wallet_or_ledger_mutated: false,
            contract_only: true,
        }
    }

    fn request_decision() -> NativeProofVerificationContractBoundaryDecisionV1 {
        NativeProofVerificationContractBoundaryDecisionV1 {
            contract_domain: "native-passport/proof-verification-contract-boundary/v1",
            contract_version: 1,
            operation_kind: NativeProofSigningOperationKind::RequestProof,
            reviewed_authority: NativePassportProofAuthority::DeviceAuthority,
            challenge_id: challenge_id(),
            passport_id: Some(passport_id()),
            device_id: Some(device_id()),
            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            request_contract_domain: Some(PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN),
            request_contract_version: Some(PHASE8C_PROOF_REQUEST_CONTRACT_VERSION),
            proof_kind: NativePassportProofKind::DeviceSession,
            transcript_hash: digest("request_transcript_hash", HEX_E),
            signer_public_key_label: "public-key:v1:phase10b-device",
            expected_signature_algorithm: PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
            signed_payload_hex: signed_payload(),
            verification_requested_at_ms: 1_045_001,
            cryptographic_verification_performed: false,
            replay_state_mutated: false,
            challenge_consumed: false,
            capability_state_changed: false,
            runtime_io_performed: false,
            wallet_or_ledger_mutated: false,
            contract_only: true,
        }
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
    fn phase10b_label_and_posture_are_locked() {
        let posture = native_proof_verification_adapter_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE10B_LABEL,
            "NATIVE_PASSPORT_PHASE10B_NATIVE_PROOF_VERIFICATION_ADAPTER"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE10B_LABEL);
        assert!(posture.proof_verification_adapter_added);
        assert!(posture.injected_local_verifier_adapter_added);
        assert!(posture.root_proof_verification_adapter_added);
        assert!(posture.device_proof_verification_adapter_added);
        assert!(posture.request_proof_verification_adapter_added);
        assert!(!posture.secret_key_access_added);
        assert!(!posture.key_loading_added);
        assert!(!posture.vault_unlock_added);
        assert!(!posture.replay_store_mutation_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "secret_key_access",
            "key_loading",
            "replay_store_mutation",
            "challenge_consumption",
            "capability_issuance",
            "runtime_io",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE10B_FORBIDDEN_VERIFICATION_ADAPTER_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase10b_surfaces_extend_phase10a_without_back_mutating_it() {
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
        assert_eq!(
            PHASE10A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ProofVerificationContractBoundaryDto)
        );
    }

    #[test]
    fn phase10b_executes_root_verification_adapter_from_boundary_decision() {
        let decision = root_decision();
        let envelope = execute_native_proof_verification_adapter(
            &decision,
            adapter_draft(&decision),
            &FixedVerifier {
                reject: false,
                unsafe_evidence: false,
                mismatch: false,
            },
        )
        .expect("root verification adapter should accept");

        assert_eq!(
            envelope.contract_domain,
            PHASE10B_PROOF_VERIFICATION_ADAPTER_DOMAIN
        );
        assert_eq!(
            envelope.operation_kind,
            NativeProofSigningOperationKind::RootProof
        );
        assert_eq!(
            envelope.reviewed_authority,
            NativePassportProofAuthority::RootAuthority
        );
        assert_eq!(
            envelope.expected_signature_algorithm,
            NativeProofSignatureAlgorithm::RootEd25519V1
        );
        assert_eq!(envelope.transcript_hash, decision.transcript_hash);
        assert!(envelope.signed_payload_accepted);
        assert!(!envelope.replay_state_mutated);
        assert!(!envelope.challenge_consumed);
        assert!(!envelope.capability_state_changed);
        assert!(!envelope.runtime_io_performed);
        assert!(!envelope.wallet_or_ledger_mutated);
    }

    #[test]
    fn phase10b_executes_request_verification_adapter_from_boundary_decision() {
        let decision = request_decision();
        let envelope = execute_native_proof_verification_adapter(
            &decision,
            adapter_draft(&decision),
            &FixedVerifier {
                reject: false,
                unsafe_evidence: false,
                mismatch: false,
            },
        )
        .expect("request verification adapter should accept");

        assert_eq!(
            envelope.operation_kind,
            NativeProofSigningOperationKind::RequestProof
        );
        assert_eq!(
            envelope.reviewed_authority,
            NativePassportProofAuthority::DeviceAuthority
        );
        assert_eq!(
            envelope.request_contract_domain,
            Some(PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN)
        );
        assert_eq!(
            envelope.expected_signature_algorithm,
            NativeProofSignatureAlgorithm::DeviceEd25519V1
        );
        assert_eq!(envelope.signed_payload_hex.as_str().len(), 128);
        assert!(envelope.signed_payload_accepted);
        assert!(!envelope.replay_state_mutated);
        assert!(!envelope.challenge_consumed);
        assert!(!envelope.capability_state_changed);
    }

    #[test]
    fn phase10b_rejects_decision_drift_verifier_rejection_and_bad_evidence() {
        let decision = request_decision();

        let mut wrong_challenge = adapter_draft(&decision);
        wrong_challenge.challenge_id = other_challenge_id();
        assert_eq!(
            execute_native_proof_verification_adapter(
                &decision,
                wrong_challenge,
                &FixedVerifier {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                },
            ),
            Err(NativeProofVerificationAdapterReviewError::ChallengeIdMismatch)
        );

        let mut wrong_label = adapter_draft(&decision);
        wrong_label.signer_public_key_label = "public-key:v1:wrong";
        assert_eq!(
            execute_native_proof_verification_adapter(
                &decision,
                wrong_label,
                &FixedVerifier {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                },
            ),
            Err(NativeProofVerificationAdapterReviewError::SignerPublicKeyLabelMismatch)
        );

        assert_eq!(
            execute_native_proof_verification_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedVerifier {
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
                &FixedVerifier {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: true,
                },
            ),
            Err(NativeProofVerificationAdapterReviewError::VerifierEvidenceMismatch)
        );

        assert_eq!(
            execute_native_proof_verification_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedVerifier {
                    reject: false,
                    unsafe_evidence: true,
                    mismatch: false,
                },
            ),
            Err(NativeProofVerificationAdapterReviewError::VerifierEvidenceUnsafe)
        );
    }

    #[test]
    fn phase10b_rejects_all_unsafe_adapter_authority_flags() {
        let decision = request_decision();

        let mutators: &[fn(&mut NativeProofVerificationAdapterDraftV1)] = &[
            |draft| draft.requests_secret_key_access = true,
            |draft| draft.requests_key_loading = true,
            |draft| draft.requests_key_derivation_runtime = true,
            |draft| draft.requests_vault_unlock = true,
            |draft| draft.includes_vault_runtime = true,
            |draft| draft.requests_platform_sealer_unseal = true,
            |draft| draft.requests_replay_store_mutation = true,
            |draft| draft.requests_challenge_consumption = true,
            |draft| draft.requests_capability_consumption = true,
            |draft| draft.requests_capability_issuance = true,
            |draft| draft.requests_capability_lifecycle_runtime = true,
            |draft| draft.requests_runtime_io = true,
            |draft| draft.adds_routes = true,
            |draft| draft.requests_storage_mutation = true,
            |draft| draft.requests_wallet_or_ledger_mutation = true,
        ];

        for mutate in mutators {
            let mut draft = adapter_draft(&decision);
            mutate(&mut draft);
            assert_eq!(
                execute_native_proof_verification_adapter(
                    &decision,
                    draft,
                    &FixedVerifier {
                        reject: false,
                        unsafe_evidence: false,
                        mismatch: false,
                    },
                ),
                Err(NativeProofVerificationAdapterReviewError::UnsafeAdapterAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase10b_source_remains_adapter_only_without_key_replay_capability_io_or_mutation() {
        let source = fs::read_to_string(repo_file("src/native/proof_verification_adapter.rs"))
            .expect("proof verification adapter source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeLocalProofVerificationAdapter"));
        assert!(source.contains("execute_native_proof_verification_adapter"));
        assert!(source.contains("NativeProofVerificationAdapterEnvelopeV1"));
        assert!(native_mod.contains("ProofVerificationAdapterDto"));
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
                "Phase 10B proof-verification adapter source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
