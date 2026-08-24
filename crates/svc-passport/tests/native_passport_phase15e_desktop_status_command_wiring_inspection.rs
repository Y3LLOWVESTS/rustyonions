#[cfg(not(feature = "native-passport"))]
#[test]
fn phase15e_desktop_status_command_wiring_inspection_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile desktop status wiring inspection"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_client_status_command_wiring_inspection_posture,
        review_native_client_status_command_wiring_inspection,
        NativeClientCommandSurfaceAcceptanceDecisionV1,
        NativeClientStatusCommandWiringInspectionDraftV1,
        NativeClientStatusCommandWiringInspectionReviewError, NATIVE_PASSPORT_PHASE15E_LABEL,
        PHASE15A_PASSPORT_STATUS_COMMAND, PHASE15C_REDACTED_DTO_TARGET,
        PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_DOMAIN,
        PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_VERSION, PHASE15E_APP_STATE_PATH,
        PHASE15E_COMMAND_HANDLER_REGISTRY_PATH, PHASE15E_COMMAND_MODULE_REGISTRY_PATH,
        PHASE15E_EXISTING_IDENTITY_COMMAND_NAME, PHASE15E_EXISTING_IDENTITY_COMMAND_PATH,
        PHASE15E_FORBIDDEN_WIRING_INSPECTION_FLAGS, PHASE15E_PROPOSED_PASSPORT_COMMAND_PATH,
        PHASE15E_STATUS_COMMAND_WIRING_INSPECTION_DOMAIN,
        PHASE15E_STATUS_COMMAND_WIRING_INSPECTION_VERSION, PHASE15E_TAURI_CARGO_PATH,
        PHASE15E_TAURI_CRATE_ROOT,
    };

    type WiringInspectionMutationCase = (
        fn(&mut NativeClientStatusCommandWiringInspectionDraftV1),
        NativeClientStatusCommandWiringInspectionReviewError,
    );

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn safe_acceptance() -> NativeClientCommandSurfaceAcceptanceDecisionV1 {
        NativeClientCommandSurfaceAcceptanceDecisionV1 {
            contract_domain: PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_DOMAIN,
            contract_version: PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_VERSION,
            requester_label: "desktop-tauri:command-surface",
            surface_label: "crablink-desktop",
            target_label: PHASE15C_REDACTED_DTO_TARGET,
            command_count: 18,
            forbidden_command_count: 6,
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
            live_desktop_command_wiring_added: false,
            lifecycle_runtime_added: false,
            unlock_runtime_added: false,
            proof_capability_username_runtime_added: false,
            arbitrary_scope_issued: false,
            policy_disabled: false,
            platform_sealer_unseal_performed: false,
            runtime_io_performed: false,
            storage_mutated: false,
            wallet_or_ledger_mutated: false,
            secret_material_exposed: false,
            capability_material_exposed: false,
            vault_material_exposed: false,
            command_surface_contract_complete: true,
            full_desktop_runtime_complete: false,
            contract_only: true,
        }
    }

    fn safe_draft() -> NativeClientStatusCommandWiringInspectionDraftV1 {
        NativeClientStatusCommandWiringInspectionDraftV1 {
            contract_domain: PHASE15E_STATUS_COMMAND_WIRING_INSPECTION_DOMAIN,
            contract_version: PHASE15E_STATUS_COMMAND_WIRING_INSPECTION_VERSION,
            requester_label: "desktop-tauri:passport-status-wiring-inspection",
            tauri_crate_root: PHASE15E_TAURI_CRATE_ROOT,
            tauri_cargo_path: PHASE15E_TAURI_CARGO_PATH,
            command_module_registry_path: PHASE15E_COMMAND_MODULE_REGISTRY_PATH,
            command_handler_registry_path: PHASE15E_COMMAND_HANDLER_REGISTRY_PATH,
            app_state_path: PHASE15E_APP_STATE_PATH,
            existing_identity_command_path: PHASE15E_EXISTING_IDENTITY_COMMAND_PATH,
            proposed_passport_command_path: PHASE15E_PROPOSED_PASSPORT_COMMAND_PATH,
            status_command_name: PHASE15A_PASSPORT_STATUS_COMMAND,
            existing_identity_command_name: PHASE15E_EXISTING_IDENTITY_COMMAND_NAME,
            native_feature_enabled: true,
            desktop_surface: true,
            phase15d_acceptance_reviewed: true,
            direct_svc_passport_dependency_present: false,
            crablink_native_core_dependency_present: true,
            dev_passport_label_present: true,
            dev_label_treated_as_native_truth: false,
            live_status_command_present: false,
            status_command_registered: false,
            forbidden_command_target_present: false,
            redacted_dto_target_confirmed: true,
            exposes_secret_material: false,
            exposes_capability_material: false,
            exposes_vault_material: false,
            requests_runtime_change: false,
            requests_storage_mutation: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase15e_posture_and_exact_tauri_anchors_are_locked() {
        let posture = native_client_status_command_wiring_inspection_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE15E_LABEL,
            "NATIVE_PASSPORT_PHASE15E_DESKTOP_STATUS_COMMAND_WIRING_INSPECTION"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE15E_LABEL);
        assert!(posture.exact_tauri_anchors_inspected);
        assert!(posture.status_command_selected_first);
        assert!(posture.phase15d_acceptance_reused);
        assert!(posture.redacted_react_target_reused);
        assert!(posture.dev_identity_compatibility_identified);
        assert!(posture.dependency_pin_required_before_live_wiring);
        assert!(!posture.live_status_command_added);
        assert!(!posture.command_registry_hook_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.storage_mutation_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);

        assert_eq!(PHASE15E_TAURI_CRATE_ROOT, "apps/crablink-tauri/src-tauri");
        assert_eq!(
            PHASE15E_COMMAND_MODULE_REGISTRY_PATH,
            "apps/crablink-tauri/src-tauri/src/commands/mod.rs"
        );
        assert_eq!(
            PHASE15E_COMMAND_HANDLER_REGISTRY_PATH,
            "apps/crablink-tauri/src-tauri/src/lib.rs"
        );
        assert_eq!(
            PHASE15E_PROPOSED_PASSPORT_COMMAND_PATH,
            "apps/crablink-tauri/src-tauri/src/commands/passport.rs"
        );
        assert!(PHASE15E_FORBIDDEN_WIRING_INSPECTION_FLAGS.contains(&"ledger_mutation"));
    }

    #[test]
    fn phase15e_accepts_current_tauri_posture_and_prepares_phase15f() {
        let decision =
            review_native_client_status_command_wiring_inspection(safe_draft(), &safe_acceptance())
                .expect("Phase 15E wiring inspection");

        assert_eq!(
            decision.status_command_name,
            PHASE15A_PASSPORT_STATUS_COMMAND
        );
        assert_eq!(decision.response_target, PHASE15C_REDACTED_DTO_TARGET);
        assert_eq!(
            decision.proposed_passport_command_path,
            PHASE15E_PROPOSED_PASSPORT_COMMAND_PATH
        );
        assert!(!decision.direct_svc_passport_dependency_present);
        assert!(decision.crablink_native_core_dependency_present);
        assert!(decision.svc_passport_dependency_pin_required_before_live_wiring);
        assert!(decision.existing_dev_identity_is_compatibility_only);
        assert!(!decision.live_status_command_present);
        assert!(!decision.status_command_registered);
        assert!(decision.ready_for_phase15f_status_wiring);
        assert!(!decision.runtime_io_performed);
        assert!(!decision.storage_mutated);
        assert!(!decision.wallet_or_ledger_mutated);
        assert!(!decision.material_exposed);
        assert!(decision.contract_only);
    }

    #[test]
    fn phase15e_rejects_anchor_dependency_dev_live_and_exposure_drift() {
        let acceptance = safe_acceptance();
        let cases: &[WiringInspectionMutationCase] = &[
            (
                |draft| draft.command_handler_registry_path = "src/main.rs",
                NativeClientStatusCommandWiringInspectionReviewError::TauriAnchorMismatch,
            ),
            (
                |draft| draft.direct_svc_passport_dependency_present = true,
                NativeClientStatusCommandWiringInspectionReviewError::DirectSvcPassportDependencyUnexpected,
            ),
            (
                |draft| draft.crablink_native_core_dependency_present = false,
                NativeClientStatusCommandWiringInspectionReviewError::CrablinkNativeCoreDependencyNotObserved,
            ),
            (
                |draft| draft.dev_label_treated_as_native_truth = true,
                NativeClientStatusCommandWiringInspectionReviewError::DevPassportLabelPromotedToNativeTruth,
            ),
            (
                |draft| draft.live_status_command_present = true,
                NativeClientStatusCommandWiringInspectionReviewError::LiveStatusCommandUnexpected,
            ),
            (
                |draft| draft.exposes_secret_material = true,
                NativeClientStatusCommandWiringInspectionReviewError::MaterialExposureObserved,
            ),
            (
                |draft| draft.requests_runtime_change = true,
                NativeClientStatusCommandWiringInspectionReviewError::UnsafeInspectionAuthorityFlag,
            ),
        ];

        for (mutate, expected) in cases {
            let mut draft = safe_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_client_status_command_wiring_inspection(draft, &acceptance),
                Err(expected.clone())
            );
        }
    }

    #[test]
    fn phase15e_rejects_phase15d_live_or_full_runtime_claims() {
        let mut acceptance = safe_acceptance();
        acceptance.live_desktop_command_wiring_added = true;
        assert_eq!(
            review_native_client_status_command_wiring_inspection(safe_draft(), &acceptance),
            Err(NativeClientStatusCommandWiringInspectionReviewError::Phase15DAcceptanceUnsafe)
        );

        let mut acceptance = safe_acceptance();
        acceptance.full_desktop_runtime_complete = true;
        assert_eq!(
            review_native_client_status_command_wiring_inspection(safe_draft(), &acceptance),
            Err(NativeClientStatusCommandWiringInspectionReviewError::Phase15DAcceptanceUnsafe)
        );
    }

    #[test]
    fn phase15e_source_remains_inspection_only() {
        let source = fs::read_to_string(repo_file(
            "src/native/client_status_command_wiring_inspection.rs",
        ))
        .expect("Phase 15E source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("review_native_client_status_command_wiring_inspection"));
        assert!(source.contains(PHASE15E_COMMAND_HANDLER_REGISTRY_PATH));
        assert!(source.contains(PHASE15E_PROPOSED_PASSPORT_COMMAND_PATH));
        assert!(native_mod.contains("client_status_command_wiring_inspection"));

        for forbidden_runtime_pattern in [
            "tauri::",
            "Router::",
            ".route(",
            "axum::routing",
            "reqwest::",
            "tokio::fs",
            "std::fs::write",
            "issue_capability(",
            "load_secret",
            "load_key",
            "generate_device_key(",
            "derive_root_key(",
            "derive_device_key(",
            "validate_pin(",
            "unlock_vault(",
            "platform_unseal(",
            "encrypt(",
            "decrypt(",
            "runtime_read(",
            "runtime_write(",
            "wallet.spend(",
            "ledger.write(",
            "seed_phrase:",
            "private_key:",
            "device_key:",
            "capability_token:",
            "raw_capability:",
            "vault_secret:",
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
                "Phase 15E inspection source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
