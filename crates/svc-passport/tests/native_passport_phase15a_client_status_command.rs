#[cfg(not(feature = "native-passport"))]
#[test]
fn phase15a_client_status_command_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native client status command surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_client_status_command_posture, review_native_client_status_command,
        NativeClientStatusCapabilityState, NativeClientStatusCommandDraftV1,
        NativeClientStatusCommandReviewError, NativeClientStatusLockState, NativePassportSurface,
        NATIVE_PASSPORT_PHASE15A_LABEL, PHASE14D_ENABLED_SURFACES,
        PHASE15A_CLIENT_STATUS_COMMAND_DOMAIN, PHASE15A_CLIENT_STATUS_COMMAND_VERSION,
        PHASE15A_ENABLED_SURFACES, PHASE15A_EXPECTED_DESKTOP_COMMANDS,
        PHASE15A_FORBIDDEN_DESKTOP_COMMANDS, PHASE15A_FORBIDDEN_STATUS_COMMAND_AUTHORITY_FLAGS,
        PHASE15A_PASSPORT_STATUS_COMMAND,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn safe_draft() -> NativeClientStatusCommandDraftV1 {
        NativeClientStatusCommandDraftV1 {
            contract_domain: PHASE15A_CLIENT_STATUS_COMMAND_DOMAIN,
            contract_version: PHASE15A_CLIENT_STATUS_COMMAND_VERSION,
            command_name: PHASE15A_PASSPORT_STATUS_COMMAND,
            requester_label: "desktop-tauri:passport-panel",
            surface_label: "crablink-desktop",
            native_feature_enabled: true,
            desktop_surface: true,
            redacted_dto_required: true,
            latest_completed_phase_label:
                "NATIVE_PASSPORT_PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE",
            lock_state: NativeClientStatusLockState::Locked,
            capability_state: NativeClientStatusCapabilityState::PresentRedacted,
            has_passport_identifier: true,
            has_device_identifier: true,
            has_confirmed_username: true,
            route_acceptance_green: true,
            local_status_inspection_green: true,
            exposes_recovery_words: false,
            exposes_root_signing_material: false,
            exposes_device_signing_material: false,
            exposes_capability_material: false,
            exposes_vault_material: false,
            requests_unlock: false,
            requests_root_confirmation: false,
            requests_platform_sealer_unseal: false,
            requests_runtime_io: false,
            requests_storage_mutation: false,
            requests_wallet_or_ledger_mutation: false,
            requests_arbitrary_scope_issue: false,
            requests_policy_disable: false,
        }
    }

    #[test]
    fn phase15a_label_posture_and_command_inventory_are_locked() {
        let posture = native_client_status_command_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE15A_LABEL,
            "NATIVE_PASSPORT_PHASE15A_CLIENT_STATUS_COMMAND"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE15A_LABEL);
        assert!(posture.status_command_contract_added);
        assert!(posture.passport_status_command_named);
        assert!(posture.expected_desktop_command_inventory_added);
        assert!(posture.forbidden_command_inventory_added);
        assert!(posture.redacted_dto_added);
        assert!(posture.local_status_inspection_reused);
        assert!(posture.route_acceptance_reused);
        assert!(!posture.live_tauri_command_added);
        assert!(!posture.create_restore_unlock_command_implementation_added);
        assert!(!posture.proof_or_capability_command_implementation_added);
        assert!(!posture.username_command_implementation_added);
        assert!(!posture.seed_to_webview_added);
        assert!(!posture.private_key_export_added);
        assert!(!posture.device_key_export_added);
        assert!(!posture.raw_capability_export_added);
        assert!(!posture.arbitrary_scope_issue_added);
        assert!(!posture.policy_disable_added);
        assert!(!posture.platform_sealer_unseal_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.storage_mutation_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);

        assert_eq!(PHASE15A_PASSPORT_STATUS_COMMAND, "passport_status");
        assert!(PHASE15A_EXPECTED_DESKTOP_COMMANDS.contains(&"passport_status"));
        assert!(PHASE15A_EXPECTED_DESKTOP_COMMANDS.contains(&"passport_create_native"));
        assert!(PHASE15A_EXPECTED_DESKTOP_COMMANDS.contains(&"passport_unlock_operational"));
        assert!(PHASE15A_EXPECTED_DESKTOP_COMMANDS.contains(&"passport_unlock_root"));
        assert!(PHASE15A_EXPECTED_DESKTOP_COMMANDS.contains(&"passport_device_authorize"));
        assert!(PHASE15A_EXPECTED_DESKTOP_COMMANDS.contains(&"passport_username_claim"));
        assert_eq!(PHASE15A_EXPECTED_DESKTOP_COMMANDS.len(), 18);

        for forbidden in [
            "passport_get_seed_to_webview",
            "passport_export_private_key",
            "passport_get_device_private_key",
            "passport_get_raw_capability",
            "passport_issue_arbitrary_scope",
            "passport_disable_policy",
        ] {
            assert!(PHASE15A_FORBIDDEN_DESKTOP_COMMANDS.contains(&forbidden));
        }

        for forbidden in [
            "seed_to_webview",
            "private_key_export",
            "device_key_export",
            "raw_capability_export",
            "arbitrary_scope_issue",
            "policy_disable",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE15A_FORBIDDEN_STATUS_COMMAND_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase15a_surfaces_extend_phase14d_without_back_mutating_it() {
        assert_eq!(
            PHASE15A_ENABLED_SURFACES.len(),
            PHASE14D_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE15A_ENABLED_SURFACES[..PHASE14D_ENABLED_SURFACES.len()],
            PHASE14D_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE15A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ClientStatusCommandDto)
        );
        assert_eq!(
            PHASE14D_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::GatewayOmnigateRouteMountAcceptanceDto)
        );
    }

    #[test]
    fn phase15a_reviews_redacted_passport_status_command_without_secret_or_unlock_behavior() {
        let decision =
            review_native_client_status_command(safe_draft()).expect("status command review");

        assert_eq!(
            decision.contract_domain,
            PHASE15A_CLIENT_STATUS_COMMAND_DOMAIN
        );
        assert_eq!(decision.command_name, "passport_status");
        assert_eq!(decision.requester_label, "desktop-tauri:passport-panel");
        assert_eq!(decision.surface_label, "crablink-desktop");
        assert_eq!(decision.lock_state, NativeClientStatusLockState::Locked);
        assert_eq!(
            decision.capability_state,
            NativeClientStatusCapabilityState::PresentRedacted
        );
        assert_eq!(decision.redacted_passport_identifier, "REDACTED");
        assert_eq!(decision.redacted_device_identifier, "REDACTED");
        assert_eq!(decision.redacted_username_handle, "REDACTED");
        assert_eq!(decision.redacted_capability_material, "REDACTED");
        assert!(decision.native_feature_enabled);
        assert!(decision.desktop_surface);
        assert!(decision.redacted_dto);
        assert!(decision.route_acceptance_green);
        assert!(decision.local_status_inspection_green);
        assert!(!decision.seed_phrase_exposed);
        assert!(!decision.private_key_exposed);
        assert!(!decision.raw_capability_exposed);
        assert!(!decision.vault_material_exposed);
        assert!(!decision.unlock_performed);
        assert!(!decision.root_confirmation_requested);
        assert!(!decision.platform_sealer_unseal_performed);
        assert!(!decision.runtime_io_performed);
        assert!(!decision.storage_mutated);
        assert!(!decision.wallet_or_ledger_mutated);
        assert!(!decision.arbitrary_scope_issued);
        assert!(!decision.policy_disabled);
        assert!(decision.contract_only);
    }

    #[test]
    fn phase15a_reviews_absent_unlocked_and_revoked_status_shapes_as_redacted_dtos() {
        let mut no_passport = safe_draft();
        no_passport.lock_state = NativeClientStatusLockState::NoPassport;
        no_passport.capability_state = NativeClientStatusCapabilityState::Absent;
        no_passport.has_passport_identifier = false;
        no_passport.has_device_identifier = false;
        no_passport.has_confirmed_username = false;

        let decision =
            review_native_client_status_command(no_passport).expect("no Passport status review");
        assert_eq!(decision.lock_state, NativeClientStatusLockState::NoPassport);
        assert_eq!(decision.redacted_passport_identifier, "ABSENT");
        assert_eq!(decision.redacted_device_identifier, "ABSENT");
        assert_eq!(decision.redacted_username_handle, "ABSENT");
        assert_eq!(decision.redacted_capability_material, "ABSENT");

        let mut operational = safe_draft();
        operational.lock_state = NativeClientStatusLockState::OperationalUnlocked;
        let decision = review_native_client_status_command(operational)
            .expect("operational unlocked status review");
        assert_eq!(
            decision.lock_state,
            NativeClientStatusLockState::OperationalUnlocked
        );
        assert!(!decision.unlock_performed);

        let mut root = safe_draft();
        root.lock_state = NativeClientStatusLockState::RootUnlocked;
        root.capability_state = NativeClientStatusCapabilityState::Revoked;
        let decision =
            review_native_client_status_command(root).expect("root unlocked status review");
        assert_eq!(
            decision.lock_state,
            NativeClientStatusLockState::RootUnlocked
        );
        assert_eq!(
            decision.capability_state,
            NativeClientStatusCapabilityState::Revoked
        );
        assert_eq!(decision.redacted_capability_material, "ABSENT");

        let mut expired = safe_draft();
        expired.capability_state = NativeClientStatusCapabilityState::Expired;
        let decision = review_native_client_status_command(expired).expect("expired capability");
        assert_eq!(
            decision.capability_state,
            NativeClientStatusCapabilityState::Expired
        );
        assert_eq!(decision.redacted_capability_material, "ABSENT");
    }

    #[test]
    fn phase15a_rejects_wrong_command_missing_phase_non_desktop_and_unready_postures() {
        let mut wrong_command = safe_draft();
        wrong_command.command_name = "passport_unlock_root";
        assert_eq!(
            review_native_client_status_command(wrong_command),
            Err(NativeClientStatusCommandReviewError::CommandNameMismatch)
        );

        let mut missing_phase = safe_draft();
        missing_phase.latest_completed_phase_label = "";
        assert_eq!(
            review_native_client_status_command(missing_phase),
            Err(NativeClientStatusCommandReviewError::MissingLatestCompletedPhaseLabel)
        );

        let mut no_feature = safe_draft();
        no_feature.native_feature_enabled = false;
        assert_eq!(
            review_native_client_status_command(no_feature),
            Err(NativeClientStatusCommandReviewError::NativeFeatureNotEnabled)
        );

        let mut non_desktop = safe_draft();
        non_desktop.desktop_surface = false;
        assert_eq!(
            review_native_client_status_command(non_desktop),
            Err(NativeClientStatusCommandReviewError::NotDesktopSurface)
        );

        let mut no_redaction = safe_draft();
        no_redaction.redacted_dto_required = false;
        assert_eq!(
            review_native_client_status_command(no_redaction),
            Err(NativeClientStatusCommandReviewError::RedactedDtoNotRequired)
        );

        let mut route_not_green = safe_draft();
        route_not_green.route_acceptance_green = false;
        assert_eq!(
            review_native_client_status_command(route_not_green),
            Err(NativeClientStatusCommandReviewError::RouteAcceptanceNotGreen)
        );

        let mut local_not_green = safe_draft();
        local_not_green.local_status_inspection_green = false;
        assert_eq!(
            review_native_client_status_command(local_not_green),
            Err(NativeClientStatusCommandReviewError::LocalStatusInspectionNotGreen)
        );
    }

    #[test]
    fn phase15a_rejects_secret_capability_vault_unlock_runtime_storage_wallet_and_policy_flags() {
        let mut recovery = safe_draft();
        recovery.exposes_recovery_words = true;
        assert_eq!(
            review_native_client_status_command(recovery),
            Err(NativeClientStatusCommandReviewError::SecretMaterialExposure)
        );

        let mut root_signing = safe_draft();
        root_signing.exposes_root_signing_material = true;
        assert_eq!(
            review_native_client_status_command(root_signing),
            Err(NativeClientStatusCommandReviewError::SecretMaterialExposure)
        );

        let mut device_signing = safe_draft();
        device_signing.exposes_device_signing_material = true;
        assert_eq!(
            review_native_client_status_command(device_signing),
            Err(NativeClientStatusCommandReviewError::SecretMaterialExposure)
        );

        let mut capability = safe_draft();
        capability.exposes_capability_material = true;
        assert_eq!(
            review_native_client_status_command(capability),
            Err(NativeClientStatusCommandReviewError::CapabilityMaterialExposure)
        );

        let mut vault = safe_draft();
        vault.exposes_vault_material = true;
        assert_eq!(
            review_native_client_status_command(vault),
            Err(NativeClientStatusCommandReviewError::VaultMaterialExposure)
        );

        let mutators: &[fn(&mut NativeClientStatusCommandDraftV1)] = &[
            |draft| draft.requests_unlock = true,
            |draft| draft.requests_root_confirmation = true,
            |draft| draft.requests_platform_sealer_unseal = true,
            |draft| draft.requests_runtime_io = true,
            |draft| draft.requests_storage_mutation = true,
            |draft| draft.requests_wallet_or_ledger_mutation = true,
            |draft| draft.requests_arbitrary_scope_issue = true,
            |draft| draft.requests_policy_disable = true,
        ];

        for mutate in mutators {
            let mut draft = safe_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_client_status_command(draft),
                Err(NativeClientStatusCommandReviewError::UnsafeStatusCommandAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase15a_source_remains_status_command_contract_only_without_tauri_runtime_storage_wallet_ledger_or_secrets(
    ) {
        let source = fs::read_to_string(repo_file("src/native/client_status_command.rs"))
            .expect("client status command source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("PHASE15A_PASSPORT_STATUS_COMMAND"));
        assert!(source.contains("passport_status"));
        assert!(source.contains("review_native_client_status_command"));
        assert!(native_mod.contains("ClientStatusCommandDto"));
        assert!(!source.contains("serde_json"));

        for forbidden_runtime_pattern in [
            "#[tauri::command]",
            "tauri::",
            "Router::",
            ".route(",
            "axum::routing",
            "lookup_projection.write(",
            "index.write(",
            "username_store.write(",
            "storage.write(",
            "persist_username(",
            "finalize_username(",
            "mutate_index_projection(",
            "issue_capability(",
            "refresh_capability(",
            "revoke_capability(",
            "load_secret",
            "load_key",
            "secret_key_bytes",
            "seed_phrase:",
            "private_key:",
            "device_key:",
            "proof_payload:",
            "capability_token:",
            "raw_capability:",
            "vault_secret:",
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
            "raw_pin:",
            "pin_hash:",
            "root_private_key:",
            "device_private_key:",
            "vault_master_key:",
            "ciphertext:",
            "plaintext:",
            "sealed_bytes:",
        ] {
            assert!(
                !source.contains(forbidden_runtime_pattern),
                "Phase 15A client status command source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
