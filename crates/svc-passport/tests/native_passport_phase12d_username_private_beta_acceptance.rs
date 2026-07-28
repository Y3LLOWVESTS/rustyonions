#[cfg(not(feature = "native-passport"))]
#[test]
fn phase12d_username_private_beta_acceptance_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native username private-beta acceptance surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        execute_native_username_registry_adapter, native_username_index_projection_posture,
        native_username_registry_adapter_posture, native_username_registry_contract_posture,
        review_native_username_index_projection_contract,
        review_native_username_registry_transition_contract, B3DigestHex, ChallengeIdV1,
        DeviceIdV1, NativeLocalUsernameRegistryAdapter, NativePassportSurface,
        NativeUsernameIndexProjectionDraftV1, NativeUsernameIndexProjectionKind,
        NativeUsernameIndexProjectionReviewError, NativeUsernameRegistryAdapterDraftV1,
        NativeUsernameRegistryAdapterEvidenceV1, NativeUsernameRegistryAdapterRequestV1,
        NativeUsernameRegistryAdapterReviewError, NativeUsernameRegistryTransitionDraftV1,
        NativeUsernameRegistryTransitionKind, NativeUsernameRegistryTransitionReviewError,
        PassportIdV1, UsernameV1, PHASE12A_ENABLED_SURFACES,
        PHASE12A_USERNAME_REGISTRY_CONTRACT_DOMAIN, PHASE12A_USERNAME_REGISTRY_CONTRACT_VERSION,
        PHASE12B_ENABLED_SURFACES, PHASE12B_USERNAME_REGISTRY_ADAPTER_DOMAIN,
        PHASE12B_USERNAME_REGISTRY_ADAPTER_VERSION, PHASE12C_ENABLED_SURFACES,
        PHASE12C_USERNAME_INDEX_PROJECTION_DOMAIN, PHASE12C_USERNAME_INDEX_PROJECTION_VERSION,
    };

    const PHASE12D_ACCEPTANCE_LABEL: &str =
        "NATIVE_PASSPORT_PHASE12D_USERNAME_PRIVATE_BETA_ACCEPTANCE";

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const HEX_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    struct AcceptanceUsernameWriter {
        reject: bool,
        unsafe_evidence: bool,
        mismatch_transition: bool,
        omit_finalization: bool,
        authoritative_projection: bool,
    }

    impl NativeLocalUsernameRegistryAdapter for AcceptanceUsernameWriter {
        fn apply_username_transition(
            &self,
            request: &NativeUsernameRegistryAdapterRequestV1,
        ) -> Result<NativeUsernameRegistryAdapterEvidenceV1, NativeUsernameRegistryAdapterReviewError>
        {
            assert!(request.contract_only);
            assert!(request.index_projection_only);
            assert!(request.canonical_handle.starts_with('@'));
            assert_eq!(
                request.canonical_handle,
                request.canonical_username.handle()
            );
            assert!(!request.signer_public_key_label.is_empty());

            let transition_hash = if self.mismatch_transition {
                digest("mismatched_transition_hash", HEX_F)
            } else {
                request.transition_hash.clone()
            };

            Ok(NativeUsernameRegistryAdapterEvidenceV1 {
                accepted: !self.reject,
                private_beta_writer_label: "phase12d-private-beta-username-writer",
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
                index_projection_authoritative: self.authoritative_projection,
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

    fn other_challenge_id() -> ChallengeIdV1 {
        ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_F}")).expect("other challenge id")
    }

    fn passport_id() -> PassportIdV1 {
        PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_B}")).expect("passport id")
    }

    fn recipient_passport_id() -> PassportIdV1 {
        PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_E}"))
            .expect("recipient passport id")
    }

    fn device_id() -> DeviceIdV1 {
        DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}")).expect("device id")
    }

    fn registry_draft(
        transition_kind: NativeUsernameRegistryTransitionKind,
    ) -> NativeUsernameRegistryTransitionDraftV1 {
        let (current_owner_passport_id, recipient, signer_public_key_label, root_or_admin) =
            match transition_kind {
                NativeUsernameRegistryTransitionKind::Claim => {
                    (None, None, "public-key:v1:phase12d-device", false)
                }
                NativeUsernameRegistryTransitionKind::Transfer => (
                    Some(passport_id()),
                    Some(recipient_passport_id()),
                    "public-key:v1:phase12d-root",
                    true,
                ),
                NativeUsernameRegistryTransitionKind::Release => (
                    Some(passport_id()),
                    None,
                    "public-key:v1:phase12d-root",
                    true,
                ),
            };

        NativeUsernameRegistryTransitionDraftV1 {
            contract_domain: PHASE12A_USERNAME_REGISTRY_CONTRACT_DOMAIN,
            contract_version: PHASE12A_USERNAME_REGISTRY_CONTRACT_VERSION,
            transition_kind,
            username_input: "@Crab_User7",
            canonical_username: username("crab_user7"),
            passport_id: passport_id(),
            device_id: Some(device_id()),
            current_owner_passport_id,
            recipient_passport_id: recipient,
            current_primary_username: None,
            challenge_id: challenge_id(),
            operation_body_hash: digest("operation_body_hash", HEX_D),
            transition_hash: digest("username_transition_hash", HEX_E),
            idempotency_key_hash: digest("idempotency_key_hash", HEX_A),
            fresh_proof_transcript_hash: digest("fresh_proof_transcript_hash", HEX_B),
            signer_public_key_label,
            transition_requested_at_ms: 1_100_000,
            transition_expires_at_ms: 1_160_000,
            release_cooldown_until_ms: Some(1_000_000),
            proof_fresh: true,
            root_or_admin_fresh_proof_present: root_or_admin,
            policy_reserved_name: false,
            policy_rate_limited: false,
            policy_existing_primary_for_passport: false,
            policy_username_already_owned_by_other: false,
            expects_single_writer_private_beta: true,
            index_projection_only: true,
            contract_only: true,
            treats_username_as_real_or_legal_name_authority: false,
            treats_username_as_recovery_authority: false,
            treats_username_as_root_authority: false,
            treats_username_as_wallet_or_ledger_authority: false,
            treats_index_projection_as_authority: false,
            requests_durable_username_writer_execution: false,
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

    fn adapter_draft(
        decision: &svc_passport::native::NativeUsernameRegistryTransitionDecisionV1,
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

    fn projection_kind(
        transition_kind: NativeUsernameRegistryTransitionKind,
    ) -> NativeUsernameIndexProjectionKind {
        match transition_kind {
            NativeUsernameRegistryTransitionKind::Claim
            | NativeUsernameRegistryTransitionKind::Transfer => {
                NativeUsernameIndexProjectionKind::ExactLookupUpsert
            }
            NativeUsernameRegistryTransitionKind::Release => {
                NativeUsernameIndexProjectionKind::ExactLookupDelete
            }
        }
    }

    fn projected_owner(
        envelope: &svc_passport::native::NativeUsernameRegistryAdapterEnvelopeV1,
    ) -> Option<PassportIdV1> {
        match envelope.transition_kind {
            NativeUsernameRegistryTransitionKind::Claim => Some(envelope.passport_id.clone()),
            NativeUsernameRegistryTransitionKind::Transfer => {
                envelope.recipient_passport_id.clone()
            }
            NativeUsernameRegistryTransitionKind::Release => None,
        }
    }

    fn projection_draft(
        envelope: &svc_passport::native::NativeUsernameRegistryAdapterEnvelopeV1,
    ) -> NativeUsernameIndexProjectionDraftV1 {
        NativeUsernameIndexProjectionDraftV1 {
            contract_domain: PHASE12C_USERNAME_INDEX_PROJECTION_DOMAIN,
            contract_version: PHASE12C_USERNAME_INDEX_PROJECTION_VERSION,
            projection_kind: projection_kind(envelope.transition_kind),
            transition_kind: envelope.transition_kind,
            canonical_username: envelope.canonical_username.clone(),
            canonical_handle: envelope.canonical_handle.clone(),
            lookup_key: format!("username/exact/{}", envelope.canonical_handle),
            passport_id: envelope.passport_id.clone(),
            projected_owner_passport_id: projected_owner(envelope),
            device_id: envelope.device_id.clone(),
            current_owner_passport_id: envelope.current_owner_passport_id.clone(),
            recipient_passport_id: envelope.recipient_passport_id.clone(),
            challenge_id: envelope.challenge_id.clone(),
            operation_body_hash: envelope.operation_body_hash.clone(),
            transition_hash: envelope.transition_hash.clone(),
            idempotency_key_hash: envelope.idempotency_key_hash.clone(),
            fresh_proof_transcript_hash: envelope.fresh_proof_transcript_hash.clone(),
            projection_event_hash: digest("projection_event_hash", HEX_C),
            lookup_key_hash: digest("lookup_key_hash", HEX_D),
            signer_public_key_label: envelope.signer_public_key_label,
            private_beta_writer_label: envelope.private_beta_writer_label,
            transition_requested_at_ms: envelope.transition_requested_at_ms,
            transition_expires_at_ms: envelope.transition_expires_at_ms,
            projection_queued_at_ms: envelope.transition_requested_at_ms + 1,
            expects_username_finalized_by_adapter: true,
            expects_index_projection_queued_by_adapter: true,
            expects_index_projection_non_authoritative: true,
            projection_only: true,
            contract_only: true,
            treats_index_as_username_ownership_authority: false,
            treats_index_as_real_or_legal_name_authority: false,
            treats_index_as_recovery_authority: false,
            treats_index_as_root_authority: false,
            treats_index_as_wallet_or_ledger_authority: false,
            requests_namespace_route: false,
            requests_index_writer_execution_inside_native: false,
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
    fn phase12d_acceptance_label_is_locked() {
        assert_eq!(
            PHASE12D_ACCEPTANCE_LABEL,
            "NATIVE_PASSPORT_PHASE12D_USERNAME_PRIVATE_BETA_ACCEPTANCE"
        );
    }

    #[test]
    fn phase12d_accepts_phase12_postures_and_surface_chain_without_runtime_authority() {
        let registry_posture = native_username_registry_contract_posture();
        let adapter_posture = native_username_registry_adapter_posture();
        let projection_posture = native_username_index_projection_posture();

        assert_eq!(
            &PHASE12B_ENABLED_SURFACES[..PHASE12A_ENABLED_SURFACES.len()],
            PHASE12A_ENABLED_SURFACES
        );
        assert_eq!(
            &PHASE12C_ENABLED_SURFACES[..PHASE12B_ENABLED_SURFACES.len()],
            PHASE12B_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE12C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::UsernamePrivateBetaIndexProjectionDto)
        );

        assert!(registry_posture.username_registry_contract_added);
        assert!(registry_posture.canonical_ron_naming_reuse);
        assert!(registry_posture.one_primary_username_per_passport_review_added);
        assert!(adapter_posture.username_registry_adapter_added);
        assert!(adapter_posture.injected_private_beta_writer_added);
        assert!(projection_posture.username_index_projection_contract_added);
        assert!(projection_posture.exact_lookup_projection_added);

        assert!(!registry_posture.durable_username_writer_execution_added);
        assert!(!adapter_posture.durable_username_writer_implemented_inside_native);
        assert!(!projection_posture.index_writer_execution_inside_native_added);
        assert!(!projection_posture.index_as_username_ownership_authority_added);
        assert!(!projection_posture.namespace_routes_added);
        assert!(!projection_posture.storage_mutation_inside_native_added);
        assert!(!projection_posture.wallet_or_ledger_mutation_added);
        assert!(!projection_posture.native_secret_implementation_added);
    }

    #[test]
    fn phase12d_accepts_claim_transfer_release_from_registry_to_projection() {
        for transition_kind in [
            NativeUsernameRegistryTransitionKind::Claim,
            NativeUsernameRegistryTransitionKind::Transfer,
            NativeUsernameRegistryTransitionKind::Release,
        ] {
            let registry_decision = review_native_username_registry_transition_contract(
                registry_draft(transition_kind),
            )
            .expect("registry contract should review");

            let adapter_envelope = execute_native_username_registry_adapter(
                &registry_decision,
                adapter_draft(&registry_decision),
                &AcceptanceUsernameWriter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch_transition: false,
                    omit_finalization: false,
                    authoritative_projection: false,
                },
            )
            .expect("private-beta writer adapter should accept");

            let projection_decision = review_native_username_index_projection_contract(
                &adapter_envelope,
                projection_draft(&adapter_envelope),
            )
            .expect("exact lookup projection contract should review");

            assert_eq!(projection_decision.transition_kind, transition_kind);
            assert_eq!(
                projection_decision.canonical_username.as_str(),
                "crab_user7"
            );
            assert_eq!(projection_decision.canonical_handle, "@crab_user7");
            assert_eq!(projection_decision.lookup_key, "username/exact/@crab_user7");
            assert_eq!(
                projection_decision.projection_kind,
                projection_kind(transition_kind)
            );
            assert_eq!(
                projection_decision.projected_owner_passport_id,
                projected_owner(&adapter_envelope)
            );
            assert!(projection_decision.projection_only);
            assert!(!projection_decision.username_authority_changed);
            assert!(!projection_decision.index_writer_called);
            assert!(!projection_decision.storage_mutated_inside_native);
            assert!(!projection_decision.runtime_io_performed);
            assert!(!projection_decision.wallet_or_ledger_mutated);
            assert!(!projection_decision.secret_material_exposed);
            assert!(projection_decision.contract_only);
        }
    }

    #[test]
    fn phase12d_rejects_reserved_stale_writer_and_authoritative_projection_drift() {
        let mut reserved = registry_draft(NativeUsernameRegistryTransitionKind::Claim);
        reserved.username_input = "@wallet";
        assert_eq!(
            review_native_username_registry_transition_contract(reserved),
            Err(NativeUsernameRegistryTransitionReviewError::CanonicalUsernameRejected)
        );

        let mut stale = registry_draft(NativeUsernameRegistryTransitionKind::Transfer);
        stale.proof_fresh = false;
        assert_eq!(
            review_native_username_registry_transition_contract(stale),
            Err(NativeUsernameRegistryTransitionReviewError::MissingFreshProof)
        );

        let registry_decision = review_native_username_registry_transition_contract(
            registry_draft(NativeUsernameRegistryTransitionKind::Claim),
        )
        .expect("registry contract should review");

        assert_eq!(
            execute_native_username_registry_adapter(
                &registry_decision,
                adapter_draft(&registry_decision),
                &AcceptanceUsernameWriter {
                    reject: true,
                    unsafe_evidence: false,
                    mismatch_transition: false,
                    omit_finalization: false,
                    authoritative_projection: false,
                },
            ),
            Err(NativeUsernameRegistryAdapterReviewError::WriterRejected)
        );

        assert_eq!(
            execute_native_username_registry_adapter(
                &registry_decision,
                adapter_draft(&registry_decision),
                &AcceptanceUsernameWriter {
                    reject: false,
                    unsafe_evidence: false,
                    mismatch_transition: true,
                    omit_finalization: false,
                    authoritative_projection: false,
                },
            ),
            Err(NativeUsernameRegistryAdapterReviewError::WriterEvidenceMismatch)
        );

        assert_eq!(
            execute_native_username_registry_adapter(
                &registry_decision,
                adapter_draft(&registry_decision),
                &AcceptanceUsernameWriter {
                    reject: false,
                    unsafe_evidence: true,
                    mismatch_transition: false,
                    omit_finalization: false,
                    authoritative_projection: false,
                },
            ),
            Err(NativeUsernameRegistryAdapterReviewError::WriterEvidenceUnsafe)
        );

        let mut bad_adapter_envelope = execute_native_username_registry_adapter(
            &registry_decision,
            adapter_draft(&registry_decision),
            &AcceptanceUsernameWriter {
                reject: false,
                unsafe_evidence: false,
                mismatch_transition: false,
                omit_finalization: false,
                authoritative_projection: false,
            },
        )
        .expect("adapter envelope should exist");

        bad_adapter_envelope.index_projection_authoritative = true;
        assert_eq!(
            review_native_username_index_projection_contract(
                &bad_adapter_envelope,
                projection_draft(&bad_adapter_envelope),
            ),
            Err(NativeUsernameIndexProjectionReviewError::RegistryAdapterEnvelopeUnsafe)
        );

        let good_adapter_envelope = execute_native_username_registry_adapter(
            &registry_decision,
            adapter_draft(&registry_decision),
            &AcceptanceUsernameWriter {
                reject: false,
                unsafe_evidence: false,
                mismatch_transition: false,
                omit_finalization: false,
                authoritative_projection: false,
            },
        )
        .expect("adapter envelope should exist");

        let mut wrong_projection = projection_draft(&good_adapter_envelope);
        wrong_projection.challenge_id = other_challenge_id();
        assert_eq!(
            review_native_username_index_projection_contract(
                &good_adapter_envelope,
                wrong_projection,
            ),
            Err(NativeUsernameIndexProjectionReviewError::ChallengeIdMismatch)
        );
    }

    #[test]
    fn phase12d_acceptance_sources_preserve_username_projection_authority_boundaries() {
        let registry_source =
            fs::read_to_string(repo_file("src/native/username_registry.rs")).expect("registry");
        let adapter_source =
            fs::read_to_string(repo_file("src/native/username_registry_adapter.rs"))
                .expect("adapter");
        let projection_source =
            fs::read_to_string(repo_file("src/native/username_index_projection.rs"))
                .expect("projection");
        let native_username_source =
            fs::read_to_string(repo_file("src/native/username.rs")).expect("username");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(registry_source.contains("UsernameV1::parse"));
        assert!(adapter_source.contains("execute_native_username_registry_adapter"));
        assert!(projection_source.contains("review_native_username_index_projection_contract"));
        assert!(native_username_source.contains("pub type UsernameV1 = ron_naming::UsernameV1"));
        assert!(native_mod.contains("UsernamePrivateBetaRegistryContractDto"));
        assert!(native_mod.contains("UsernamePrivateBetaRegistryAdapterDto"));
        assert!(native_mod.contains("UsernamePrivateBetaIndexProjectionDto"));

        let combined = format!(
            "{registry_source}\n{adapter_source}\n{projection_source}\n{native_username_source}\n{native_mod}"
        );

        assert!(!combined.contains("serde_json"));

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
                !combined.contains(forbidden_runtime_pattern),
                "Phase 12D username private-beta acceptance sources must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
