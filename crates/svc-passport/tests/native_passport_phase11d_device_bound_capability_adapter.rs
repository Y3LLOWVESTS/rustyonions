#[cfg(not(feature = "native-passport"))]
#[test]
fn phase11d_capability_adapter_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native device-bound capability adapter surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        execute_native_device_bound_capability_adapter,
        native_device_bound_capability_adapter_posture, B3DigestHex, ChallengeIdV1, DeviceIdV1,
        NativeDeviceBoundCapabilityAdapterDraftV1, NativeDeviceBoundCapabilityAdapterEvidenceV1,
        NativeDeviceBoundCapabilityAdapterRequestV1, NativeDeviceBoundCapabilityAdapterReviewError,
        NativeDeviceBoundCapabilityContractDecisionV1, NativeLocalDeviceBoundCapabilityAdapter,
        NativePassportProofAuthority, NativePassportProofKind, NativePassportScope,
        NativePassportSurface, NativeProofSignedPayloadHex, NativeProofSigningOperationKind,
        PassportIdV1, NATIVE_PASSPORT_PHASE11D_LABEL,
        PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_DOMAIN,
        PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_VERSION, PHASE11C_ENABLED_SURFACES,
        PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER_DOMAIN,
        PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER_VERSION, PHASE11D_ENABLED_SURFACES,
        PHASE11D_FORBIDDEN_CAPABILITY_ADAPTER_AUTHORITY_FLAGS,
        PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_PROOF_CONTRACT_DOMAIN,
        PHASE8B_PROOF_CONTRACT_VERSION, PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN,
        PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
    };

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const HEX_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    struct FixedCapabilityAdapter {
        reject: bool,
        unsafe_evidence: bool,
        mismatch: bool,
        omit_record: bool,
        consume: bool,
    }

    impl NativeLocalDeviceBoundCapabilityAdapter for FixedCapabilityAdapter {
        fn apply_device_bound_capability(
            &self,
            request: &NativeDeviceBoundCapabilityAdapterRequestV1,
        ) -> Result<
            NativeDeviceBoundCapabilityAdapterEvidenceV1,
            NativeDeviceBoundCapabilityAdapterReviewError,
        > {
            assert!(request.contract_only);
            assert!(request.device_bound);
            assert_eq!(
                request.expected_authority,
                NativePassportProofAuthority::DeviceAuthority
            );

            let capability_id_hash = if self.mismatch {
                digest("mismatched_capability_id_hash", HEX_F)
            } else {
                request.capability_id_hash.clone()
            };

            Ok(NativeDeviceBoundCapabilityAdapterEvidenceV1 {
                accepted: !self.reject,
                capability_state_adapter_label: "phase11d-capability-state-adapter",
                challenge_id: request.challenge_id.clone(),
                device_id: request.device_id.clone(),
                capability_id_hash,
                capability_binding_hash: request.capability_binding_hash.clone(),
                capability_scope_hash: request.capability_scope_hash.clone(),
                replay_key_hash: request.replay_key_hash.clone(),
                idempotency_key_hash: request.idempotency_key_hash.clone(),
                capability_recorded: !self.omit_record,
                device_bound: true,
                capability_consumed: self.consume,
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
        NativeProofSignedPayloadHex::parse("phase11d_signed_payload_hex", hex_pair.repeat(64))
            .expect("signed payload")
    }

    fn decision() -> NativeDeviceBoundCapabilityContractDecisionV1 {
        NativeDeviceBoundCapabilityContractDecisionV1 {
            contract_domain: PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_DOMAIN,
            contract_version: PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_VERSION,
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
            transcript_hash: digest("request_transcript_hash", HEX_D),
            signer_public_key_label: "public-key:v1:phase11d-device",
            expected_signature_algorithm: PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
            signed_payload_hex: signed_payload("ac"),
            verification_evidence_label: "phase11d-verification-evidence",
            verifier_public_key_label: "public-key:v1:phase11d-verifier",
            state_adapter_label: "phase11d-replay-state-adapter",
            replay_key_hash: digest("replay_key_hash", HEX_E),
            idempotency_key_hash: digest("idempotency_key_hash", HEX_D),
            capability_id_hash: digest("capability_id_hash", HEX_A),
            capability_binding_hash: digest("capability_binding_hash", HEX_B),
            capability_scope_hash: digest("capability_scope_hash", HEX_C),
            capability_scopes: vec![
                NativePassportScope::IdentityRead,
                NativePassportScope::CatalogRead,
            ],
            capability_issued_at_ms: 1_045_003,
            capability_expires_at_ms: 1_105_003,
            device_bound: true,
            replay_recorded_reviewed: true,
            challenge_consumed_reviewed: true,
            capability_issued: false,
            capability_state_changed: false,
            runtime_io_performed: false,
            wallet_or_ledger_mutated: false,
            contract_only: true,
        }
    }

    fn adapter_draft(
        decision: &NativeDeviceBoundCapabilityContractDecisionV1,
    ) -> NativeDeviceBoundCapabilityAdapterDraftV1 {
        NativeDeviceBoundCapabilityAdapterDraftV1 {
            contract_domain: PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER_DOMAIN,
            contract_version: PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER_VERSION,
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
            state_adapter_label: decision.state_adapter_label,
            replay_key_hash: decision.replay_key_hash.clone(),
            idempotency_key_hash: decision.idempotency_key_hash.clone(),
            capability_id_hash: decision.capability_id_hash.clone(),
            capability_binding_hash: decision.capability_binding_hash.clone(),
            capability_scope_hash: decision.capability_scope_hash.clone(),
            capability_scopes: decision.capability_scopes.clone(),
            capability_issued_at_ms: decision.capability_issued_at_ms,
            capability_expires_at_ms: decision.capability_expires_at_ms,
            device_bound: decision.device_bound,
            allows_injected_capability_state_adapter: true,
            requests_secret_key_access: false,
            requests_key_loading: false,
            requests_key_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            requests_platform_sealer_unseal: false,
            requests_capability_consumption_execution: false,
            requests_capability_lifecycle_runtime: false,
            requests_runtime_io: false,
            adds_routes: false,
            requests_storage_mutation_inside_native: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase11d_label_and_posture_are_locked() {
        let posture = native_device_bound_capability_adapter_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE11D_LABEL,
            "NATIVE_PASSPORT_PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE11D_LABEL);
        assert!(posture.device_bound_capability_adapter_added);
        assert!(posture.injected_capability_state_adapter_added);
        assert!(posture.capability_recording_evidence_added);
        assert!(!posture.capability_consumption_execution_added);
        assert!(!posture.capability_lifecycle_runtime_added);
        assert!(!posture.secret_key_access_added);
        assert!(!posture.key_loading_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.storage_mutation_inside_native_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "secret_key_access",
            "key_loading",
            "capability_consumption_execution",
            "runtime_io",
            "storage_mutation_inside_svc_passport_native",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE11D_FORBIDDEN_CAPABILITY_ADAPTER_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase11d_surfaces_extend_phase11c_without_back_mutating_it() {
        assert_eq!(
            PHASE11D_ENABLED_SURFACES.len(),
            PHASE11C_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE11D_ENABLED_SURFACES[..PHASE11C_ENABLED_SURFACES.len()],
            PHASE11C_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE11D_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::DeviceBoundCapabilityAdapterDto)
        );
        assert_eq!(
            PHASE11C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::DeviceBoundCapabilityContractDto)
        );
    }

    #[test]
    fn phase11d_executes_capability_adapter_from_contract_decision() {
        let decision = decision();
        let envelope = execute_native_device_bound_capability_adapter(
            &decision,
            adapter_draft(&decision),
            &FixedCapabilityAdapter {
                reject: false,
                unsafe_evidence: false,
                mismatch: false,
                omit_record: false,
                consume: false,
            },
        )
        .expect("accepted capability decision should produce adapter envelope");

        assert_eq!(
            envelope.contract_domain,
            PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER_DOMAIN
        );
        assert_eq!(
            envelope.operation_kind,
            NativeProofSigningOperationKind::RequestProof
        );
        assert_eq!(
            envelope.reviewed_authority,
            NativePassportProofAuthority::DeviceAuthority
        );
        assert_eq!(envelope.challenge_id, decision.challenge_id);
        assert_eq!(envelope.device_id, decision.device_id);
        assert_eq!(envelope.capability_id_hash, decision.capability_id_hash);
        assert_eq!(
            envelope.capability_binding_hash,
            decision.capability_binding_hash
        );
        assert_eq!(
            envelope.capability_scope_hash,
            decision.capability_scope_hash
        );
        assert_eq!(envelope.capability_scopes, decision.capability_scopes);
        assert!(envelope.device_bound);
        assert!(envelope.capability_recorded_by_adapter);
        assert!(!envelope.capability_consumed);
        assert!(!envelope.runtime_io_performed);
        assert!(!envelope.wallet_or_ledger_mutated);
    }

    #[test]
    fn phase11d_rejects_decision_drift_rejected_state_and_bad_evidence() {
        let decision = decision();

        let mut wrong_challenge = adapter_draft(&decision);
        wrong_challenge.challenge_id = other_challenge_id();
        assert_eq!(
            execute_native_device_bound_capability_adapter(
                &decision,
                wrong_challenge,
                &FixedCapabilityAdapter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_record: false,
                    consume: false,
                },
            ),
            Err(NativeDeviceBoundCapabilityAdapterReviewError::ChallengeIdMismatch)
        );

        let mut wrong_capability = adapter_draft(&decision);
        wrong_capability.capability_id_hash = digest("wrong_capability_id_hash", HEX_F);
        assert_eq!(
            execute_native_device_bound_capability_adapter(
                &decision,
                wrong_capability,
                &FixedCapabilityAdapter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_record: false,
                    consume: false,
                },
            ),
            Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityIdMismatch)
        );

        assert_eq!(
            execute_native_device_bound_capability_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedCapabilityAdapter {
                    reject: true,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_record: false,
                    consume: false,
                },
            ),
            Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityStateAdapterRejected)
        );

        assert_eq!(
            execute_native_device_bound_capability_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedCapabilityAdapter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: true,
                    omit_record: false,
                    consume: false,
                },
            ),
            Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityStateAdapterEvidenceMismatch)
        );

        assert_eq!(
            execute_native_device_bound_capability_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedCapabilityAdapter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_record: true,
                    consume: false,
                },
            ),
            Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityStateAdapterEvidenceMismatch)
        );

        assert_eq!(
            execute_native_device_bound_capability_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedCapabilityAdapter {
                    reject: false,
                    unsafe_evidence: true,
                    mismatch: false,
                    omit_record: false,
                    consume: false,
                },
            ),
            Err(
                NativeDeviceBoundCapabilityAdapterReviewError::CapabilityStateAdapterEvidenceUnsafe
            )
        );

        assert_eq!(
            execute_native_device_bound_capability_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedCapabilityAdapter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_record: false,
                    consume: true,
                },
            ),
            Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityStateAdapterEvidenceMismatch)
        );
    }

    #[test]
    fn phase11d_rejects_all_unsafe_adapter_authority_flags() {
        let decision = decision();

        let mutators: &[fn(&mut NativeDeviceBoundCapabilityAdapterDraftV1)] = &[
            |draft| draft.requests_secret_key_access = true,
            |draft| draft.requests_key_loading = true,
            |draft| draft.requests_key_derivation_runtime = true,
            |draft| draft.requests_vault_unlock = true,
            |draft| draft.includes_vault_runtime = true,
            |draft| draft.requests_platform_sealer_unseal = true,
            |draft| draft.requests_capability_consumption_execution = true,
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
                execute_native_device_bound_capability_adapter(
                    &decision,
                    draft,
                    &FixedCapabilityAdapter {
                        reject: false,
                        unsafe_evidence: false,
                        mismatch: false,
                        omit_record: false,
                        consume: false,
                    },
                ),
                Err(NativeDeviceBoundCapabilityAdapterReviewError::UnsafeAdapterAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase11d_source_remains_injected_adapter_only_without_routes_io_wallet_ledger_or_secrets() {
        let source = fs::read_to_string(repo_file("src/native/capability_adapter.rs"))
            .expect("capability adapter source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeLocalDeviceBoundCapabilityAdapter"));
        assert!(source.contains("execute_native_device_bound_capability_adapter"));
        assert!(source.contains("NativeDeviceBoundCapabilityAdapterEnvelopeV1"));
        assert!(native_mod.contains("DeviceBoundCapabilityAdapterDto"));
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
                "Phase 11D capability adapter source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
