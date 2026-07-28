#[cfg(not(feature = "native-passport"))]
#[test]
fn phase15b_desktop_command_inventory_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native desktop command inventory surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_client_command_inventory_command_names,
        native_client_command_inventory_forbidden_command_names,
        native_client_command_inventory_posture,
        native_client_command_inventory_status_command_name,
        review_native_client_command_inventory, NativeClientCommandCategory,
        NativeClientCommandInventoryDraftV1, NativeClientCommandInventoryReviewError,
        NativeClientCommandRisk, NativePassportSurface, NATIVE_PASSPORT_PHASE15B_LABEL,
        PHASE15A_ENABLED_SURFACES, PHASE15B_CLIENT_COMMAND_DENYLIST,
        PHASE15B_CLIENT_COMMAND_INVENTORY, PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN,
        PHASE15B_CLIENT_COMMAND_INVENTORY_VERSION, PHASE15B_ENABLED_SURFACES,
        PHASE15B_EXPECTED_COMMAND_COUNT, PHASE15B_EXPECTED_FORBIDDEN_COMMAND_COUNT,
        PHASE15B_FORBIDDEN_CLIENT_COMMAND_AUTHORITY_FLAGS,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn safe_draft() -> NativeClientCommandInventoryDraftV1 {
        NativeClientCommandInventoryDraftV1 {
            contract_domain: PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN,
            contract_version: PHASE15B_CLIENT_COMMAND_INVENTORY_VERSION,
            requester_label: "desktop-tauri:command-registry",
            surface_label: "crablink-desktop",
            native_feature_enabled: true,
            desktop_surface: true,
            status_command_contract_green: true,
            command_inventory_count: PHASE15B_EXPECTED_COMMAND_COUNT,
            forbidden_command_count: PHASE15B_EXPECTED_FORBIDDEN_COMMAND_COUNT,
            expected_inventory_included: true,
            forbidden_inventory_included: true,
            redacted_react_dtos_required: true,
            secrets_to_react_allowed: false,
            live_tauri_wiring_requested: false,
            lifecycle_runtime_requested: false,
            unlock_runtime_requested: false,
            proof_capability_username_runtime_requested: false,
            arbitrary_scope_issue_requested: false,
            policy_disable_requested: false,
            platform_sealer_unseal_requested: false,
            runtime_io_requested: false,
            storage_mutation_requested: false,
            wallet_or_ledger_mutation_requested: false,
        }
    }

    #[test]
    fn phase15b_label_posture_inventory_counts_and_command_lists_are_locked() {
        let posture = native_client_command_inventory_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE15B_LABEL,
            "NATIVE_PASSPORT_PHASE15B_DESKTOP_COMMAND_INVENTORY_AND_DENYLIST"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE15B_LABEL);
        assert!(posture.command_inventory_contract_added);
        assert!(posture.expected_command_inventory_locked);
        assert!(posture.forbidden_command_inventory_locked);
        assert!(posture.redacted_react_dto_policy_added);
        assert!(posture.status_command_contract_reused);
        assert!(!posture.live_tauri_command_added);
        assert!(!posture.lifecycle_runtime_added);
        assert!(!posture.unlock_runtime_added);
        assert!(!posture.proof_capability_username_runtime_added);
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

        assert_eq!(PHASE15B_EXPECTED_COMMAND_COUNT, 18);
        assert_eq!(PHASE15B_EXPECTED_FORBIDDEN_COMMAND_COUNT, 6);
        assert_eq!(PHASE15B_CLIENT_COMMAND_INVENTORY.len(), 18);
        assert_eq!(PHASE15B_CLIENT_COMMAND_DENYLIST.len(), 6);
        assert_eq!(
            native_client_command_inventory_status_command_name(),
            "passport_status"
        );
        assert!(native_client_command_inventory_command_names().contains(&"passport_prove"));
        assert!(
            native_client_command_inventory_command_names().contains(&"passport_username_release")
        );
        assert!(native_client_command_inventory_forbidden_command_names()
            .contains(&"passport_get_seed_to_webview"));
        assert!(native_client_command_inventory_forbidden_command_names()
            .contains(&"passport_disable_policy"));

        for forbidden in [
            "seed_to_webview",
            "private_key_export",
            "device_key_export",
            "raw_capability_export",
            "arbitrary_scope_issue",
            "policy_disable",
            "live_tauri_command",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE15B_FORBIDDEN_CLIENT_COMMAND_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase15b_surfaces_extend_phase15a_without_back_mutating_it() {
        assert_eq!(
            PHASE15B_ENABLED_SURFACES.len(),
            PHASE15A_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE15B_ENABLED_SURFACES[..PHASE15A_ENABLED_SURFACES.len()],
            PHASE15A_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE15B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ClientCommandInventoryDto)
        );
        assert_eq!(
            PHASE15A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ClientStatusCommandDto)
        );
    }

    #[test]
    fn phase15b_reviews_exact_desktop_command_inventory_and_denylist_without_live_wiring() {
        let decision = review_native_client_command_inventory(
            safe_draft(),
            PHASE15B_CLIENT_COMMAND_INVENTORY,
            PHASE15B_CLIENT_COMMAND_DENYLIST,
        )
        .expect("inventory should review");

        assert_eq!(
            decision.contract_domain,
            PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN
        );
        assert_eq!(decision.command_count, 18);
        assert_eq!(decision.forbidden_command_count, 6);
        assert!(decision.status_command_contract_green);
        assert!(decision.expected_inventory_locked);
        assert!(decision.forbidden_inventory_locked);
        assert!(decision.redacted_react_dtos_required);
        assert!(decision.forbidden_commands_absent_from_allowlist);
        assert!(!decision.live_tauri_command_added);
        assert!(!decision.lifecycle_runtime_added);
        assert!(!decision.unlock_runtime_added);
        assert!(!decision.proof_capability_username_runtime_added);
        assert!(!decision.arbitrary_scope_issued);
        assert!(!decision.policy_disabled);
        assert!(!decision.platform_sealer_unseal_performed);
        assert!(!decision.runtime_io_performed);
        assert!(!decision.storage_mutated);
        assert!(!decision.wallet_or_ledger_mutated);
        assert!(!decision.secret_material_exposed);
        assert!(decision.contract_only);

        let status = PHASE15B_CLIENT_COMMAND_INVENTORY
            .iter()
            .find(|entry| entry.command_name == "passport_status")
            .expect("status command entry");
        assert_eq!(status.category, NativeClientCommandCategory::Status);
        assert_eq!(status.risk, NativeClientCommandRisk::ReadOnlyRedacted);
        assert!(status.redacted_response_required);
        assert!(!status.secret_response_allowed);

        let root_unlock = PHASE15B_CLIENT_COMMAND_INVENTORY
            .iter()
            .find(|entry| entry.command_name == "passport_unlock_root")
            .expect("root unlock command entry");
        assert_eq!(root_unlock.category, NativeClientCommandCategory::Unlock);
        assert_eq!(
            root_unlock.risk,
            NativeClientCommandRisk::RequiresRootConfirmation
        );
        assert!(!root_unlock.live_tauri_wiring_allowed_now);
        assert!(!root_unlock.mutating_behavior_allowed_now);
    }

    #[test]
    fn phase15b_rejects_missing_duplicate_unknown_malformed_or_forbidden_commands() {
        assert_eq!(
            review_native_client_command_inventory(
                safe_draft(),
                &PHASE15B_CLIENT_COMMAND_INVENTORY[..17],
                PHASE15B_CLIENT_COMMAND_DENYLIST,
            ),
            Err(NativeClientCommandInventoryReviewError::CommandCountMismatch)
        );

        let mut duplicate = PHASE15B_CLIENT_COMMAND_INVENTORY.to_vec();
        duplicate[1] = duplicate[0];
        assert_eq!(
            review_native_client_command_inventory(
                safe_draft(),
                &duplicate,
                PHASE15B_CLIENT_COMMAND_DENYLIST,
            ),
            Err(NativeClientCommandInventoryReviewError::DuplicateCommand)
        );

        let mut unknown = PHASE15B_CLIENT_COMMAND_INVENTORY.to_vec();
        unknown[0].command_name = "passport_not_allowed";
        assert_eq!(
            review_native_client_command_inventory(
                safe_draft(),
                &unknown,
                PHASE15B_CLIENT_COMMAND_DENYLIST,
            ),
            Err(NativeClientCommandInventoryReviewError::UnknownCommand)
        );

        let mut malformed = PHASE15B_CLIENT_COMMAND_INVENTORY.to_vec();
        malformed[0].secret_response_allowed = true;
        assert_eq!(
            review_native_client_command_inventory(
                safe_draft(),
                &malformed,
                PHASE15B_CLIENT_COMMAND_DENYLIST,
            ),
            Err(NativeClientCommandInventoryReviewError::CommandShapeMismatch)
        );

        let mut forbidden_allowed = PHASE15B_CLIENT_COMMAND_INVENTORY.to_vec();
        forbidden_allowed[0].command_name = "passport_get_seed_to_webview";
        assert_eq!(
            review_native_client_command_inventory(
                safe_draft(),
                &forbidden_allowed,
                PHASE15B_CLIENT_COMMAND_DENYLIST,
            ),
            Err(NativeClientCommandInventoryReviewError::UnknownCommand)
        );

        let mut bad_denylist = PHASE15B_CLIENT_COMMAND_DENYLIST.to_vec();
        bad_denylist.pop();
        assert_eq!(
            review_native_client_command_inventory(
                safe_draft(),
                PHASE15B_CLIENT_COMMAND_INVENTORY,
                &bad_denylist,
            ),
            Err(NativeClientCommandInventoryReviewError::ForbiddenCommandCountMismatch)
        );

        let mut bad_denylist = PHASE15B_CLIENT_COMMAND_DENYLIST.to_vec();
        bad_denylist[0].reason = "";
        assert_eq!(
            review_native_client_command_inventory(
                safe_draft(),
                PHASE15B_CLIENT_COMMAND_INVENTORY,
                &bad_denylist,
            ),
            Err(NativeClientCommandInventoryReviewError::ForbiddenCommandShapeMismatch)
        );
    }

    #[test]
    fn phase15b_rejects_unready_status_non_desktop_unredacted_secret_live_wiring_and_policy_bypass()
    {
        let mut no_status = safe_draft();
        no_status.status_command_contract_green = false;
        assert_eq!(
            review_native_client_command_inventory(
                no_status,
                PHASE15B_CLIENT_COMMAND_INVENTORY,
                PHASE15B_CLIENT_COMMAND_DENYLIST,
            ),
            Err(NativeClientCommandInventoryReviewError::StatusCommandContractNotGreen)
        );

        let mut non_desktop = safe_draft();
        non_desktop.desktop_surface = false;
        assert_eq!(
            review_native_client_command_inventory(
                non_desktop,
                PHASE15B_CLIENT_COMMAND_INVENTORY,
                PHASE15B_CLIENT_COMMAND_DENYLIST,
            ),
            Err(NativeClientCommandInventoryReviewError::NotDesktopSurface)
        );

        let mut no_redaction = safe_draft();
        no_redaction.redacted_react_dtos_required = false;
        assert_eq!(
            review_native_client_command_inventory(
                no_redaction,
                PHASE15B_CLIENT_COMMAND_INVENTORY,
                PHASE15B_CLIENT_COMMAND_DENYLIST,
            ),
            Err(NativeClientCommandInventoryReviewError::RedactedReactDtosNotRequired)
        );

        let mut secrets_to_react = safe_draft();
        secrets_to_react.secrets_to_react_allowed = true;
        assert_eq!(
            review_native_client_command_inventory(
                secrets_to_react,
                PHASE15B_CLIENT_COMMAND_INVENTORY,
                PHASE15B_CLIENT_COMMAND_DENYLIST,
            ),
            Err(NativeClientCommandInventoryReviewError::SecretsToReactAllowed)
        );

        let mut live_wiring = safe_draft();
        live_wiring.live_tauri_wiring_requested = true;
        assert_eq!(
            review_native_client_command_inventory(
                live_wiring,
                PHASE15B_CLIENT_COMMAND_INVENTORY,
                PHASE15B_CLIENT_COMMAND_DENYLIST,
            ),
            Err(NativeClientCommandInventoryReviewError::LiveTauriWiringRequested)
        );

        let mutators: &[fn(&mut NativeClientCommandInventoryDraftV1)] = &[
            |draft| draft.lifecycle_runtime_requested = true,
            |draft| draft.unlock_runtime_requested = true,
            |draft| draft.proof_capability_username_runtime_requested = true,
            |draft| draft.arbitrary_scope_issue_requested = true,
            |draft| draft.policy_disable_requested = true,
            |draft| draft.platform_sealer_unseal_requested = true,
            |draft| draft.runtime_io_requested = true,
            |draft| draft.storage_mutation_requested = true,
            |draft| draft.wallet_or_ledger_mutation_requested = true,
        ];

        for mutate in mutators {
            let mut draft = safe_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_client_command_inventory(
                    draft,
                    PHASE15B_CLIENT_COMMAND_INVENTORY,
                    PHASE15B_CLIENT_COMMAND_DENYLIST,
                ),
                Err(NativeClientCommandInventoryReviewError::UnsafeClientCommandAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase15b_source_remains_inventory_contract_only_without_tauri_runtime_storage_wallet_ledger_or_secrets(
    ) {
        let source = fs::read_to_string(repo_file("src/native/client_command_inventory.rs"))
            .expect("client command inventory source");
        let status_source = fs::read_to_string(repo_file("src/native/client_status_command.rs"))
            .expect("client status command source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("PHASE15B_CLIENT_COMMAND_INVENTORY"));
        assert!(source.contains("PHASE15B_CLIENT_COMMAND_DENYLIST"));
        assert!(source.contains("review_native_client_command_inventory"));
        assert!(status_source.contains("review_native_client_status_command"));
        assert!(native_mod.contains("ClientCommandInventoryDto"));
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
                "Phase 15B command inventory source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
