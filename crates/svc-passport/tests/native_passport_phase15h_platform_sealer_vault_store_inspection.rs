#[cfg(not(feature = "native-passport"))]
#[test]
fn phase15h_platform_storage_inspection_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native desktop platform-storage inspection"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_desktop_platform_storage_inspection_posture,
        review_native_desktop_platform_storage_inspection,
        NativeDesktopPlatformStorageInspectionDraftV1,
        NativeDesktopPlatformStorageInspectionReviewError, NativePlatformFamily,
        NATIVE_PASSPORT_PHASE15H_LABEL, PHASE15H_DESKTOP_PLATFORM_STORAGE_INSPECTION_DOMAIN,
        PHASE15H_DESKTOP_PLATFORM_STORAGE_INSPECTION_VERSION,
        PHASE15H_FORBIDDEN_PLATFORM_STORAGE_FLAGS, PHASE15H_FRONTEND_OWNER,
        PHASE15H_PLATFORM_ADAPTER_OWNER, PHASE15H_PLATFORM_NEUTRAL_OWNER,
        PHASE15H_REQUIRED_ATOMIC_WRITE_STEPS, PHASE15H_REQUIRED_DESKTOP_PLATFORM_FAMILIES,
        PHASE15H_TAURI_CARGO_PATH, PHASE15H_TAURI_PASSPORT_COMMAND_PATH, PHASE15H_TAURI_STATE_PATH,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn valid_draft() -> NativeDesktopPlatformStorageInspectionDraftV1 {
        NativeDesktopPlatformStorageInspectionDraftV1 {
            contract_domain: PHASE15H_DESKTOP_PLATFORM_STORAGE_INSPECTION_DOMAIN,
            contract_version: PHASE15H_DESKTOP_PLATFORM_STORAGE_INSPECTION_VERSION,
            requester_label: "crablink-desktop:platform-storage-inspection",

            platform_neutral_owner: PHASE15H_PLATFORM_NEUTRAL_OWNER,
            platform_adapter_owner: PHASE15H_PLATFORM_ADAPTER_OWNER,
            frontend_owner: PHASE15H_FRONTEND_OWNER,

            phase5a_sealer_contract_reviewed: true,
            phase6a_vault_header_reviewed: true,

            direct_svc_passport_dependency_present: true,
            native_passport_feature_enabled: true,
            dev_kms_disabled: true,
            passport_status_registered: true,

            platform_sealer_backend_dependency_present: false,
            platform_sealer_adapter_present: false,
            vault_store_adapter_present: false,
            passport_runtime_state_owner_present: false,

            frontend_redacted_only: true,
            frontend_secret_custody_present: false,
            plaintext_temporary_file_requested: false,
            raw_platform_material_export_requested: false,

            platform_sealer_runtime_requested: false,
            vault_store_runtime_requested: false,
            vault_unlock_requested: false,
            encryption_or_decryption_requested: false,
            runtime_io_requested: false,
            secret_storage_requested: false,
            capability_issuance_requested: false,
            wallet_or_ledger_mutation_requested: false,
        }
    }

    #[test]
    fn phase15h_posture_ownership_and_platforms_are_locked() {
        let posture = native_desktop_platform_storage_inspection_posture();

        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE15H_LABEL);
        assert!(posture.ownership_boundary_locked);
        assert!(posture.desktop_platform_families_locked);
        assert!(posture.atomic_write_sequence_locked);
        assert!(!posture.platform_sealer_trait_added);
        assert!(!posture.vault_store_trait_added);
        assert!(!posture.platform_sealer_adapter_added);
        assert!(!posture.vault_store_adapter_added);
        assert!(!posture.runtime_state_owner_added);
        assert!(!posture.frontend_secret_custody_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.native_secret_implementation_added);

        assert_eq!(
            PHASE15H_REQUIRED_DESKTOP_PLATFORM_FAMILIES,
            &[
                NativePlatformFamily::MacosKeychain,
                NativePlatformFamily::WindowsDpapi,
                NativePlatformFamily::LinuxSecretService,
            ]
        );

        assert_eq!(PHASE15H_REQUIRED_ATOMIC_WRITE_STEPS.len(), 6);

        assert_eq!(
            PHASE15H_TAURI_CARGO_PATH,
            "apps/crablink-tauri/src-tauri/Cargo.toml"
        );
        assert_eq!(
            PHASE15H_TAURI_STATE_PATH,
            "apps/crablink-tauri/src-tauri/src/state.rs"
        );
        assert_eq!(
            PHASE15H_TAURI_PASSPORT_COMMAND_PATH,
            "apps/crablink-tauri/src-tauri/src/commands/passport.rs"
        );

        for forbidden in [
            "plaintext_temporary_file",
            "frontend_secret_custody",
            "raw_platform_secret_export",
            "dev_kms_native_authority",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(PHASE15H_FORBIDDEN_PLATFORM_STORAGE_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase15h_accepts_current_desktop_posture_and_assigns_owners() {
        let decision = review_native_desktop_platform_storage_inspection(valid_draft())
            .expect("Phase 15H inspection");

        assert_eq!(
            decision.platform_neutral_owner,
            PHASE15H_PLATFORM_NEUTRAL_OWNER
        );
        assert_eq!(
            decision.platform_adapter_owner,
            PHASE15H_PLATFORM_ADAPTER_OWNER
        );
        assert_eq!(decision.frontend_owner, PHASE15H_FRONTEND_OWNER);

        assert!(decision.sealer_backend_pin_required);
        assert!(decision.platform_sealer_trait_required);
        assert!(decision.vault_store_trait_required);
        assert!(decision.tauri_os_adapters_required);
        assert!(decision.tauri_app_data_adapter_required);
        assert!(decision.frontend_redacted_only);
        assert!(decision.plaintext_temporary_files_forbidden);
        assert!(decision.raw_platform_material_export_forbidden);

        assert!(!decision.platform_sealer_runtime_added);
        assert!(!decision.vault_store_runtime_added);
        assert!(!decision.vault_unlock_added);
        assert!(!decision.encryption_or_decryption_added);
        assert!(!decision.runtime_io_added);
        assert!(!decision.secret_storage_added);
        assert!(!decision.capability_issuance_added);
        assert!(!decision.wallet_or_ledger_mutation_added);

        assert!(decision.ready_for_platform_trait_foundation);
        assert!(decision.inspection_only);
    }

    #[test]
    fn phase15h_rejects_missing_contract_dependency_or_ownership_posture() {
        let mut draft = valid_draft();
        draft.phase5a_sealer_contract_reviewed = false;
        assert_eq!(
            review_native_desktop_platform_storage_inspection(draft),
            Err(NativeDesktopPlatformStorageInspectionReviewError::PriorContractNotReviewed)
        );

        let mut draft = valid_draft();
        draft.platform_adapter_owner = "packages/crablink-platform";
        assert_eq!(
            review_native_desktop_platform_storage_inspection(draft),
            Err(NativeDesktopPlatformStorageInspectionReviewError::OwnershipBoundaryMismatch)
        );

        let mut draft = valid_draft();
        draft.native_passport_feature_enabled = false;
        assert_eq!(
            review_native_desktop_platform_storage_inspection(draft),
            Err(
                NativeDesktopPlatformStorageInspectionReviewError::DesktopDependencyPostureMismatch
            )
        );
    }

    #[test]
    fn phase15h_rejects_existing_runtime_frontend_or_authority_drift() {
        let runtime_mutators: &[fn(&mut NativeDesktopPlatformStorageInspectionDraftV1)] = &[
            |draft| draft.platform_sealer_backend_dependency_present = true,
            |draft| draft.platform_sealer_adapter_present = true,
            |draft| draft.vault_store_adapter_present = true,
            |draft| draft.passport_runtime_state_owner_present = true,
        ];

        for mutate in runtime_mutators {
            let mut draft = valid_draft();
            mutate(&mut draft);

            assert_eq!(
                review_native_desktop_platform_storage_inspection(
                    draft
                ),
                Err(
                    NativeDesktopPlatformStorageInspectionReviewError::
                        ExistingRuntimePostureChanged
                )
            );
        }

        let frontend_mutators: &[fn(&mut NativeDesktopPlatformStorageInspectionDraftV1)] = &[
            |draft| draft.frontend_redacted_only = false,
            |draft| draft.frontend_secret_custody_present = true,
            |draft| draft.plaintext_temporary_file_requested = true,
            |draft| draft.raw_platform_material_export_requested = true,
        ];

        for mutate in frontend_mutators {
            let mut draft = valid_draft();
            mutate(&mut draft);

            assert_eq!(
                review_native_desktop_platform_storage_inspection(draft),
                Err(NativeDesktopPlatformStorageInspectionReviewError::FrontendBoundaryUnsafe)
            );
        }

        let authority_mutators: &[fn(&mut NativeDesktopPlatformStorageInspectionDraftV1)] = &[
            |draft| draft.platform_sealer_runtime_requested = true,
            |draft| draft.vault_store_runtime_requested = true,
            |draft| draft.vault_unlock_requested = true,
            |draft| draft.encryption_or_decryption_requested = true,
            |draft| draft.runtime_io_requested = true,
            |draft| draft.secret_storage_requested = true,
            |draft| draft.capability_issuance_requested = true,
            |draft| draft.wallet_or_ledger_mutation_requested = true,
        ];

        for mutate in authority_mutators {
            let mut draft = valid_draft();
            mutate(&mut draft);

            assert_eq!(
                review_native_desktop_platform_storage_inspection(
                    draft
                ),
                Err(
                    NativeDesktopPlatformStorageInspectionReviewError::
                        UnsafePlatformStorageAuthorityFlag
                )
            );
        }
    }

    #[test]
    fn phase15h_source_remains_inspection_only() {
        let source = fs::read_to_string(repo_file(
            "src/native/desktop_platform_storage_inspection.rs",
        ))
        .expect("Phase 15H source");

        assert!(source.contains("review_native_desktop_platform_storage_inspection"));
        assert!(source.contains("PHASE15H_REQUIRED_ATOMIC_WRITE_STEPS"));

        for forbidden_runtime_pattern in [
            "#[tauri::command]",
            "tauri::",
            "keyring::",
            "security_framework::",
            "secret_service::",
            "windows::Security",
            "std::fs::",
            "tokio::fs",
            "OpenOptions::",
            "File::create",
            "rename(",
            "sync_all(",
            "unlock_vault(",
            "platform_unseal(",
            "encrypt(",
            "decrypt(",
            "issue_capability(",
            "wallet.spend(",
            "ledger.write(",
        ] {
            assert!(
                !source.contains(forbidden_runtime_pattern),
                "Phase 15H inspection must not add runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
