#[cfg(not(feature = "native-passport"))]
#[test]
fn phase12b_username_registry_adapter_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native username registry adapter surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        execute_native_username_registry_adapter, native_username_registry_adapter_posture,
        B3DigestHex, ChallengeIdV1, DeviceIdV1, NativeLocalUsernameRegistryAdapter,
        NativePassportSurface, NativeUsernameRegistryAdapterDraftV1,
        NativeUsernameRegistryAdapterEvidenceV1, NativeUsernameRegistryAdapterRequestV1,
        NativeUsernameRegistryAdapterReviewError, NativeUsernameRegistryTransitionDecisionV1,
        NativeUsernameRegistryTransitionKind, PassportIdV1, UsernameV1,
        NATIVE_PASSPORT_PHASE12B_LABEL, PHASE12A_ENABLED_SURFACES,
        PHASE12A_USERNAME_REGISTRY_CONTRACT_DOMAIN, PHASE12A_USERNAME_REGISTRY_CONTRACT_VERSION,
        PHASE12B_ENABLED_SURFACES, PHASE12B_FORBIDDEN_USERNAME_REGISTRY_ADAPTER_AUTHORITY_FLAGS,
        PHASE12B_USERNAME_REGISTRY_ADAPTER_DOMAIN, PHASE12B_USERNAME_REGISTRY_ADAPTER_VERSION,
    };

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const HEX_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    struct FixedUsernameWriter {
        reject: bool,
        unsafe_evidence: bool,
        mismatch: bool,
        omit_finalization: bool,
    }

    impl NativeLocalUsernameRegistryAdapter for FixedUsernameWriter {
        fn apply_username_transition(
            &self,
            request: &NativeUsernameRegistryAdapterRequestV1,
        ) -> Result<NativeUsernameRegistryAdapterEvidenceV1, NativeUsernameRegistryAdapterReviewError>
        {
            assert!(request.contract_only);
            assert!(request.index_projection_only);
            assert!(request.canonical_handle.starts_with('@'));

            let transition_hash = if self.mismatch {
                digest("mismatched_transition_hash", HEX_F)
            } else {
                request.transition_hash.clone()
            };

            Ok(NativeUsernameRegistryAdapterEvidenceV1 {
                accepted: !self.reject,
                private_beta_writer_label: "phase12b-private-beta-username-writer",
                transition_kind: request.transition_kind,
                canonical_username: request.canonical_username.clone(),
                canonical_handle: request.canonical_handle.clone(),
                passport_id: request.passport_id.clone(),
                current_owner_passport_id: request.current_owner_passport_id.clone(),
                recipient_passport_id: request.recipient_passport_id.clone(),
                transition_hash,
                idempotency_key_hash: request.idempotency_key_hash.clone(),
                username_finalized: !self.omit_finalization,
                index_projection_queued: !self.omit_finalization,
                index_projection_authoritative: false,
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

    fn username(input: &'static str) -> UsernameV1 {
        UsernameV1::parse(input).expect("canonical username")
    }

    fn challenge_id() -> ChallengeIdV1 {
        ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_A}")).expect("challenge id")
    }

    fn passport_id() -> PassportIdV1 {
        PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_B}")).expect("passport id")
    }

    fn other_passport_id() -> PassportIdV1 {
        PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_F}"))
            .expect("other passport id")
    }

    fn device_id() -> DeviceIdV1 {
        DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}")).expect("device id")
    }

    fn claim_decision() -> NativeUsernameRegistryTransitionDecisionV1 {
        NativeUsernameRegistryTransitionDecisionV1 {
            contract_domain: PHASE12A_USERNAME_REGISTRY_CONTRACT_DOMAIN,
            contract_version: PHASE12A_USERNAME_REGISTRY_CONTRACT_VERSION,
            transition_kind: NativeUsernameRegistryTransitionKind::Claim,
            canonical_username: username("crab_user7"),
            canonical_handle: "@crab_user7".to_string(),
            passport_id: passport_id(),
            device_id: Some(device_id()),
            current_owner_passport_id: None,
            recipient_passport_id: None,
            challenge_id: challenge_id(),
            operation_body_hash: digest("operation_body_hash", HEX_D),
            transition_hash: digest("username_transition_hash", HEX_E),
            idempotency_key_hash: digest("idempotency_key_hash", HEX_A),
            fresh_proof_transcript_hash: digest("fresh_proof_transcript_hash", HEX_B),
            signer_public_key_label: "public-key:v1:phase12b-device",
            transition_requested_at_ms: 1_100_000,
            transition_expires_at_ms: 1_160_000,
            single_writer_private_beta_reviewed: true,
            index_projection_only: true,
            username_finalized: false,
            durable_writer_called: false,
            index_projection_mutated: false,
            storage_mutated_inside_native: false,
            wallet_or_ledger_mutated: false,
            secret_material_exposed: false,
            contract_only: true,
        }
    }

    fn transfer_decision() -> NativeUsernameRegistryTransitionDecisionV1 {
        let mut decision = claim_decision();
        decision.transition_kind = NativeUsernameRegistryTransitionKind::Transfer;
        decision.current_owner_passport_id = Some(passport_id());
        decision.recipient_passport_id = Some(other_passport_id());
        decision.signer_public_key_label = "public-key:v1:phase12b-root";
        decision
    }

    fn release_decision() -> NativeUsernameRegistryTransitionDecisionV1 {
        let mut decision = claim_decision();
        decision.transition_kind = NativeUsernameRegistryTransitionKind::Release;
        decision.current_owner_passport_id = Some(passport_id());
        decision.recipient_passport_id = None;
        decision.signer_public_key_label = "public-key:v1:phase12b-root";
        decision
    }

    fn adapter_draft(
        decision: &NativeUsernameRegistryTransitionDecisionV1,
    ) -> NativeUsernameRegistryAdapterDraftV1 {
        NativeUsernameRegistryAdapterDraftV1 {
            contract_domain: PHASE12B_USERNAME_REGISTRY_ADAPTER_DOMAIN,
            contract_version: PHASE12B_USERNAME_REGISTRY_ADAPTER_VERSION,
            transition_kind: decision.transition_kind,
            canonical_username: decision.canonical_username.clone(),
            canonical_handle: decision.canonical_handle.clone(),
            passport_id: decision.passport_id.clone(),
            device_id: decision.device_id.clone(),
            current_owner_passport_id: decision.current_owner_passport_id.clone(),
            recipient_passport_id: decision.recipient_passport_id.clone(),
            challenge_id: decision.challenge_id.clone(),
            operation_body_hash: decision.operation_body_hash.clone(),
            transition_hash: decision.transition_hash.clone(),
            idempotency_key_hash: decision.idempotency_key_hash.clone(),
            fresh_proof_transcript_hash: decision.fresh_proof_transcript_hash.clone(),
            signer_public_key_label: decision.signer_public_key_label,
            transition_requested_at_ms: decision.transition_requested_at_ms,
            transition_expires_at_ms: decision.transition_expires_at_ms,
            allows_injected_private_beta_writer: true,
            index_projection_only: true,
            treats_index_projection_as_authority: false,
            treats_username_as_real_or_legal_name_authority: false,
            treats_username_as_recovery_authority: false,
            treats_username_as_root_authority: false,
            treats_username_as_wallet_or_ledger_authority: false,
            requests_namespace_route: false,
            requests_storage_mutation_inside_native: false,
            requests_secret_key_access: false,
            requests_key_loading: false,
            requests_key_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            requests_platform_sealer_unseal: false,
            requests_runtime_io: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase12b_label_and_posture_are_locked() {
        let posture = native_username_registry_adapter_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE12B_LABEL,
            "NATIVE_PASSPORT_PHASE12B_USERNAME_PRIVATE_BETA_REGISTRY_ADAPTER"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE12B_LABEL);
        assert!(posture.username_registry_adapter_added);
        assert!(posture.injected_private_beta_writer_added);
        assert!(posture.username_finalization_evidence_added);
        assert!(posture.index_projection_queue_evidence_added);
        assert!(!posture.durable_username_writer_implemented_inside_native);
        assert!(!posture.index_projection_authority_added);
        assert!(!posture.namespace_routes_added);
        assert!(!posture.storage_mutation_inside_native_added);
        assert!(!posture.username_as_real_or_legal_name_authority_added);
        assert!(!posture.username_as_recovery_authority_added);
        assert!(!posture.username_as_wallet_or_ledger_authority_added);
        assert!(!posture.secret_key_access_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "username_as_real_or_legal_name_authority",
            "username_as_recovery_authority",
            "index_projection_authority",
            "namespace_route_added",
            "storage_mutation_inside_svc_passport_native",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(
                PHASE12B_FORBIDDEN_USERNAME_REGISTRY_ADAPTER_AUTHORITY_FLAGS.contains(&forbidden)
            );
        }
    }

    #[test]
    fn phase12b_surfaces_extend_phase12a_without_back_mutating_it() {
        assert_eq!(
            PHASE12B_ENABLED_SURFACES.len(),
            PHASE12A_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE12B_ENABLED_SURFACES[..PHASE12A_ENABLED_SURFACES.len()],
            PHASE12A_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE12B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::UsernamePrivateBetaRegistryAdapterDto)
        );
        assert_eq!(
            PHASE12A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::UsernamePrivateBetaRegistryContractDto)
        );
    }

    #[test]
    fn phase12b_executes_claim_transfer_and_release_through_injected_writer() {
        for decision in [claim_decision(), transfer_decision(), release_decision()] {
            let envelope = execute_native_username_registry_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedUsernameWriter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_finalization: false,
                },
            )
            .expect("accepted username decision should produce writer envelope");

            assert_eq!(
                envelope.contract_domain,
                PHASE12B_USERNAME_REGISTRY_ADAPTER_DOMAIN
            );
            assert_eq!(envelope.transition_kind, decision.transition_kind);
            assert_eq!(envelope.canonical_username, decision.canonical_username);
            assert_eq!(envelope.canonical_handle, "@crab_user7");
            assert_eq!(envelope.passport_id, decision.passport_id);
            assert_eq!(
                envelope.current_owner_passport_id,
                decision.current_owner_passport_id
            );
            assert_eq!(
                envelope.recipient_passport_id,
                decision.recipient_passport_id
            );
            assert_eq!(envelope.transition_hash, decision.transition_hash);
            assert_eq!(envelope.idempotency_key_hash, decision.idempotency_key_hash);
            assert!(envelope.username_finalized_by_adapter);
            assert!(envelope.index_projection_queued_by_adapter);
            assert!(!envelope.index_projection_authoritative);
            assert!(!envelope.storage_mutated_inside_native);
            assert!(!envelope.runtime_io_performed);
            assert!(!envelope.wallet_or_ledger_mutated);
            assert!(!envelope.secret_material_exposed);
        }
    }

    #[test]
    fn phase12b_rejects_decision_drift_writer_rejection_and_bad_evidence() {
        let decision = claim_decision();

        let mut wrong_handle = adapter_draft(&decision);
        wrong_handle.canonical_handle = "@wrong".to_string();
        assert_eq!(
            execute_native_username_registry_adapter(
                &decision,
                wrong_handle,
                &FixedUsernameWriter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_finalization: false,
                },
            ),
            Err(NativeUsernameRegistryAdapterReviewError::CanonicalHandleMismatch)
        );

        let mut wrong_challenge = adapter_draft(&decision);
        wrong_challenge.challenge_id =
            ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_F}")).expect("other challenge");
        assert_eq!(
            execute_native_username_registry_adapter(
                &decision,
                wrong_challenge,
                &FixedUsernameWriter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_finalization: false,
                },
            ),
            Err(NativeUsernameRegistryAdapterReviewError::ChallengeIdMismatch)
        );

        let mut finalized_decision = claim_decision();
        finalized_decision.username_finalized = true;
        assert_eq!(
            execute_native_username_registry_adapter(
                &finalized_decision,
                adapter_draft(&finalized_decision),
                &FixedUsernameWriter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_finalization: false,
                },
            ),
            Err(NativeUsernameRegistryAdapterReviewError::RegistryDecisionAlreadyFinalized)
        );

        assert_eq!(
            execute_native_username_registry_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedUsernameWriter {
                    reject: true,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_finalization: false,
                },
            ),
            Err(NativeUsernameRegistryAdapterReviewError::WriterRejected)
        );

        assert_eq!(
            execute_native_username_registry_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedUsernameWriter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: true,
                    omit_finalization: false,
                },
            ),
            Err(NativeUsernameRegistryAdapterReviewError::WriterEvidenceMismatch)
        );

        assert_eq!(
            execute_native_username_registry_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedUsernameWriter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch: false,
                    omit_finalization: true,
                },
            ),
            Err(NativeUsernameRegistryAdapterReviewError::WriterEvidenceMismatch)
        );

        assert_eq!(
            execute_native_username_registry_adapter(
                &decision,
                adapter_draft(&decision),
                &FixedUsernameWriter {
                    reject: false,
                    unsafe_evidence: true,
                    mismatch: false,
                    omit_finalization: false,
                },
            ),
            Err(NativeUsernameRegistryAdapterReviewError::WriterEvidenceUnsafe)
        );
    }

    #[test]
    fn phase12b_rejects_all_unsafe_adapter_authority_flags() {
        let decision = claim_decision();

        let mutators: &[fn(&mut NativeUsernameRegistryAdapterDraftV1)] = &[
            |draft| draft.treats_username_as_real_or_legal_name_authority = true,
            |draft| draft.treats_username_as_recovery_authority = true,
            |draft| draft.treats_username_as_root_authority = true,
            |draft| draft.treats_username_as_wallet_or_ledger_authority = true,
            |draft| draft.requests_namespace_route = true,
            |draft| draft.requests_storage_mutation_inside_native = true,
            |draft| draft.requests_secret_key_access = true,
            |draft| draft.requests_key_loading = true,
            |draft| draft.requests_key_derivation_runtime = true,
            |draft| draft.requests_vault_unlock = true,
            |draft| draft.includes_vault_runtime = true,
            |draft| draft.requests_platform_sealer_unseal = true,
            |draft| draft.requests_runtime_io = true,
            |draft| draft.requests_wallet_or_ledger_mutation = true,
        ];

        for mutate in mutators {
            let mut draft = adapter_draft(&decision);
            mutate(&mut draft);
            assert_eq!(
                execute_native_username_registry_adapter(
                    &decision,
                    draft,
                    &FixedUsernameWriter {
                        reject: false,
                        unsafe_evidence: false,
                        mismatch: false,
                        omit_finalization: false,
                    },
                ),
                Err(NativeUsernameRegistryAdapterReviewError::UnsafeUsernameRegistryAdapterAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase12b_source_remains_injected_writer_only_without_routes_storage_index_wallet_ledger_or_secrets(
    ) {
        let source = fs::read_to_string(repo_file("src/native/username_registry_adapter.rs"))
            .expect("username registry adapter source");
        let registry_source = fs::read_to_string(repo_file("src/native/username_registry.rs"))
            .expect("registry source");
        let native_username_source = fs::read_to_string(repo_file("src/native/username.rs"))
            .expect("native username source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeLocalUsernameRegistryAdapter"));
        assert!(source.contains("execute_native_username_registry_adapter"));
        assert!(source.contains("NativeUsernameRegistryAdapterEnvelopeV1"));
        assert!(registry_source.contains("UsernameV1::parse"));
        assert!(native_username_source.contains("pub type UsernameV1 = ron_naming::UsernameV1"));
        assert!(native_mod.contains("UsernamePrivateBetaRegistryAdapterDto"));
        assert!(!source.contains("serde_json"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "lookup_projection.write(",
            "index.write(",
            "username_store.write(",
            "storage.write(",
            "persist_username(",
            "finalize_username(",
            "mutate_index_projection(",
            "load_secret",
            "load_key",
            "secret_key_bytes",
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
            "capability_token:",
            "raw_capability:",
        ] {
            assert!(
                !source.contains(forbidden_runtime_pattern),
                "Phase 12B username registry adapter source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
