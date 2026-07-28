#[cfg(not(feature = "native-passport"))]
#[test]
fn phase12a_username_registry_contract_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native username registry contract surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_username_registry_contract_posture,
        review_native_username_registry_transition_contract, B3DigestHex, ChallengeIdV1,
        DeviceIdV1, NativePassportSurface, NativeUsernameRegistryTransitionDraftV1,
        NativeUsernameRegistryTransitionKind, NativeUsernameRegistryTransitionReviewError,
        PassportIdV1, UsernameV1, NATIVE_PASSPORT_PHASE12A_LABEL, PHASE11D_ENABLED_SURFACES,
        PHASE12A_ENABLED_SURFACES, PHASE12A_FORBIDDEN_USERNAME_REGISTRY_AUTHORITY_FLAGS,
        PHASE12A_MAX_USERNAME_TRANSITION_TTL_MS, PHASE12A_USERNAME_REGISTRY_CONTRACT_DOMAIN,
        PHASE12A_USERNAME_REGISTRY_CONTRACT_VERSION,
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

    fn claim_draft() -> NativeUsernameRegistryTransitionDraftV1 {
        NativeUsernameRegistryTransitionDraftV1 {
            contract_domain: PHASE12A_USERNAME_REGISTRY_CONTRACT_DOMAIN,
            contract_version: PHASE12A_USERNAME_REGISTRY_CONTRACT_VERSION,
            transition_kind: NativeUsernameRegistryTransitionKind::Claim,
            username_input: "@Crab_User7",
            canonical_username: username("crab_user7"),
            passport_id: passport_id(),
            device_id: Some(device_id()),
            current_owner_passport_id: None,
            recipient_passport_id: None,
            current_primary_username: None,
            challenge_id: challenge_id(),
            operation_body_hash: digest("operation_body_hash", HEX_D),
            transition_hash: digest("username_transition_hash", HEX_E),
            idempotency_key_hash: digest("idempotency_key_hash", HEX_A),
            fresh_proof_transcript_hash: digest("fresh_proof_transcript_hash", HEX_B),
            signer_public_key_label: "public-key:v1:phase12a-device",
            transition_requested_at_ms: 1_100_000,
            transition_expires_at_ms: 1_160_000,
            release_cooldown_until_ms: None,
            proof_fresh: true,
            root_or_admin_fresh_proof_present: false,
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

    fn transfer_draft() -> NativeUsernameRegistryTransitionDraftV1 {
        let mut draft = claim_draft();
        draft.transition_kind = NativeUsernameRegistryTransitionKind::Transfer;
        draft.username_input = "@crab_user7";
        draft.current_owner_passport_id = Some(passport_id());
        draft.recipient_passport_id = Some(other_passport_id());
        draft.root_or_admin_fresh_proof_present = true;
        draft.signer_public_key_label = "public-key:v1:phase12a-root";
        draft
    }

    fn release_draft() -> NativeUsernameRegistryTransitionDraftV1 {
        let mut draft = claim_draft();
        draft.transition_kind = NativeUsernameRegistryTransitionKind::Release;
        draft.username_input = "@crab_user7";
        draft.current_owner_passport_id = Some(passport_id());
        draft.root_or_admin_fresh_proof_present = true;
        draft.signer_public_key_label = "public-key:v1:phase12a-root";
        draft.release_cooldown_until_ms = Some(1_000_000);
        draft
    }

    #[test]
    fn phase12a_label_and_posture_are_locked() {
        let posture = native_username_registry_contract_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE12A_LABEL,
            "NATIVE_PASSPORT_PHASE12A_USERNAME_PRIVATE_BETA_REGISTRY_CONTRACT"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE12A_LABEL);
        assert!(posture.username_registry_contract_added);
        assert_eq!(posture.canonical_username_owner, "ron-naming");
        assert!(posture.canonical_ron_naming_reuse);
        assert!(posture.one_primary_username_per_passport_review_added);
        assert!(posture.single_writer_private_beta_review_added);
        assert!(posture.idempotency_binding_added);
        assert!(!posture.durable_username_writer_execution_added);
        assert!(!posture.index_projection_mutation_added);
        assert!(!posture.routes_added);
        assert!(!posture.storage_mutation_inside_native_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);
        assert_eq!(PHASE12A_MAX_USERNAME_TRANSITION_TTL_MS, 120_000);

        for forbidden in [
            "username_as_real_or_legal_name_authority",
            "username_as_recovery_authority",
            "username_as_wallet_authority",
            "index_projection_authority",
            "durable_username_writer_execution",
            "storage_mutation_inside_svc_passport_native",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE12A_FORBIDDEN_USERNAME_REGISTRY_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase12a_surfaces_extend_phase11d_without_back_mutating_it() {
        assert_eq!(
            PHASE12A_ENABLED_SURFACES.len(),
            PHASE11D_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE12A_ENABLED_SURFACES[..PHASE11D_ENABLED_SURFACES.len()],
            PHASE11D_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE12A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::UsernamePrivateBetaRegistryContractDto)
        );
        assert_eq!(
            PHASE11D_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::DeviceBoundCapabilityAdapterDto)
        );
    }

    #[test]
    fn phase12a_accepts_canonical_claim_contract_without_finalization_or_index_authority() {
        let decision = review_native_username_registry_transition_contract(claim_draft())
            .expect("claim contract should review");

        assert_eq!(
            decision.contract_domain,
            PHASE12A_USERNAME_REGISTRY_CONTRACT_DOMAIN
        );
        assert_eq!(
            decision.transition_kind,
            NativeUsernameRegistryTransitionKind::Claim
        );
        assert_eq!(decision.canonical_username.as_str(), "crab_user7");
        assert_eq!(decision.canonical_handle, "@crab_user7");
        assert_eq!(decision.passport_id, passport_id());
        assert!(decision.current_owner_passport_id.is_none());
        assert!(decision.recipient_passport_id.is_none());
        assert!(decision.single_writer_private_beta_reviewed);
        assert!(decision.index_projection_only);
        assert!(!decision.username_finalized);
        assert!(!decision.durable_writer_called);
        assert!(!decision.index_projection_mutated);
        assert!(!decision.storage_mutated_inside_native);
        assert!(!decision.wallet_or_ledger_mutated);
        assert!(!decision.secret_material_exposed);
        assert!(decision.contract_only);
    }

    #[test]
    fn phase12a_accepts_transfer_and_release_only_with_current_owner_fresh_root_or_admin_proof() {
        let transfer = review_native_username_registry_transition_contract(transfer_draft())
            .expect("transfer contract should review");
        assert_eq!(
            transfer.transition_kind,
            NativeUsernameRegistryTransitionKind::Transfer
        );
        assert_eq!(transfer.current_owner_passport_id, Some(passport_id()));
        assert_eq!(transfer.recipient_passport_id, Some(other_passport_id()));

        let release = review_native_username_registry_transition_contract(release_draft())
            .expect("release contract should review");
        assert_eq!(
            release.transition_kind,
            NativeUsernameRegistryTransitionKind::Release
        );
        assert_eq!(release.current_owner_passport_id, Some(passport_id()));
        assert!(release.recipient_passport_id.is_none());

        let mut transfer_without_root = transfer_draft();
        transfer_without_root.root_or_admin_fresh_proof_present = false;
        assert_eq!(
            review_native_username_registry_transition_contract(transfer_without_root),
            Err(NativeUsernameRegistryTransitionReviewError::MissingRootOrAdminFreshProof)
        );

        let mut release_wrong_owner = release_draft();
        release_wrong_owner.current_owner_passport_id = Some(other_passport_id());
        assert_eq!(
            review_native_username_registry_transition_contract(release_wrong_owner),
            Err(NativeUsernameRegistryTransitionReviewError::CurrentOwnerMismatch)
        );
    }

    #[test]
    fn phase12a_rejects_reserved_rate_limited_existing_owner_and_index_authority_drift() {
        let mut reserved = claim_draft();
        reserved.username_input = "@wallet";
        reserved.canonical_username = username("crab_user7");
        assert_eq!(
            review_native_username_registry_transition_contract(reserved),
            Err(NativeUsernameRegistryTransitionReviewError::CanonicalUsernameRejected)
        );

        let mut mismatch = claim_draft();
        mismatch.username_input = "@Another_User";
        assert_eq!(
            review_native_username_registry_transition_contract(mismatch),
            Err(NativeUsernameRegistryTransitionReviewError::CanonicalUsernameMismatch)
        );

        let mut rate_limited = claim_draft();
        rate_limited.policy_rate_limited = true;
        assert_eq!(
            review_native_username_registry_transition_contract(rate_limited),
            Err(NativeUsernameRegistryTransitionReviewError::RateLimited)
        );

        let mut already_has_primary = claim_draft();
        already_has_primary.current_primary_username = Some(username("oldhandle"));
        assert_eq!(
            review_native_username_registry_transition_contract(already_has_primary),
            Err(NativeUsernameRegistryTransitionReviewError::PassportAlreadyHasPrimaryUsername)
        );

        let mut owned_by_other = claim_draft();
        owned_by_other.policy_username_already_owned_by_other = true;
        assert_eq!(
            review_native_username_registry_transition_contract(owned_by_other),
            Err(NativeUsernameRegistryTransitionReviewError::UsernameAlreadyOwnedByOther)
        );

        let mut index_authority = claim_draft();
        index_authority.treats_index_projection_as_authority = true;
        assert_eq!(
            review_native_username_registry_transition_contract(index_authority),
            Err(NativeUsernameRegistryTransitionReviewError::IndexProjectionTreatedAsAuthority)
        );

        let mut cooldown = release_draft();
        cooldown.release_cooldown_until_ms = Some(cooldown.transition_requested_at_ms + 1);
        assert_eq!(
            review_native_username_registry_transition_contract(cooldown),
            Err(NativeUsernameRegistryTransitionReviewError::ReleaseCooldownActive)
        );
    }

    #[test]
    fn phase12a_rejects_all_unsafe_username_registry_authority_flags() {
        let mutators: &[fn(&mut NativeUsernameRegistryTransitionDraftV1)] = &[
            |draft| draft.treats_username_as_real_or_legal_name_authority = true,
            |draft| draft.treats_username_as_recovery_authority = true,
            |draft| draft.treats_username_as_root_authority = true,
            |draft| draft.treats_username_as_wallet_or_ledger_authority = true,
            |draft| draft.requests_durable_username_writer_execution = true,
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
            let mut draft = claim_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_username_registry_transition_contract(draft),
                Err(NativeUsernameRegistryTransitionReviewError::UnsafeUsernameRegistryAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase12a_source_reuses_ron_naming_without_routes_storage_index_wallet_ledger_or_secrets() {
        let source = fs::read_to_string(repo_file("src/native/username_registry.rs"))
            .expect("username registry source");
        let native_username_source = fs::read_to_string(repo_file("src/native/username.rs"))
            .expect("native username source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("UsernameV1::parse"));
        assert!(native_username_source.contains("pub type UsernameV1 = ron_naming::UsernameV1"));
        assert!(source.contains("review_native_username_registry_transition_contract"));
        assert!(native_mod.contains("UsernamePrivateBetaRegistryContractDto"));
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
                "Phase 12A username registry source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
