#[cfg(not(feature = "native-passport"))]
#[test]
fn phase5c_sealer_surface_acceptance_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native sealer/surface acceptance"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_platform_sealer_posture, native_passport_secure_surface_posture,
        review_native_platform_sealer_contract_draft, review_native_secure_surface_contract_draft,
        validate_native_platform_sealer_contract_descriptor,
        validate_native_secure_surface_contract_descriptor, NativePassportSurface,
        NativePlatformFamily, NativePlatformSealerContractDraftV1,
        NativePlatformSealerContractReviewError, NativeSecureCompartment,
        NativeSecureSurfaceContractDraftV1, NativeSecureSurfaceContractReviewError,
        NATIVE_PASSPORT_PHASE5A_LABEL, NATIVE_PASSPORT_PHASE5B_LABEL, PHASE5A_ENABLED_SURFACES,
        PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN, PHASE5A_REQUIRED_SECURE_COMPARTMENTS,
        PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL, PHASE5B_ENABLED_SURFACES,
        PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL, PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS,
        PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
    };

    const PHASE5C_LABEL: &str = "NATIVE_PASSPORT_PHASE5C_SEALER_SURFACE_ACCEPTANCE";

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn platform_contract_draft() -> NativePlatformSealerContractDraftV1 {
        NativePlatformSealerContractDraftV1 {
            contract_domain: PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
            platform_family: NativePlatformFamily::MacosKeychain,
            requested_compartments: PHASE5A_REQUIRED_SECURE_COMPARTMENTS.to_vec(),
            includes_platform_sealer_implementation: false,
            stores_secret_material: false,
            exports_material: false,
            requests_vault_unlock: false,
            requests_encryption_or_decryption: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    fn secure_surface_draft() -> NativeSecureSurfaceContractDraftV1 {
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
        }
    }

    #[test]
    fn phase5c_acceptance_label_and_phase5_predecessor_labels_are_locked() {
        assert_eq!(
            PHASE5C_LABEL,
            "NATIVE_PASSPORT_PHASE5C_SEALER_SURFACE_ACCEPTANCE"
        );
        assert_eq!(
            NATIVE_PASSPORT_PHASE5A_LABEL,
            "NATIVE_PASSPORT_PHASE5A_PLATFORM_SEALER_CONTRACT_DTO"
        );
        assert_eq!(
            NATIVE_PASSPORT_PHASE5B_LABEL,
            "NATIVE_PASSPORT_PHASE5B_SECURE_SURFACE_COMPARTMENT_CONTRACT"
        );
    }

    #[test]
    fn phase5c_accepts_phase5_postures_without_runtime_authority() {
        let sealer_posture = native_passport_platform_sealer_posture();
        let surface_posture = native_passport_secure_surface_posture();

        assert!(sealer_posture.platform_sealer_contract_dtos_added);
        assert!(!sealer_posture.platform_sealer_implementation_added);
        assert!(!sealer_posture.secret_storage_added);
        assert!(!sealer_posture.material_export_added);
        assert!(!sealer_posture.encryption_runtime_added);
        assert!(!sealer_posture.decryption_runtime_added);
        assert!(!sealer_posture.vault_runtime_added);
        assert!(!sealer_posture.signing_runtime_added);
        assert!(!sealer_posture.signature_verification_runtime_added);
        assert!(!sealer_posture.capability_issuance_added);
        assert!(!sealer_posture.runtime_authority_changed);
        assert!(!sealer_posture.native_secret_implementation_added);

        assert!(surface_posture.secure_surface_contract_dtos_added);
        assert!(!surface_posture.platform_sealer_implementation_added);
        assert!(!surface_posture.secret_storage_added);
        assert!(!surface_posture.material_export_added);
        assert!(!surface_posture.encryption_runtime_added);
        assert!(!surface_posture.decryption_runtime_added);
        assert!(!surface_posture.runtime_io_added);
        assert!(!surface_posture.vault_runtime_added);
        assert!(!surface_posture.signing_runtime_added);
        assert!(!surface_posture.signature_verification_runtime_added);
        assert!(!surface_posture.capability_issuance_added);
        assert!(!surface_posture.runtime_authority_changed);
        assert!(!surface_posture.native_secret_implementation_added);
    }

    #[test]
    fn phase5c_accepts_phase5_surface_chain_without_back_mutating_phase5a() {
        assert_eq!(
            PHASE5A_ENABLED_SURFACES,
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
            ]
        );

        assert_eq!(
            PHASE5B_ENABLED_SURFACES,
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
            ]
        );
    }

    #[test]
    fn phase5c_accepts_platform_contract_to_secure_surface_flow() {
        let platform_contract =
            review_native_platform_sealer_contract_draft(platform_contract_draft())
                .expect("valid platform sealer contract should review");

        let secure_surface =
            review_native_secure_surface_contract_draft(&platform_contract, secure_surface_draft())
                .expect("valid secure-surface contract should review");

        assert_eq!(
            platform_contract.contract_domain,
            PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN
        );
        assert_eq!(
            platform_contract.secure_compartments,
            PHASE5A_REQUIRED_SECURE_COMPARTMENTS
        );
        assert!(platform_contract.contract_only);

        assert_eq!(
            secure_surface.contract_domain,
            PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN
        );
        assert_eq!(
            secure_surface.platform_contract_domain,
            PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN
        );
        assert_eq!(
            secure_surface.compartment_bindings,
            PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS
        );
        assert!(secure_surface.contract_only);

        validate_native_platform_sealer_contract_descriptor(&platform_contract)
            .expect("platform contract descriptor should validate");
        validate_native_secure_surface_contract_descriptor(&platform_contract, &secure_surface)
            .expect("secure-surface descriptor should validate");
    }

    #[test]
    fn phase5c_accepts_locked_compartment_contract_labels() {
        assert_eq!(
            PHASE5A_REQUIRED_SECURE_COMPARTMENTS,
            &[
                NativeSecureCompartment::RecoveryRoot,
                NativeSecureCompartment::DeviceKey,
            ]
        );

        assert_eq!(
            PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS[0].compartment,
            NativeSecureCompartment::RecoveryRoot
        );
        assert_eq!(
            PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS[0].compartment_label,
            PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL
        );
        assert_eq!(
            PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS[1].compartment,
            NativeSecureCompartment::DeviceKey
        );
        assert_eq!(
            PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS[1].compartment_label,
            PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL
        );

        assert_eq!(
            PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL,
            "passport_root_compartment"
        );
        assert_eq!(PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL, "device_compartment");
    }

    #[test]
    fn phase5c_accepts_platform_contract_rejection_boundaries() {
        let mut bad_domain = platform_contract_draft();
        bad_domain.contract_domain = "native-passport/platform-sealer-contract/v0";
        assert_eq!(
            review_native_platform_sealer_contract_draft(bad_domain),
            Err(NativePlatformSealerContractReviewError::ContractDomainMismatch)
        );

        let mut missing = platform_contract_draft();
        missing.requested_compartments.clear();
        assert_eq!(
            review_native_platform_sealer_contract_draft(missing),
            Err(NativePlatformSealerContractReviewError::MissingCompartments)
        );

        let mut duplicate = platform_contract_draft();
        duplicate
            .requested_compartments
            .push(NativeSecureCompartment::RecoveryRoot);
        assert_eq!(
            review_native_platform_sealer_contract_draft(duplicate),
            Err(NativePlatformSealerContractReviewError::DuplicateCompartment)
        );

        for mutate in [
            |draft: &mut NativePlatformSealerContractDraftV1| {
                draft.includes_platform_sealer_implementation = true
            },
            |draft: &mut NativePlatformSealerContractDraftV1| draft.stores_secret_material = true,
            |draft: &mut NativePlatformSealerContractDraftV1| draft.exports_material = true,
            |draft: &mut NativePlatformSealerContractDraftV1| draft.requests_vault_unlock = true,
            |draft: &mut NativePlatformSealerContractDraftV1| {
                draft.requests_encryption_or_decryption = true
            },
            |draft: &mut NativePlatformSealerContractDraftV1| {
                draft.requests_wallet_or_ledger_mutation = true
            },
        ] {
            let mut draft = platform_contract_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_platform_sealer_contract_draft(draft),
                Err(NativePlatformSealerContractReviewError::UnsafePlatformSealerAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase5c_accepts_secure_surface_rejection_boundaries() {
        let platform_contract =
            review_native_platform_sealer_contract_draft(platform_contract_draft())
                .expect("valid platform contract");

        let mut bad_domain = secure_surface_draft();
        bad_domain.contract_domain = "native-passport/secure-surface-compartment/v0";
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, bad_domain),
            Err(NativeSecureSurfaceContractReviewError::ContractDomainMismatch)
        );

        let mut bad_platform_domain = secure_surface_draft();
        bad_platform_domain.platform_contract_domain =
            "native-passport/platform-sealer-contract/v0";
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, bad_platform_domain),
            Err(NativeSecureSurfaceContractReviewError::PlatformContractDomainMismatch)
        );

        let mut bad_family = secure_surface_draft();
        bad_family.platform_family = NativePlatformFamily::AndroidKeystore;
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, bad_family),
            Err(NativeSecureSurfaceContractReviewError::PlatformFamilyMismatch)
        );

        let mut missing = secure_surface_draft();
        missing.requested_bindings.clear();
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, missing),
            Err(NativeSecureSurfaceContractReviewError::MissingBindings)
        );

        let mut duplicate = secure_surface_draft();
        duplicate
            .requested_bindings
            .push(PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS[0]);
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, duplicate),
            Err(NativeSecureSurfaceContractReviewError::DuplicateCompartmentBinding)
        );

        let mut missing_required = secure_surface_draft();
        missing_required.requested_bindings = vec![PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS[0]];
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, missing_required),
            Err(NativeSecureSurfaceContractReviewError::MissingRequiredCompartmentBinding)
        );

        let mut bad_label = secure_surface_draft();
        bad_label.requested_bindings[1].compartment_label = "wrong_device_compartment";
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, bad_label),
            Err(NativeSecureSurfaceContractReviewError::CompartmentLabelMismatch)
        );

        for mutate in [
            |draft: &mut NativeSecureSurfaceContractDraftV1| {
                draft.includes_platform_sealer_implementation = true
            },
            |draft: &mut NativeSecureSurfaceContractDraftV1| draft.stores_secret_material = true,
            |draft: &mut NativeSecureSurfaceContractDraftV1| draft.exports_material = true,
            |draft: &mut NativeSecureSurfaceContractDraftV1| draft.requests_vault_unlock = true,
            |draft: &mut NativeSecureSurfaceContractDraftV1| {
                draft.requests_encryption_or_decryption = true
            },
            |draft: &mut NativeSecureSurfaceContractDraftV1| draft.requests_runtime_io = true,
            |draft: &mut NativeSecureSurfaceContractDraftV1| {
                draft.requests_wallet_or_ledger_mutation = true
            },
        ] {
            let mut draft = secure_surface_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_secure_surface_contract_draft(&platform_contract, draft),
                Err(NativeSecureSurfaceContractReviewError::UnsafeSecureSurfaceAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase5c_sources_remain_contract_only_without_runtime_sealer_vault_crypto_routes_or_mutation()
    {
        let sealer_source =
            fs::read_to_string(repo_file("src/native/sealer.rs")).expect("sealer source");
        let surface_source =
            fs::read_to_string(repo_file("src/native/secure_surface.rs")).expect("surface source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(sealer_source.contains("review_native_platform_sealer_contract_draft"));
        assert!(surface_source.contains("review_native_secure_surface_contract_draft"));
        assert!(native_mod.contains("PlatformSealerContractDto"));
        assert!(native_mod.contains("SecureSurfaceCompartmentContract"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "generate_recovery_root(",
            "generate_device_key(",
            "store_recovery_secret(",
            "store_device_secret(",
            "platform_seal(",
            "platform_unseal(",
            "seal_bytes(",
            "unseal_bytes(",
            "encrypt(",
            "decrypt(",
            "runtime_read(",
            "runtime_write(",
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
            "root_private_key:",
            "device_private_key:",
            "vault_master_key:",
            "derived_vault_key:",
        ] {
            assert!(
                !sealer_source.contains(forbidden_runtime_pattern),
                "Phase 5 sealer contract source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
            assert!(
                !surface_source.contains(forbidden_runtime_pattern),
                "Phase 5 secure-surface source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
