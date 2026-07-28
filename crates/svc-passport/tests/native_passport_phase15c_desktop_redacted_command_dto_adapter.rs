#[cfg(not(feature = "native-passport"))]
#[test]
fn phase15c_desktop_redacted_command_dto_adapter_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native desktop redacted command DTO adapter surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        adapt_native_client_status_to_redacted_command_dto,
        native_client_redacted_command_dto_adapter_posture, review_native_client_command_inventory,
        review_native_client_status_command, NativeClientRedactedCommandDtoAdapterDraftV1,
        NativeClientRedactedCommandDtoAdapterReviewError, NativeClientStatusCapabilityState,
        NativeClientStatusCommandDecisionV1, NativeClientStatusCommandDraftV1,
        NativeClientStatusLockState, NativePassportSurface, NATIVE_PASSPORT_PHASE15C_LABEL,
        PHASE15A_CLIENT_STATUS_COMMAND_DOMAIN, PHASE15A_CLIENT_STATUS_COMMAND_VERSION,
        PHASE15A_PASSPORT_STATUS_COMMAND, PHASE15B_CLIENT_COMMAND_DENYLIST,
        PHASE15B_CLIENT_COMMAND_INVENTORY, PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN,
        PHASE15B_CLIENT_COMMAND_INVENTORY_VERSION, PHASE15B_ENABLED_SURFACES,
        PHASE15C_ENABLED_SURFACES, PHASE15C_FORBIDDEN_REDACTED_DTO_ADAPTER_FLAGS,
        PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_DOMAIN,
        PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_VERSION, PHASE15C_REDACTED_DTO_TARGET,
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

    fn inventory_draft() -> svc_passport::native::NativeClientCommandInventoryDraftV1 {
        svc_passport::native::NativeClientCommandInventoryDraftV1 {
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

    fn reviewed_status() -> NativeClientStatusCommandDecisionV1 {
        review_native_client_status_command(status_draft()).expect("status decision")
    }

    fn reviewed_inventory() -> svc_passport::native::NativeClientCommandInventoryDecisionV1 {
        review_native_client_command_inventory(
            inventory_draft(),
            PHASE15B_CLIENT_COMMAND_INVENTORY,
            PHASE15B_CLIENT_COMMAND_DENYLIST,
        )
        .expect("inventory decision")
    }

    #[test]
    fn phase15c_label_and_posture_are_locked() {
        let posture = native_client_redacted_command_dto_adapter_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE15C_LABEL,
            "NATIVE_PASSPORT_PHASE15C_DESKTOP_REDACTED_COMMAND_DTO_ADAPTER"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE15C_LABEL);
        assert!(posture.redacted_command_dto_adapter_added);
        assert!(posture.status_command_contract_reused);
        assert!(posture.command_inventory_contract_reused);
        assert!(posture.react_dto_target_locked);
        assert!(posture.forbidden_command_rejection_added);
        assert!(posture.unredacted_material_rejection_added);
        assert!(!posture.live_tauri_command_added);
        assert!(!posture.unlock_runtime_added);
        assert!(!posture.platform_sealer_unseal_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.storage_mutation_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);
        assert_eq!(PHASE15C_REDACTED_DTO_TARGET, "crablink-desktop-react");

        for forbidden in [
            "unreviewed_command_to_react",
            "forbidden_command_to_react",
            "secret_material_to_react",
            "capability_material_to_react",
            "vault_material_to_react",
            "live_tauri_command",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE15C_FORBIDDEN_REDACTED_DTO_ADAPTER_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase15c_surfaces_extend_phase15b_without_back_mutating_it() {
        assert_eq!(
            PHASE15C_ENABLED_SURFACES.len(),
            PHASE15B_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE15C_ENABLED_SURFACES[..PHASE15B_ENABLED_SURFACES.len()],
            PHASE15B_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE15C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ClientRedactedCommandDtoAdapter)
        );
        assert_eq!(
            PHASE15B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ClientCommandInventoryDto)
        );
    }

    #[test]
    fn phase15c_adapts_reviewed_status_and_inventory_into_redacted_react_dto() {
        let envelope = adapt_native_client_status_to_redacted_command_dto(
            adapter_draft(),
            &reviewed_status(),
            &reviewed_inventory(),
        )
        .expect("redacted command DTO envelope");

        assert_eq!(
            envelope.contract_domain,
            PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_DOMAIN
        );
        assert_eq!(
            envelope.contract_version,
            PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_VERSION
        );
        assert_eq!(envelope.command_name, "passport_status");
        assert_eq!(envelope.target_label, "crablink-desktop-react");
        assert_eq!(envelope.requester_label, "desktop-tauri:passport-panel");
        assert_eq!(
            envelope.inventory_contract_domain,
            PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN
        );
        assert_eq!(envelope.lock_state, NativeClientStatusLockState::Locked);
        assert_eq!(
            envelope.capability_state,
            NativeClientStatusCapabilityState::PresentRedacted
        );
        assert_eq!(envelope.redacted_passport_identifier, "REDACTED");
        assert_eq!(envelope.redacted_device_identifier, "REDACTED");
        assert_eq!(envelope.redacted_username_handle, "REDACTED");
        assert_eq!(envelope.redacted_capability_material, "REDACTED");
        assert!(envelope.redacted_react_dto);
        assert!(envelope.command_allowlisted);
        assert!(!envelope.forbidden_command);
        assert!(!envelope.live_tauri_command_added);
        assert!(!envelope.unlock_runtime_added);
        assert!(!envelope.platform_sealer_unseal_added);
        assert!(!envelope.runtime_io_added);
        assert!(!envelope.storage_mutation_added);
        assert!(!envelope.wallet_or_ledger_mutation_added);
        assert!(!envelope.secret_material_exposed);
        assert!(!envelope.capability_material_exposed);
        assert!(!envelope.vault_material_exposed);
        assert!(envelope.contract_only);
    }

    #[test]
    fn phase15c_rejects_unreviewed_forbidden_unallowlisted_or_unredacted_adapter_inputs() {
        let status = reviewed_status();
        let inventory = reviewed_inventory();

        let mut draft = adapter_draft();
        draft.status_decision_reviewed = false;
        assert_eq!(
            adapt_native_client_status_to_redacted_command_dto(draft, &status, &inventory),
            Err(NativeClientRedactedCommandDtoAdapterReviewError::StatusDecisionNotReviewed)
        );

        let mut draft = adapter_draft();
        draft.inventory_decision_reviewed = false;
        assert_eq!(
            adapt_native_client_status_to_redacted_command_dto(draft, &status, &inventory),
            Err(NativeClientRedactedCommandDtoAdapterReviewError::InventoryDecisionNotReviewed)
        );

        let mut draft = adapter_draft();
        draft.command_allowlisted = false;
        assert_eq!(
            adapt_native_client_status_to_redacted_command_dto(draft, &status, &inventory),
            Err(NativeClientRedactedCommandDtoAdapterReviewError::CommandNotAllowlisted)
        );

        let mut draft = adapter_draft();
        draft.command_forbidden = true;
        assert_eq!(
            adapt_native_client_status_to_redacted_command_dto(draft, &status, &inventory),
            Err(NativeClientRedactedCommandDtoAdapterReviewError::ForbiddenCommandRequested)
        );

        let mut draft = adapter_draft();
        draft.redacted_react_dto_required = false;
        assert_eq!(
            adapt_native_client_status_to_redacted_command_dto(draft, &status, &inventory),
            Err(NativeClientRedactedCommandDtoAdapterReviewError::RedactedReactDtoNotRequired)
        );

        let mut draft = adapter_draft();
        draft.target_label = "browser-window";
        assert_eq!(
            adapt_native_client_status_to_redacted_command_dto(draft, &status, &inventory),
            Err(NativeClientRedactedCommandDtoAdapterReviewError::TargetMismatch)
        );
    }

    #[test]
    fn phase15c_rejects_unsafe_status_inventory_exposure_and_runtime_flags() {
        let status = reviewed_status();
        let inventory = reviewed_inventory();

        let mut unsafe_status = status.clone();
        unsafe_status.runtime_io_performed = true;
        assert_eq!(
            adapt_native_client_status_to_redacted_command_dto(
                adapter_draft(),
                &unsafe_status,
                &inventory,
            ),
            Err(NativeClientRedactedCommandDtoAdapterReviewError::StatusDecisionUnsafe)
        );

        let mut unsafe_inventory = inventory.clone();
        unsafe_inventory.live_tauri_command_added = true;
        assert_eq!(
            adapt_native_client_status_to_redacted_command_dto(
                adapter_draft(),
                &status,
                &unsafe_inventory,
            ),
            Err(NativeClientRedactedCommandDtoAdapterReviewError::InventoryDecisionUnsafe)
        );

        let mut wrong_inventory = inventory.clone();
        wrong_inventory.contract_domain = "wrong";
        assert_eq!(
            adapt_native_client_status_to_redacted_command_dto(
                adapter_draft(),
                &status,
                &wrong_inventory,
            ),
            Err(
                NativeClientRedactedCommandDtoAdapterReviewError::InventoryDecisionContractMismatch
            )
        );

        let exposure_mutators: &[(
            fn(&mut NativeClientRedactedCommandDtoAdapterDraftV1),
            NativeClientRedactedCommandDtoAdapterReviewError,
        )] = &[
            (
                |draft| draft.exposes_identifier_material = true,
                NativeClientRedactedCommandDtoAdapterReviewError::IdentifierMaterialExposure,
            ),
            (
                |draft| draft.exposes_secret_material = true,
                NativeClientRedactedCommandDtoAdapterReviewError::SecretMaterialExposure,
            ),
            (
                |draft| draft.exposes_capability_material = true,
                NativeClientRedactedCommandDtoAdapterReviewError::CapabilityMaterialExposure,
            ),
            (
                |draft| draft.exposes_vault_material = true,
                NativeClientRedactedCommandDtoAdapterReviewError::VaultMaterialExposure,
            ),
        ];

        for (mutate, expected) in exposure_mutators {
            let mut draft = adapter_draft();
            mutate(&mut draft);
            assert_eq!(
                adapt_native_client_status_to_redacted_command_dto(draft, &status, &inventory),
                Err(expected.clone())
            );
        }

        let unsafe_mutators: &[fn(&mut NativeClientRedactedCommandDtoAdapterDraftV1)] = &[
            |draft| draft.requests_live_tauri_command = true,
            |draft| draft.requests_unlock_runtime = true,
            |draft| draft.requests_platform_sealer_unseal = true,
            |draft| draft.requests_runtime_io = true,
            |draft| draft.requests_storage_mutation = true,
            |draft| draft.requests_wallet_or_ledger_mutation = true,
        ];

        for mutate in unsafe_mutators {
            let mut draft = adapter_draft();
            mutate(&mut draft);
            assert_eq!(
                adapt_native_client_status_to_redacted_command_dto(draft, &status, &inventory),
                Err(NativeClientRedactedCommandDtoAdapterReviewError::UnsafeAdapterAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase15c_source_remains_redacted_dto_adapter_only_without_tauri_runtime_storage_wallet_ledger_or_secrets(
    ) {
        let source = fs::read_to_string(repo_file(
            "src/native/client_redacted_command_dto_adapter.rs",
        ))
        .expect("client redacted command DTO adapter source");
        let inventory_source =
            fs::read_to_string(repo_file("src/native/client_command_inventory.rs"))
                .expect("client command inventory source");
        let status_source = fs::read_to_string(repo_file("src/native/client_status_command.rs"))
            .expect("client status command source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("adapt_native_client_status_to_redacted_command_dto"));
        assert!(source.contains("PHASE15C_REDACTED_DTO_TARGET"));
        assert!(inventory_source.contains("review_native_client_command_inventory"));
        assert!(status_source.contains("review_native_client_status_command"));
        assert!(native_mod.contains("ClientRedactedCommandDtoAdapter"));
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
                "Phase 15C redacted command DTO adapter source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
