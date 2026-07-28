#[cfg(not(feature = "native-passport"))]
#[test]
fn phase12c_username_index_projection_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native username index projection surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_username_index_projection_posture, review_native_username_index_projection_contract,
        B3DigestHex, ChallengeIdV1, DeviceIdV1, NativePassportSurface,
        NativeUsernameIndexProjectionDraftV1, NativeUsernameIndexProjectionKind,
        NativeUsernameIndexProjectionReviewError, NativeUsernameRegistryAdapterEnvelopeV1,
        NativeUsernameRegistryTransitionKind, PassportIdV1, UsernameV1,
        NATIVE_PASSPORT_PHASE12C_LABEL, PHASE12B_ENABLED_SURFACES,
        PHASE12B_USERNAME_REGISTRY_ADAPTER_DOMAIN, PHASE12B_USERNAME_REGISTRY_ADAPTER_VERSION,
        PHASE12C_ENABLED_SURFACES, PHASE12C_FORBIDDEN_USERNAME_INDEX_PROJECTION_AUTHORITY_FLAGS,
        PHASE12C_USERNAME_INDEX_PROJECTION_DOMAIN, PHASE12C_USERNAME_INDEX_PROJECTION_VERSION,
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

    fn adapter_envelope(
        transition_kind: NativeUsernameRegistryTransitionKind,
    ) -> NativeUsernameRegistryAdapterEnvelopeV1 {
        let (current_owner_passport_id, recipient_passport_id) = match transition_kind {
            NativeUsernameRegistryTransitionKind::Claim => (None, None),
            NativeUsernameRegistryTransitionKind::Transfer => {
                (Some(passport_id()), Some(recipient_passport_id()))
            }
            NativeUsernameRegistryTransitionKind::Release => (Some(passport_id()), None),
        };

        NativeUsernameRegistryAdapterEnvelopeV1 {
            contract_domain: PHASE12B_USERNAME_REGISTRY_ADAPTER_DOMAIN,
            contract_version: PHASE12B_USERNAME_REGISTRY_ADAPTER_VERSION,
            transition_kind,
            canonical_username: username("crab_user7"),
            canonical_handle: "@crab_user7".to_string(),
            passport_id: passport_id(),
            device_id: Some(device_id()),
            current_owner_passport_id,
            recipient_passport_id,
            challenge_id: challenge_id(),
            operation_body_hash: digest("operation_body_hash", HEX_D),
            transition_hash: digest("username_transition_hash", HEX_E),
            idempotency_key_hash: digest("idempotency_key_hash", HEX_A),
            fresh_proof_transcript_hash: digest("fresh_proof_transcript_hash", HEX_B),
            signer_public_key_label: "public-key:v1:phase12c-device",
            private_beta_writer_label: "phase12c-private-beta-username-writer",
            transition_requested_at_ms: 1_100_000,
            transition_expires_at_ms: 1_160_000,
            username_finalized_by_adapter: true,
            index_projection_queued_by_adapter: true,
            index_projection_authoritative: false,
            storage_mutated_inside_native: false,
            runtime_io_performed: false,
            wallet_or_ledger_mutated: false,
            secret_material_exposed: false,
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

    fn projected_owner(envelope: &NativeUsernameRegistryAdapterEnvelopeV1) -> Option<PassportIdV1> {
        match envelope.transition_kind {
            NativeUsernameRegistryTransitionKind::Claim => Some(envelope.passport_id.clone()),
            NativeUsernameRegistryTransitionKind::Transfer => {
                envelope.recipient_passport_id.clone()
            }
            NativeUsernameRegistryTransitionKind::Release => None,
        }
    }

    fn projection_draft(
        envelope: &NativeUsernameRegistryAdapterEnvelopeV1,
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
    fn phase12c_label_and_posture_are_locked() {
        let posture = native_username_index_projection_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE12C_LABEL,
            "NATIVE_PASSPORT_PHASE12C_USERNAME_PRIVATE_BETA_INDEX_PROJECTION"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE12C_LABEL);
        assert!(posture.username_index_projection_contract_added);
        assert!(posture.exact_lookup_projection_added);
        assert!(posture.projection_event_binding_added);
        assert!(posture.lookup_key_binding_added);
        assert!(!posture.index_writer_execution_inside_native_added);
        assert!(!posture.namespace_routes_added);
        assert!(!posture.storage_mutation_inside_native_added);
        assert!(!posture.index_as_username_ownership_authority_added);
        assert!(!posture.index_as_real_or_legal_name_authority_added);
        assert!(!posture.index_as_recovery_authority_added);
        assert!(!posture.index_as_wallet_or_ledger_authority_added);
        assert!(!posture.secret_key_access_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "index_as_username_ownership_authority",
            "index_as_real_or_legal_name_authority",
            "index_as_recovery_authority",
            "namespace_route_added",
            "index_writer_execution_inside_svc_passport_native",
            "storage_mutation_inside_svc_passport_native",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(
                PHASE12C_FORBIDDEN_USERNAME_INDEX_PROJECTION_AUTHORITY_FLAGS.contains(&forbidden)
            );
        }
    }

    #[test]
    fn phase12c_surfaces_extend_phase12b_without_back_mutating_it() {
        assert_eq!(
            PHASE12C_ENABLED_SURFACES.len(),
            PHASE12B_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE12C_ENABLED_SURFACES[..PHASE12B_ENABLED_SURFACES.len()],
            PHASE12B_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE12C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::UsernamePrivateBetaIndexProjectionDto)
        );
        assert_eq!(
            PHASE12B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::UsernamePrivateBetaRegistryAdapterDto)
        );
    }

    #[test]
    fn phase12c_reviews_claim_transfer_and_release_projection_contracts_without_index_authority() {
        for transition_kind in [
            NativeUsernameRegistryTransitionKind::Claim,
            NativeUsernameRegistryTransitionKind::Transfer,
            NativeUsernameRegistryTransitionKind::Release,
        ] {
            let envelope = adapter_envelope(transition_kind);
            let decision = review_native_username_index_projection_contract(
                &envelope,
                projection_draft(&envelope),
            )
            .expect("adapter envelope should review for exact lookup projection");

            assert_eq!(
                decision.contract_domain,
                PHASE12C_USERNAME_INDEX_PROJECTION_DOMAIN
            );
            assert_eq!(decision.transition_kind, transition_kind);
            assert_eq!(decision.projection_kind, projection_kind(transition_kind));
            assert_eq!(decision.canonical_handle, "@crab_user7");
            assert_eq!(decision.lookup_key, "username/exact/@crab_user7");
            assert_eq!(decision.passport_id, envelope.passport_id);
            assert_eq!(
                decision.projected_owner_passport_id,
                projected_owner(&envelope)
            );
            assert!(decision.projection_only);
            assert!(!decision.username_authority_changed);
            assert!(!decision.index_writer_called);
            assert!(!decision.storage_mutated_inside_native);
            assert!(!decision.runtime_io_performed);
            assert!(!decision.wallet_or_ledger_mutated);
            assert!(!decision.secret_material_exposed);
            assert!(decision.contract_only);
        }
    }

    #[test]
    fn phase12c_rejects_unfinalized_authoritative_or_mismatched_projection_inputs() {
        let mut unfinalized = adapter_envelope(NativeUsernameRegistryTransitionKind::Claim);
        unfinalized.username_finalized_by_adapter = false;
        assert_eq!(
            review_native_username_index_projection_contract(
                &unfinalized,
                projection_draft(&unfinalized),
            ),
            Err(NativeUsernameIndexProjectionReviewError::RegistryAdapterEnvelopeNotFinalized)
        );

        let mut authoritative = adapter_envelope(NativeUsernameRegistryTransitionKind::Claim);
        authoritative.index_projection_authoritative = true;
        assert_eq!(
            review_native_username_index_projection_contract(
                &authoritative,
                projection_draft(&authoritative),
            ),
            Err(NativeUsernameIndexProjectionReviewError::RegistryAdapterEnvelopeUnsafe)
        );

        let envelope = adapter_envelope(NativeUsernameRegistryTransitionKind::Transfer);

        let mut wrong_kind = projection_draft(&envelope);
        wrong_kind.projection_kind = NativeUsernameIndexProjectionKind::ExactLookupDelete;
        assert_eq!(
            review_native_username_index_projection_contract(&envelope, wrong_kind),
            Err(NativeUsernameIndexProjectionReviewError::ProjectionKindMismatch)
        );

        let mut wrong_lookup = projection_draft(&envelope);
        wrong_lookup.lookup_key = "username/exact/@wrong".to_string();
        assert_eq!(
            review_native_username_index_projection_contract(&envelope, wrong_lookup),
            Err(NativeUsernameIndexProjectionReviewError::LookupKeyMismatch)
        );

        let mut wrong_owner = projection_draft(&envelope);
        wrong_owner.projected_owner_passport_id = Some(passport_id());
        assert_eq!(
            review_native_username_index_projection_contract(&envelope, wrong_owner),
            Err(NativeUsernameIndexProjectionReviewError::ProjectedOwnerMismatch)
        );

        let mut wrong_challenge = projection_draft(&envelope);
        wrong_challenge.challenge_id = other_challenge_id();
        assert_eq!(
            review_native_username_index_projection_contract(&envelope, wrong_challenge),
            Err(NativeUsernameIndexProjectionReviewError::ChallengeIdMismatch)
        );
    }

    #[test]
    fn phase12c_rejects_index_authority_flags_with_specific_authority_error() {
        let envelope = adapter_envelope(NativeUsernameRegistryTransitionKind::Claim);

        let mutators: &[fn(&mut NativeUsernameIndexProjectionDraftV1)] = &[
            |draft| draft.treats_index_as_username_ownership_authority = true,
            |draft| draft.treats_index_as_real_or_legal_name_authority = true,
            |draft| draft.treats_index_as_recovery_authority = true,
            |draft| draft.treats_index_as_root_authority = true,
            |draft| draft.treats_index_as_wallet_or_ledger_authority = true,
        ];

        for mutate in mutators {
            let mut draft = projection_draft(&envelope);
            mutate(&mut draft);
            assert_eq!(
                review_native_username_index_projection_contract(&envelope, draft),
                Err(NativeUsernameIndexProjectionReviewError::IndexProjectionTreatedAsAuthority)
            );
        }
    }

    #[test]
    fn phase12c_rejects_projection_runtime_mutation_flags_as_unsafe() {
        let envelope = adapter_envelope(NativeUsernameRegistryTransitionKind::Claim);

        let mutators: &[fn(&mut NativeUsernameIndexProjectionDraftV1)] = &[
            |draft| draft.requests_namespace_route = true,
            |draft| draft.requests_index_writer_execution_inside_native = true,
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
            let mut draft = projection_draft(&envelope);
            mutate(&mut draft);
            assert_eq!(
                review_native_username_index_projection_contract(&envelope, draft),
                Err(NativeUsernameIndexProjectionReviewError::UnsafeUsernameIndexProjectionAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase12c_source_remains_projection_contract_only_without_routes_storage_index_wallet_ledger_or_secrets(
    ) {
        let source = fs::read_to_string(repo_file("src/native/username_index_projection.rs"))
            .expect("username index projection source");
        let adapter_source =
            fs::read_to_string(repo_file("src/native/username_registry_adapter.rs"))
                .expect("username registry adapter source");
        let registry_source = fs::read_to_string(repo_file("src/native/username_registry.rs"))
            .expect("registry source");
        let native_username_source = fs::read_to_string(repo_file("src/native/username.rs"))
            .expect("native username source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeUsernameIndexProjectionDraftV1"));
        assert!(source.contains("review_native_username_index_projection_contract"));
        assert!(adapter_source.contains("execute_native_username_registry_adapter"));
        assert!(registry_source.contains("UsernameV1::parse"));
        assert!(native_username_source.contains("pub type UsernameV1 = ron_naming::UsernameV1"));
        assert!(native_mod.contains("UsernamePrivateBetaIndexProjectionDto"));
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
                "Phase 12C username index projection source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
