#[cfg(not(feature = "native-passport"))]
#[test]
fn phase11b_replay_consumption_adapter_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native replay/challenge adapter surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        execute_native_replay_challenge_consumption_adapter,
        native_replay_challenge_consumption_adapter_posture, B3DigestHex, ChallengeIdV1,
        DeviceIdV1, NativeLocalReplayChallengeConsumptionAdapter, NativePassportProofAuthority,
        NativePassportProofKind, NativePassportSurface, NativeProofSignatureAlgorithm,
        NativeProofSignedPayloadHex, NativeProofSigningOperationKind,
        NativeReplayChallengeConsumptionAdapterDraftV1,
        NativeReplayChallengeConsumptionAdapterEvidenceV1,
        NativeReplayChallengeConsumptionAdapterRequestV1,
        NativeReplayChallengeConsumptionAdapterReviewError,
        NativeReplayChallengeConsumptionContractDecisionV1, PassportIdV1,
        NATIVE_PASSPORT_PHASE11B_LABEL, PHASE11A_ENABLED_SURFACES, PHASE11B_ENABLED_SURFACES,
        PHASE11B_FORBIDDEN_CONSUMPTION_ADAPTER_AUTHORITY_FLAGS,
        PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_DOMAIN,
        PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_VERSION,
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

    struct FixedStateAdapter {
        reject: bool,
        unsafe_evidence: bool,
        mismatch: bool,
        omit_application: bool,
    }

    impl NativeLocalReplayChallengeConsumptionAdapter for FixedStateAdapter {
        fn apply_consumption_state(
            &self,
            request: &NativeReplayChallengeConsumptionAdapterRequestV1,
        ) -> Result<
            NativeReplayChallengeConsumptionAdapterEvidenceV1,
            NativeReplayChallengeConsumptionAdapterReviewError,
        > {
            assert!(request.contract_only);
            assert!(!request.signer_public_key_label.is_empty());
            assert!(request.consumption_requested_at_ms >= 1_035_002);

            let replay_key_hash = if self.mismatch {
                digest("mismatched_replay_key_hash", HEX_F)
            } else {
                request.replay_key_hash.clone()
            };

            Ok(NativeReplayChallengeConsumptionAdapterEvidenceV1 {
                accepted: !self.reject,
                state_adapter_label: "phase11b-test-state-adapter",
                challenge_id: request.challenge_id.clone(),
                replay_key_hash,
                idempotency_key_hash: request.idempotency_key_hash.clone(),
                replay_recorded: !self.omit_application,
                challenge_marked_consumed: !self.omit_application,
                capability_state_changed: false,
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
        NativeProofSignedPayloadHex::parse("phase11b_signed_payload_hex", hex_pair.repeat(64))
            .expect("signed payload")
    }

    fn decision(
        operation_kind: NativeProofSigningOperationKind,
        authority: NativePassportProofAuthority,
        proof_kind: NativePassportProofKind,
        transcript_hash: B3DigestHex,
        signer_public_key_label: &'static str,
        algorithm: NativeProofSignatureAlgorithm,
        payload_pair: &'static str,
        request_contract_domain: Option<&'static str>,
        request_contract_version: Option<u16>,
        consumption_requested_at_ms: u64,
    ) -> NativeReplayChallengeConsumptionContractDecisionV1 {
        NativeReplayChallengeConsumptionContractDecisionV1 {
            contract_domain: "native-passport/replay-challenge-consumption-contract/v1",
            contract_version: 1,
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
            verification_evidence_label: "phase11b-verification-evidence",
            verifier_public_key_label: "public-key:v1:phase11b-verifier",
            replay_key_hash: digest("replay_key_hash", HEX_D),
            idempotency_key_hash: digest("idempotency_key_hash", HEX_E),
            consumption_requested_at_ms,
            replay_state_reviewed_unseen: true,
            challenge_reviewed_unconsumed: true,
            replay_store_mutated: false,
            challenge_consumed: false,
            capability_state_changed: false,
            runtime_io_performed: false,
            wallet_or_ledger_mutated: false,
            contract_only: true,
        }
    }

    fn root_decision() -> NativeReplayChallengeConsumptionContractDecisionV1 {
        decision(
            NativeProofSigningOperationKind::RootProof,
            NativePassportProofAuthority::RootAuthority,
            NativePassportProofKind::RootRegistration,
            digest("proof_transcript_hash", HEX_D),
            "public-key:v1:phase11b-root",
            PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
            "aa",
            None,
            None,
            1_035_002,
        )
    }

    fn request_decision() -> NativeReplayChallengeConsumptionContractDecisionV1 {
        decision(
            NativeProofSigningOperationKind::RequestProof,
            NativePassportProofAuthority::DeviceAuthority,
            NativePassportProofKind::DeviceSession,
            digest("request_transcript_hash", HEX_E),
            "public-key:v1:phase11b-request",
            PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
            "bb",
            Some(PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN),
            Some(PHASE8C_PROOF_REQUEST_CONTRACT_VERSION),
            1_045_002,
        )
    }

    fn adapter_draft(
        decision: &NativeReplayChallengeConsumptionContractDecisionV1,
    ) -> NativeReplayChallengeConsumptionAdapterDraftV1 {
        NativeReplayChallengeConsumptionAdapterDraftV1 {
            contract_domain: PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_DOMAIN,
            contract_version: PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_VERSION,
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
            verification_evidence_label: decision.verification_evidence_label,
            verifier_public_key_label: decision.verifier_public_key_label,
            replay_key_hash: decision.replay_key_hash.clone(),
            idempotency_key_hash: decision.idempotency_key_hash.clone(),
            consumption_requested_at_ms: decision.consumption_requested_at_ms,
            allows_injected_local_state_adapter: true,
            requests_secret_key_access: false,
            requests_key_loading: false,
            requests_key_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            requests_platform_sealer_unseal: false,
            requests_capability_consumption: false,
            requests_capability_issuance: false,
            requests_capability_lifecycle_runtime: false,
            requests_runtime_io: false,
            adds_routes: false,
            requests_storage_mutation_inside_native: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase11b_label_and_posture_are_locked() {
        let posture = native_replay_challenge_consumption_adapter_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE11B_LABEL,
            "NATIVE_PASSPORT_PHASE11B_REPLAY_AND_CHALLENGE_CONSUMPTION_ADAPTER"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE11B_LABEL);
        assert!(posture.replay_challenge_consumption_adapter_added);
        assert!(posture.injected_local_state_adapter_added);
        assert!(posture.replay_recording_evidence_added);
        assert!(posture.challenge_consumption_evidence_added);
        assert!(!posture.capability_issuance_added);
        assert!(!posture.secret_key_access_added);
        assert!(!posture.key_loading_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.storage_mutation_inside_native_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "secret_key_access",
            "key_loading",
            "capability_consumption",
            "capability_issuance",
            "runtime_io",
            "storage_mutation_inside_svc_passport_native",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE11B_FORBIDDEN_CONSUMPTION_ADAPTER_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase11b_surfaces_extend_phase11a_without_back_mutating_it() {
        assert_eq!(
            PHASE11B_ENABLED_SURFACES.len(),
            PHASE11A_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE11B_ENABLED_SURFACES[..PHASE11A_ENABLED_SURFACES.len()],
            PHASE11A_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE11B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ReplayChallengeConsumptionAdapterDto)
        );
        assert_eq!(
            PHASE11A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ReplayChallengeConsumptionContractDto)
        );
    }

    #[test]
    fn phase11b_executes_root_and_request_state_adapter_from_contract_decision() {
        for decision in [root_decision(), request_decision()] {
            let envelope = execute_native_replay_challenge_consumption_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedStateAdapter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_application: false,
                },
            )
            .expect("accepted contract decision should produce adapter envelope");

            assert_eq!(
                envelope.contract_domain,
                PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_DOMAIN
            );
            assert_eq!(envelope.operation_kind, decision.operation_kind);
            assert_eq!(envelope.reviewed_authority, decision.reviewed_authority);
            assert_eq!(envelope.challenge_id, decision.challenge_id);
            assert_eq!(envelope.replay_key_hash, decision.replay_key_hash);
            assert_eq!(envelope.idempotency_key_hash, decision.idempotency_key_hash);
            assert!(envelope.replay_recorded_by_adapter);
            assert!(envelope.challenge_consumed_by_adapter);
            assert!(!envelope.capability_state_changed);
            assert!(!envelope.runtime_io_performed);
            assert!(!envelope.wallet_or_ledger_mutated);
        }
    }

    #[test]
    fn phase11b_rejects_decision_drift_rejected_state_and_bad_evidence() {
        let decision = request_decision();

        let mut wrong_challenge = adapter_draft(&decision);
        wrong_challenge.challenge_id = other_challenge_id();
        assert_eq!(
            execute_native_replay_challenge_consumption_adapter(
                &decision,
                wrong_challenge,
                &FixedStateAdapter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_application: false,
                },
            ),
            Err(NativeReplayChallengeConsumptionAdapterReviewError::ChallengeIdMismatch)
        );

        let mut wrong_replay = adapter_draft(&decision);
        wrong_replay.replay_key_hash = digest("wrong_replay_key_hash", HEX_F);
        assert_eq!(
            execute_native_replay_challenge_consumption_adapter(
                &decision,
                wrong_replay,
                &FixedStateAdapter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_application: false,
                },
            ),
            Err(NativeReplayChallengeConsumptionAdapterReviewError::ReplayKeyMismatch)
        );

        assert_eq!(
            execute_native_replay_challenge_consumption_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedStateAdapter {
                    reject: true,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_application: false,
                },
            ),
            Err(NativeReplayChallengeConsumptionAdapterReviewError::StateAdapterRejected)
        );

        assert_eq!(
            execute_native_replay_challenge_consumption_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedStateAdapter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: true,
                    omit_application: false,
                },
            ),
            Err(NativeReplayChallengeConsumptionAdapterReviewError::StateAdapterEvidenceMismatch)
        );

        assert_eq!(
            execute_native_replay_challenge_consumption_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedStateAdapter {
                    reject: false,
                    unsafe_evidence: true,
                    mismatch: false,
                    omit_application: false,
                },
            ),
            Err(NativeReplayChallengeConsumptionAdapterReviewError::StateAdapterEvidenceUnsafe)
        );

        assert_eq!(
            execute_native_replay_challenge_consumption_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedStateAdapter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_application: true,
                },
            ),
            Err(NativeReplayChallengeConsumptionAdapterReviewError::StateAdapterEvidenceMismatch)
        );
    }

    #[test]
    fn phase11b_rejects_all_unsafe_adapter_authority_flags() {
        let decision = request_decision();

        let mutators: &[fn(&mut NativeReplayChallengeConsumptionAdapterDraftV1)] = &[
            |draft| draft.requests_secret_key_access = true,
            |draft| draft.requests_key_loading = true,
            |draft| draft.requests_key_derivation_runtime = true,
            |draft| draft.requests_vault_unlock = true,
            |draft| draft.includes_vault_runtime = true,
            |draft| draft.requests_platform_sealer_unseal = true,
            |draft| draft.requests_capability_consumption = true,
            |draft| draft.requests_capability_issuance = true,
            |draft| draft.requests_capability_lifecycle_runtime = true,
            |draft| draft.requests_runtime_io = true,
            |draft| draft.adds_routes = true,
            |draft| draft.requests_storage_mutation_inside_native = true,
            |draft| draft.requests_wallet_or_ledger_mutation = true,
        ];

        for mutate in mutators {
            let mut draft = adapter_draft(&decision);
            mutate(&mut draft);
            assert_eq!(
                execute_native_replay_challenge_consumption_adapter(
                    &decision,
                    draft,
                    &FixedStateAdapter {
                        reject: false,
                        unsafe_evidence: false,
                        mismatch: false,
                        omit_application: false,
                    },
                ),
                Err(NativeReplayChallengeConsumptionAdapterReviewError::UnsafeAdapterAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase11b_source_remains_injected_adapter_only_without_routes_io_wallet_ledger_or_secrets() {
        let source = fs::read_to_string(repo_file("src/native/replay_consumption_adapter.rs"))
            .expect("replay consumption adapter source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeLocalReplayChallengeConsumptionAdapter"));
        assert!(source.contains("execute_native_replay_challenge_consumption_adapter"));
        assert!(source.contains("NativeReplayChallengeConsumptionAdapterEnvelopeV1"));
        assert!(native_mod.contains("ReplayChallengeConsumptionAdapterDto"));
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
            "refresh_capability(",
            "revoke_capability(",
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
                "Phase 11B replay/challenge adapter source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
