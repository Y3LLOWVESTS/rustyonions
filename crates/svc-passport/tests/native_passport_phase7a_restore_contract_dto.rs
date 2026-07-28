#[cfg(not(feature = "native-passport"))]
#[test]
fn phase7a_restore_contract_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native restore contract DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_restore_posture, review_native_pin_unlock_contract_draft,
        review_native_platform_sealer_contract_draft, review_native_restore_contract_draft,
        review_native_secure_surface_contract_draft,
        review_native_two_compartment_vault_header_draft,
        validate_native_restore_contract_descriptor, NativePassportSurface,
        NativePinUnlockContractDescriptorV1, NativePinUnlockContractDraftV1, NativePlatformFamily,
        NativePlatformSealerContractDraftV1, NativeRestoreContractDraftV1,
        NativeRestoreContractReviewError, NativeRestoreIntent, NativeRestoreSource,
        NativeSecureSurfaceContractDraftV1, NativeTwoCompartmentVaultHeaderDraftV1,
        NATIVE_PASSPORT_PHASE7A_LABEL, PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
        PHASE5A_REQUIRED_SECURE_COMPARTMENTS, PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS,
        PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN, PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS,
        PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN, PHASE6A_VAULT_HEADER_VERSION,
        PHASE6B_ENABLED_SURFACES, PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN,
        PHASE6B_PIN_UNLOCK_CONTRACT_VERSION, PHASE6B_REQUIRED_PIN_POLICY,
        PHASE7A_ALLOWED_RESTORE_SOURCES, PHASE7A_ENABLED_SURFACES,
        PHASE7A_FORBIDDEN_RESTORE_AUTHORITY_FLAGS, PHASE7A_REQUIRED_RESTORE_INTENTS,
        PHASE7A_RESTORE_CONTRACT_DOMAIN, PHASE7A_RESTORE_CONTRACT_VERSION,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn pin_contract() -> NativePinUnlockContractDescriptorV1 {
        let platform_contract =
            review_native_platform_sealer_contract_draft(NativePlatformSealerContractDraftV1 {
                contract_domain: PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
                platform_family: NativePlatformFamily::MacosKeychain,
                requested_compartments: PHASE5A_REQUIRED_SECURE_COMPARTMENTS.to_vec(),
                includes_platform_sealer_implementation: false,
                stores_secret_material: false,
                exports_material: false,
                requests_vault_unlock: false,
                requests_encryption_or_decryption: false,
                requests_wallet_or_ledger_mutation: false,
            })
            .expect("valid platform contract");

        let secure_surface = review_native_secure_surface_contract_draft(
            &platform_contract,
            NativeSecureSurfaceContractDraftV1 {
                contract_domain: PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
                platform_family: NativePlatformFamily::MacosKeychain,
                platform_contract_domain: PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
                requested_bindings: PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS.to_vec(),
                includes_platform_sealer_implementation: false,
                stores_secret_material: false,
                exports_material: false,
                requests_vault_unlock: false,
                requests_encryption_or_decryption: false,
                requests_runtime_io: false,
                requests_wallet_or_ledger_mutation: false,
            },
        )
        .expect("valid secure surface");

        let vault_header = review_native_two_compartment_vault_header_draft(
            &secure_surface,
            NativeTwoCompartmentVaultHeaderDraftV1 {
                header_domain: PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN,
                header_version: PHASE6A_VAULT_HEADER_VERSION,
                platform_family: NativePlatformFamily::MacosKeychain,
                secure_surface_contract_domain: PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
                requested_compartment_headers: PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS.to_vec(),
                requests_pin_unlock: false,
                requests_pin_derivation_runtime: false,
                includes_vault_runtime: false,
                includes_platform_sealer_implementation: false,
                stores_secret_material: false,
                exports_material: false,
                requests_encryption_or_decryption: false,
                requests_runtime_io: false,
                requests_wallet_or_ledger_mutation: false,
            },
        )
        .expect("valid vault header");

        review_native_pin_unlock_contract_draft(
            &vault_header,
            NativePinUnlockContractDraftV1 {
                contract_domain: PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN,
                contract_version: PHASE6B_PIN_UNLOCK_CONTRACT_VERSION,
                platform_family: NativePlatformFamily::MacosKeychain,
                vault_header_domain: PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN,
                vault_header_version: PHASE6A_VAULT_HEADER_VERSION,
                requested_policy: PHASE6B_REQUIRED_PIN_POLICY,
                requests_pin_validation_runtime: false,
                requests_pin_derivation_runtime: false,
                stores_pin_secret_material: false,
                requests_vault_unlock: false,
                includes_vault_runtime: false,
                includes_platform_sealer_implementation: false,
                stores_secret_material: false,
                exports_material: false,
                requests_encryption_or_decryption: false,
                requests_runtime_io: false,
                requests_wallet_or_ledger_mutation: false,
            },
        )
        .expect("valid PIN contract")
    }

    fn valid_draft() -> NativeRestoreContractDraftV1 {
        NativeRestoreContractDraftV1 {
            contract_domain: PHASE7A_RESTORE_CONTRACT_DOMAIN,
            contract_version: PHASE7A_RESTORE_CONTRACT_VERSION,
            platform_family: NativePlatformFamily::MacosKeychain,
            pin_contract_domain: PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN,
            pin_contract_version: PHASE6B_PIN_UNLOCK_CONTRACT_VERSION,
            vault_header_domain: PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN,
            vault_header_version: PHASE6A_VAULT_HEADER_VERSION,
            requested_restore_intents: PHASE7A_REQUIRED_RESTORE_INTENTS.to_vec(),
            requested_restore_sources: PHASE7A_ALLOWED_RESTORE_SOURCES.to_vec(),
            requests_restore_execution: false,
            imports_recovery_phrase_material: false,
            requests_key_derivation_runtime: false,
            requests_pin_validation_runtime: false,
            requests_pin_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            includes_platform_sealer_implementation: false,
            stores_secret_material: false,
            exports_secret_or_material: false,
            requests_encryption_or_decryption: false,
            requests_delegated_enrollment_runtime: false,
            requests_runtime_io: false,
            adds_routes: false,
            issues_capabilities: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase7a_label_and_posture_are_locked() {
        let posture = native_passport_restore_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE7A_LABEL,
            "NATIVE_PASSPORT_PHASE7A_RESTORE_CONTRACT_DTO"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE7A_LABEL);
        assert!(posture.restore_contract_dtos_added);
        assert!(!posture.restore_execution_added);
        assert!(!posture.mnemonic_or_seed_import_added);
        assert!(!posture.key_derivation_runtime_added);
        assert!(!posture.pin_validation_runtime_added);
        assert!(!posture.pin_derivation_runtime_added);
        assert!(!posture.vault_unlock_added);
        assert!(!posture.vault_runtime_added);
        assert!(!posture.platform_sealer_implementation_added);
        assert!(!posture.secret_storage_added);
        assert!(!posture.material_export_added);
        assert!(!posture.encryption_runtime_added);
        assert!(!posture.decryption_runtime_added);
        assert!(!posture.delegated_enrollment_runtime_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.routes_added);
        assert!(!posture.capability_issuance_added);
        assert!(!posture.runtime_authority_changed);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "restore_execution",
            "mnemonic_import",
            "seed_phrase_import",
            "root_key_derivation",
            "device_key_derivation",
            "pin_validation_runtime",
            "pin_derivation_runtime",
            "vault_unlock",
            "vault_runtime",
            "platform_sealer_implementation",
            "secret_storage",
            "secret_export",
            "material_export",
            "encryption_runtime",
            "decryption_runtime",
            "delegated_enrollment_runtime",
            "runtime_io",
            "route_added",
            "capability_issuance",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE7A_FORBIDDEN_RESTORE_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase7a_surfaces_extend_phase6b_without_back_mutating_it() {
        assert_eq!(
            PHASE6B_ENABLED_SURFACES,
            &[
                NativePassportSurface::FeaturePosture,
                NativePassportSurface::Phase0Contracts,
                NativePassportSurface::NativeDtoTypes,
                NativePassportSurface::NativeStatusRedaction,
                NativePassportSurface::NativeUsernameHandles,
                NativePassportSurface::RootDeviceAuthorizationDtos,
                NativePassportSurface::PureAuthorizationReview,
                NativePassportSurface::NativeRecoveryRootDto,
                NativePassportSurface::NativeDeviceKeyDto,
                NativePassportSurface::PlatformSealerContractDto,
                NativePassportSurface::SecureSurfaceCompartmentContract,
                NativePassportSurface::TwoCompartmentVaultHeaderDto,
                NativePassportSurface::PinUnlockContractDto,
            ]
        );

        assert_eq!(
            PHASE7A_ENABLED_SURFACES,
            &[
                NativePassportSurface::FeaturePosture,
                NativePassportSurface::Phase0Contracts,
                NativePassportSurface::NativeDtoTypes,
                NativePassportSurface::NativeStatusRedaction,
                NativePassportSurface::NativeUsernameHandles,
                NativePassportSurface::RootDeviceAuthorizationDtos,
                NativePassportSurface::PureAuthorizationReview,
                NativePassportSurface::NativeRecoveryRootDto,
                NativePassportSurface::NativeDeviceKeyDto,
                NativePassportSurface::PlatformSealerContractDto,
                NativePassportSurface::SecureSurfaceCompartmentContract,
                NativePassportSurface::TwoCompartmentVaultHeaderDto,
                NativePassportSurface::PinUnlockContractDto,
                NativePassportSurface::RestoreContractDto,
            ]
        );
    }

    #[test]
    fn phase7a_reviews_restore_contract_draft_into_contract_only_descriptor() {
        let pin_contract = pin_contract();
        let descriptor = review_native_restore_contract_draft(&pin_contract, valid_draft())
            .expect("valid restore contract should review");

        assert_eq!(descriptor.contract_domain, PHASE7A_RESTORE_CONTRACT_DOMAIN);
        assert_eq!(
            descriptor.contract_version,
            PHASE7A_RESTORE_CONTRACT_VERSION
        );
        assert_eq!(
            descriptor.platform_family,
            NativePlatformFamily::MacosKeychain
        );
        assert_eq!(
            descriptor.pin_contract_domain,
            PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN
        );
        assert_eq!(
            descriptor.pin_contract_version,
            PHASE6B_PIN_UNLOCK_CONTRACT_VERSION
        );
        assert_eq!(
            descriptor.vault_header_domain,
            PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN
        );
        assert_eq!(
            descriptor.vault_header_version,
            PHASE6A_VAULT_HEADER_VERSION
        );
        assert_eq!(descriptor.restore_intents, PHASE7A_REQUIRED_RESTORE_INTENTS);
        assert_eq!(descriptor.restore_sources, PHASE7A_ALLOWED_RESTORE_SOURCES);
        assert!(descriptor.contract_only);

        validate_native_restore_contract_descriptor(&pin_contract, &descriptor)
            .expect("reviewed restore contract descriptor should validate");
    }

    #[test]
    fn phase7a_required_restore_intents_and_sources_are_locked() {
        assert_eq!(
            PHASE7A_REQUIRED_RESTORE_INTENTS,
            &[
                NativeRestoreIntent::RestorePassportRoot,
                NativeRestoreIntent::RestoreDeviceKey,
                NativeRestoreIntent::PrepareDelegatedEnrollment,
            ]
        );

        assert_eq!(
            PHASE7A_ALLOWED_RESTORE_SOURCES,
            &[
                NativeRestoreSource::LocalVaultHeader,
                NativeRestoreSource::PlatformSealerContract,
                NativeRestoreSource::DelegatedDeviceInvitation,
            ]
        );
    }

    #[test]
    fn phase7a_rejects_domain_version_platform_pin_vault_empty_duplicate_and_missing_intent_drift()
    {
        let pin_contract = pin_contract();

        let mut bad_domain = valid_draft();
        bad_domain.contract_domain = "native-passport/restore-contract/v0";
        assert_eq!(
            review_native_restore_contract_draft(&pin_contract, bad_domain),
            Err(NativeRestoreContractReviewError::ContractDomainMismatch)
        );

        let mut bad_version = valid_draft();
        bad_version.contract_version = 2;
        assert_eq!(
            review_native_restore_contract_draft(&pin_contract, bad_version),
            Err(NativeRestoreContractReviewError::ContractVersionMismatch)
        );

        let mut bad_platform = valid_draft();
        bad_platform.platform_family = NativePlatformFamily::AndroidKeystore;
        assert_eq!(
            review_native_restore_contract_draft(&pin_contract, bad_platform),
            Err(NativeRestoreContractReviewError::PlatformFamilyMismatch)
        );

        let mut bad_pin_domain = valid_draft();
        bad_pin_domain.pin_contract_domain = "native-passport/pin-unlock-contract/v0";
        assert_eq!(
            review_native_restore_contract_draft(&pin_contract, bad_pin_domain),
            Err(NativeRestoreContractReviewError::PinContractDomainMismatch)
        );

        let mut bad_vault_domain = valid_draft();
        bad_vault_domain.vault_header_domain = "native-passport/two-compartment-vault-header/v0";
        assert_eq!(
            review_native_restore_contract_draft(&pin_contract, bad_vault_domain),
            Err(NativeRestoreContractReviewError::VaultHeaderDomainMismatch)
        );

        let mut missing_intents = valid_draft();
        missing_intents.requested_restore_intents.clear();
        assert_eq!(
            review_native_restore_contract_draft(&pin_contract, missing_intents),
            Err(NativeRestoreContractReviewError::MissingRestoreIntents)
        );

        let mut duplicate_intent = valid_draft();
        duplicate_intent
            .requested_restore_intents
            .push(NativeRestoreIntent::RestorePassportRoot);
        assert_eq!(
            review_native_restore_contract_draft(&pin_contract, duplicate_intent),
            Err(NativeRestoreContractReviewError::DuplicateRestoreIntent)
        );

        let mut missing_required = valid_draft();
        missing_required.requested_restore_intents = vec![
            NativeRestoreIntent::RestorePassportRoot,
            NativeRestoreIntent::RestoreDeviceKey,
        ];
        assert_eq!(
            review_native_restore_contract_draft(&pin_contract, missing_required),
            Err(NativeRestoreContractReviewError::MissingRequiredRestoreIntent)
        );
    }

    #[test]
    fn phase7a_rejects_source_empty_duplicate_descriptor_and_version_drift() {
        let pin_contract = pin_contract();

        let mut missing_sources = valid_draft();
        missing_sources.requested_restore_sources.clear();
        assert_eq!(
            review_native_restore_contract_draft(&pin_contract, missing_sources),
            Err(NativeRestoreContractReviewError::MissingRestoreSources)
        );

        let mut duplicate_source = valid_draft();
        duplicate_source
            .requested_restore_sources
            .push(NativeRestoreSource::LocalVaultHeader);
        assert_eq!(
            review_native_restore_contract_draft(&pin_contract, duplicate_source),
            Err(NativeRestoreContractReviewError::DuplicateRestoreSource)
        );

        let mut descriptor = review_native_restore_contract_draft(&pin_contract, valid_draft())
            .expect("valid descriptor");
        descriptor.pin_contract_version = 2;
        assert_eq!(
            validate_native_restore_contract_descriptor(&pin_contract, &descriptor),
            Err(NativeRestoreContractReviewError::PinContractVersionMismatch)
        );

        let mut descriptor = review_native_restore_contract_draft(&pin_contract, valid_draft())
            .expect("valid descriptor");
        descriptor.vault_header_version = 2;
        assert_eq!(
            validate_native_restore_contract_descriptor(&pin_contract, &descriptor),
            Err(NativeRestoreContractReviewError::VaultHeaderVersionMismatch)
        );
    }

    #[test]
    fn phase7a_rejects_restore_import_derivation_pin_vault_sealer_crypto_enrollment_routes_and_wallet_ledger_flags(
    ) {
        let pin_contract = pin_contract();

        for mutate in [
            |draft: &mut NativeRestoreContractDraftV1| draft.requests_restore_execution = true,
            |draft: &mut NativeRestoreContractDraftV1| {
                draft.imports_recovery_phrase_material = true
            },
            |draft: &mut NativeRestoreContractDraftV1| draft.requests_key_derivation_runtime = true,
            |draft: &mut NativeRestoreContractDraftV1| draft.requests_pin_validation_runtime = true,
            |draft: &mut NativeRestoreContractDraftV1| draft.requests_pin_derivation_runtime = true,
            |draft: &mut NativeRestoreContractDraftV1| draft.requests_vault_unlock = true,
            |draft: &mut NativeRestoreContractDraftV1| draft.includes_vault_runtime = true,
            |draft: &mut NativeRestoreContractDraftV1| {
                draft.includes_platform_sealer_implementation = true
            },
            |draft: &mut NativeRestoreContractDraftV1| draft.stores_secret_material = true,
            |draft: &mut NativeRestoreContractDraftV1| draft.exports_secret_or_material = true,
            |draft: &mut NativeRestoreContractDraftV1| {
                draft.requests_encryption_or_decryption = true
            },
            |draft: &mut NativeRestoreContractDraftV1| {
                draft.requests_delegated_enrollment_runtime = true
            },
            |draft: &mut NativeRestoreContractDraftV1| draft.requests_runtime_io = true,
            |draft: &mut NativeRestoreContractDraftV1| draft.adds_routes = true,
            |draft: &mut NativeRestoreContractDraftV1| draft.issues_capabilities = true,
            |draft: &mut NativeRestoreContractDraftV1| {
                draft.requests_wallet_or_ledger_mutation = true
            },
        ] {
            let mut draft = valid_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_restore_contract_draft(&pin_contract, draft),
                Err(NativeRestoreContractReviewError::UnsafeRestoreAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase7a_source_remains_contract_only_without_restore_key_pin_vault_crypto_routes_or_mutation(
    ) {
        let source =
            fs::read_to_string(repo_file("src/native/restore.rs")).expect("restore source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeRestoreContractDescriptorV1"));
        assert!(source.contains("NativeRestoreContractDraftV1"));
        assert!(source.contains("review_native_restore_contract_draft"));
        assert!(native_mod.contains("RestoreContractDto"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "execute_restore(",
            "restore_passport(",
            "restore_device(",
            "import_mnemonic(",
            "import_seed_phrase(",
            "derive_root_key(",
            "derive_device_key(",
            "validate_pin(",
            "check_pin(",
            "derive_pin(",
            "unlock_pin(",
            "unlock_vault(",
            "platform_seal(",
            "platform_unseal(",
            "seal_bytes(",
            "unseal_bytes(",
            "encrypt(",
            "decrypt(",
            "runtime_read(",
            "runtime_write(",
            "delegate_enrollment(",
            "enroll_device(",
            "sign(",
            "verify_signature(",
            "verify_proof(",
            "vault_decrypt(",
            "vault_encrypt(",
            "issue_capability(",
            "wallet.spend(",
            "ledger.write(",
            "mint_roc(",
            "burn_roc(",
            "mnemonic:",
            "seed_phrase:",
            "pin:",
            "pin_hash:",
            "pin_value:",
            "pin_secret:",
            "root_private_key:",
            "device_private_key:",
            "vault_master_key:",
            "derived_vault_key:",
            "ciphertext:",
            "plaintext:",
            "sealed_bytes:",
        ] {
            assert!(
                !source.contains(forbidden_runtime_pattern),
                "Phase 7A restore contract source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
