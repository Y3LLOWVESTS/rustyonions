#[cfg(not(feature = "native-passport"))]
#[test]
fn phase15d_desktop_command_surface_acceptance_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native desktop command surface acceptance"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        adapt_native_client_status_to_redacted_command_dto,
        native_client_command_surface_acceptance_posture, review_native_client_command_inventory,
        review_native_client_command_surface_acceptance, review_native_client_status_command,
        NativeClientCommandInventoryDecisionV1, NativeClientCommandInventoryDraftV1,
        NativeClientCommandSurfaceAcceptanceDraftV1,
        NativeClientCommandSurfaceAcceptanceReviewError,
        NativeClientRedactedCommandDtoAdapterDraftV1, NativeClientRedactedCommandDtoEnvelopeV1,
        NativeClientStatusCapabilityState, NativeClientStatusCommandDecisionV1,
        NativeClientStatusCommandDraftV1, NativeClientStatusLockState, NativePassportSurface,
        NATIVE_PASSPORT_PHASE15D_LABEL, PHASE15A_CLIENT_STATUS_COMMAND_DOMAIN,
        PHASE15A_CLIENT_STATUS_COMMAND_VERSION, PHASE15A_PASSPORT_STATUS_COMMAND,
        PHASE15B_CLIENT_COMMAND_DENYLIST, PHASE15B_CLIENT_COMMAND_INVENTORY,
        PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN, PHASE15B_CLIENT_COMMAND_INVENTORY_VERSION,
        PHASE15C_ENABLED_SURFACES, PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_DOMAIN,
        PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_VERSION, PHASE15C_REDACTED_DTO_TARGET,
        PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_DOMAIN,
        PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_VERSION, PHASE15D_ENABLED_SURFACES,
        PHASE15D_FORBIDDEN_COMMAND_SURFACE_FLAGS,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn status_draft() -> NativeClientStatusCommandDraftV1 {
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

    fn inventory_draft() -> NativeClientCommandInventoryDraftV1 {
        NativeClientCommandInventoryDraftV1 {
            contract_domain: PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN,
            contract_version: PHASE15B_CLIENT_COMMAND_INVENTORY_VERSION,
            requester_label: "desktop-tauri:command-registry",
            surface_label: "crablink-desktop",
            native_feature_enabled: true,
            desktop_surface: true,
            status_command_contract_green: true,
            command_inventory_count: 18,
            forbidden_command_count: 6,
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

    fn adapter_draft() -> NativeClientRedactedCommandDtoAdapterDraftV1 {
        NativeClientRedactedCommandDtoAdapterDraftV1 {
            contract_domain: PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_DOMAIN,
            contract_version: PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_VERSION,
            requester_label: "desktop-tauri:passport-panel",
            target_label: PHASE15C_REDACTED_DTO_TARGET,
            command_name: PHASE15A_PASSPORT_STATUS_COMMAND,
            status_decision_reviewed: true,
            inventory_decision_reviewed: true,
            command_allowlisted: true,
            command_forbidden: false,
            redacted_react_dto_required: true,
            exposes_identifier_material: false,
            exposes_secret_material: false,
            exposes_capability_material: false,
            exposes_vault_material: false,
            requests_live_tauri_command: false,
            requests_unlock_runtime: false,
            requests_platform_sealer_unseal: false,
            requests_runtime_io: false,
            requests_storage_mutation: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    fn acceptance_draft() -> NativeClientCommandSurfaceAcceptanceDraftV1 {
        NativeClientCommandSurfaceAcceptanceDraftV1 {
            contract_domain: PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_DOMAIN,
            contract_version: PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_VERSION,
            requester_label: "desktop-tauri:command-surface",
            surface_label: "crablink-desktop",
            target_label: PHASE15C_REDACTED_DTO_TARGET,
            expected_command_count: 18,
            expected_forbidden_command_count: 6,
            status_command_reviewed: true,
            command_inventory_reviewed: true,
            redacted_dto_adapter_reviewed: true,
            all_desktop_commands_allowlisted: true,
            forbidden_commands_denied: true,
            react_receives_only_redacted_dtos: true,
            status_command_available: true,
            lifecycle_command_contracts_available: true,
            unlock_command_contracts_available: true,
            device_command_contracts_available: true,
            proof_command_contracts_available: true,
            capability_command_contracts_available: true,
            username_command_contracts_available: true,
            live_desktop_command_wiring_requested: false,
            lifecycle_runtime_requested: false,
            unlock_runtime_requested: false,
            proof_capability_username_runtime_requested: false,
            arbitrary_scope_issue_requested: false,
            policy_disable_requested: false,
            platform_sealer_unseal_requested: false,
            runtime_io_requested: false,
            storage_mutation_requested: false,
            wallet_or_ledger_mutation_requested: false,
            secret_material_exposure_requested: false,
            capability_material_exposure_requested: false,
            vault_material_exposure_requested: false,
        }
    }

    fn reviewed_status() -> NativeClientStatusCommandDecisionV1 {
        review_native_client_status_command(status_draft()).expect("status decision")
    }

    fn reviewed_inventory() -> NativeClientCommandInventoryDecisionV1 {
        review_native_client_command_inventory(
            inventory_draft(),
            PHASE15B_CLIENT_COMMAND_INVENTORY,
            PHASE15B_CLIENT_COMMAND_DENYLIST,
        )
        .expect("inventory decision")
    }

    fn reviewed_dto() -> NativeClientRedactedCommandDtoEnvelopeV1 {
        adapt_native_client_status_to_redacted_command_dto(
            adapter_draft(),
            &reviewed_status(),
            &reviewed_inventory(),
        )
        .expect("redacted DTO")
    }

    #[test]
    fn phase15d_label_and_posture_are_locked() {
        let posture = native_client_command_surface_acceptance_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE15D_LABEL,
            "NATIVE_PASSPORT_PHASE15D_DESKTOP_COMMAND_SURFACE_ACCEPTANCE"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE15D_LABEL);
        assert!(posture.command_surface_acceptance_added);
        assert!(posture.status_command_contract_reused);
        assert!(posture.command_inventory_contract_reused);
        assert!(posture.redacted_dto_adapter_reused);
        assert!(posture.command_group_acceptance_added);
        assert!(posture.redacted_react_dto_acceptance_added);
        assert!(posture.forbidden_command_acceptance_added);
        assert!(!posture.live_desktop_command_wiring_added);
        assert!(!posture.lifecycle_runtime_added);
        assert!(!posture.unlock_runtime_added);
        assert!(!posture.proof_capability_username_runtime_added);
        assert!(!posture.platform_sealer_unseal_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.storage_mutation_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);
        assert!(!posture.full_desktop_runtime_complete);

        for forbidden in [
            "unreviewed_desktop_command",
            "unredacted_react_dto",
            "forbidden_desktop_command",
            "secret_material_to_react",
            "capability_material_to_react",
            "vault_material_to_react",
            "live_tauri_command",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE15D_FORBIDDEN_COMMAND_SURFACE_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase15d_surfaces_extend_phase15c_without_back_mutating_it() {
        assert_eq!(
            PHASE15D_ENABLED_SURFACES.len(),
            PHASE15C_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE15D_ENABLED_SURFACES[..PHASE15C_ENABLED_SURFACES.len()],
            PHASE15C_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE15D_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ClientCommandSurfaceAcceptanceDto)
        );
        assert_eq!(
            PHASE15C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ClientRedactedCommandDtoAdapter)
        );
    }

    #[test]
    fn phase15d_accepts_full_desktop_command_surface_contract_chain_without_runtime_claim() {
        let decision = review_native_client_command_surface_acceptance(
            acceptance_draft(),
            &reviewed_status(),
            &reviewed_inventory(),
            &reviewed_dto(),
        )
        .expect("command surface acceptance");

        assert_eq!(
            decision.contract_domain,
            PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_DOMAIN
        );
        assert_eq!(decision.command_count, 18);
        assert_eq!(decision.forbidden_command_count, 6);
        assert!(decision.status_command_reviewed);
        assert!(decision.command_inventory_reviewed);
        assert!(decision.redacted_dto_adapter_reviewed);
        assert!(decision.all_desktop_commands_allowlisted);
        assert!(decision.forbidden_commands_denied);
        assert!(decision.react_receives_only_redacted_dtos);
        assert!(decision.status_command_available);
        assert!(decision.lifecycle_command_contracts_available);
        assert!(decision.unlock_command_contracts_available);
        assert!(decision.device_command_contracts_available);
        assert!(decision.proof_command_contracts_available);
        assert!(decision.capability_command_contracts_available);
        assert!(decision.username_command_contracts_available);
        assert!(!decision.live_desktop_command_wiring_added);
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
        assert!(!decision.capability_material_exposed);
        assert!(!decision.vault_material_exposed);
        assert!(decision.command_surface_contract_complete);
        assert!(!decision.full_desktop_runtime_complete);
        assert!(decision.contract_only);
    }

    #[test]
    fn phase15d_rejects_unreviewed_unredacted_missing_or_live_wiring_drafts() {
        let status = reviewed_status();
        let inventory = reviewed_inventory();
        let dto = reviewed_dto();

        let mut draft = acceptance_draft();
        draft.status_command_reviewed = false;
        assert_eq!(
            review_native_client_command_surface_acceptance(draft, &status, &inventory, &dto),
            Err(NativeClientCommandSurfaceAcceptanceReviewError::StatusCommandNotReviewed)
        );

        let mut draft = acceptance_draft();
        draft.command_inventory_reviewed = false;
        assert_eq!(
            review_native_client_command_surface_acceptance(draft, &status, &inventory, &dto),
            Err(NativeClientCommandSurfaceAcceptanceReviewError::CommandInventoryNotReviewed)
        );

        let mut draft = acceptance_draft();
        draft.redacted_dto_adapter_reviewed = false;
        assert_eq!(
            review_native_client_command_surface_acceptance(draft, &status, &inventory, &dto),
            Err(NativeClientCommandSurfaceAcceptanceReviewError::RedactedDtoAdapterNotReviewed)
        );

        let mut draft = acceptance_draft();
        draft.react_receives_only_redacted_dtos = false;
        assert_eq!(
            review_native_client_command_surface_acceptance(draft, &status, &inventory, &dto),
            Err(NativeClientCommandSurfaceAcceptanceReviewError::ReactDtoNotRedacted)
        );

        let mut draft = acceptance_draft();
        draft.username_command_contracts_available = false;
        assert_eq!(
            review_native_client_command_surface_acceptance(draft, &status, &inventory, &dto),
            Err(NativeClientCommandSurfaceAcceptanceReviewError::CommandContractGroupMissing)
        );

        let mut draft = acceptance_draft();
        draft.live_desktop_command_wiring_requested = true;
        assert_eq!(
            review_native_client_command_surface_acceptance(draft, &status, &inventory, &dto),
            Err(NativeClientCommandSurfaceAcceptanceReviewError::LiveDesktopWiringRequested)
        );
    }

    #[test]
    fn phase15d_rejects_unsafe_status_inventory_dto_exposure_and_authority_flags() {
        let status = reviewed_status();
        let inventory = reviewed_inventory();
        let dto = reviewed_dto();

        let mut unsafe_status = status.clone();
        unsafe_status.storage_mutated = true;
        assert_eq!(
            review_native_client_command_surface_acceptance(
                acceptance_draft(),
                &unsafe_status,
                &inventory,
                &dto,
            ),
            Err(NativeClientCommandSurfaceAcceptanceReviewError::StatusDecisionUnsafe)
        );

        let mut unsafe_inventory = inventory.clone();
        unsafe_inventory.unlock_runtime_added = true;
        assert_eq!(
            review_native_client_command_surface_acceptance(
                acceptance_draft(),
                &status,
                &unsafe_inventory,
                &dto,
            ),
            Err(NativeClientCommandSurfaceAcceptanceReviewError::InventoryDecisionUnsafe)
        );

        let mut unsafe_dto = dto.clone();
        unsafe_dto.secret_material_exposed = true;
        assert_eq!(
            review_native_client_command_surface_acceptance(
                acceptance_draft(),
                &status,
                &inventory,
                &unsafe_dto,
            ),
            Err(NativeClientCommandSurfaceAcceptanceReviewError::RedactedDtoEnvelopeUnsafe)
        );

        let mut wrong_target = acceptance_draft();
        wrong_target.target_label = "browser-window";
        assert_eq!(
            review_native_client_command_surface_acceptance(
                wrong_target,
                &status,
                &inventory,
                &dto,
            ),
            Err(NativeClientCommandSurfaceAcceptanceReviewError::TargetMismatch)
        );

        let exposure_mutators: &[fn(&mut NativeClientCommandSurfaceAcceptanceDraftV1)] = &[
            |draft| draft.secret_material_exposure_requested = true,
            |draft| draft.capability_material_exposure_requested = true,
            |draft| draft.vault_material_exposure_requested = true,
        ];

        for mutate in exposure_mutators {
            let mut draft = acceptance_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_client_command_surface_acceptance(draft, &status, &inventory, &dto),
                Err(NativeClientCommandSurfaceAcceptanceReviewError::ExposureRequested)
            );
        }

        let unsafe_mutators: &[fn(&mut NativeClientCommandSurfaceAcceptanceDraftV1)] = &[
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

        for mutate in unsafe_mutators {
            let mut draft = acceptance_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_client_command_surface_acceptance(draft, &status, &inventory, &dto),
                Err(
                    NativeClientCommandSurfaceAcceptanceReviewError::UnsafeDesktopCommandSurfaceAuthorityFlag
                )
            );
        }
    }

    #[test]
    fn phase15d_source_remains_command_surface_acceptance_only_without_tauri_runtime_storage_wallet_ledger_or_secrets(
    ) {
        let source =
            fs::read_to_string(repo_file("src/native/client_command_surface_acceptance.rs"))
                .expect("client command surface acceptance source");
        let adapter_source = fs::read_to_string(repo_file(
            "src/native/client_redacted_command_dto_adapter.rs",
        ))
        .expect("client redacted DTO adapter source");
        let inventory_source =
            fs::read_to_string(repo_file("src/native/client_command_inventory.rs"))
                .expect("client command inventory source");
        let status_source = fs::read_to_string(repo_file("src/native/client_status_command.rs"))
            .expect("client status command source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("review_native_client_command_surface_acceptance"));
        assert!(adapter_source.contains("adapt_native_client_status_to_redacted_command_dto"));
        assert!(inventory_source.contains("review_native_client_command_inventory"));
        assert!(status_source.contains("review_native_client_status_command"));
        assert!(native_mod.contains("ClientCommandSurfaceAcceptanceDto"));
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
                "Phase 15D command surface acceptance source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
