#[cfg(not(feature = "native-passport"))]
#[test]
fn phase5b_secure_surface_contract_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native secure-surface contract DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_secure_surface_posture, review_native_platform_sealer_contract_draft,
        review_native_secure_surface_contract_draft,
        validate_native_secure_surface_contract_descriptor, NativePassportSurface,
        NativePlatformFamily, NativePlatformSealerContractDescriptorV1,
        NativePlatformSealerContractDraftV1, NativeSecureCompartment,
        NativeSecureSurfaceCompartmentBindingV1, NativeSecureSurfaceContractDraftV1,
        NativeSecureSurfaceContractReviewError, NATIVE_PASSPORT_PHASE5B_LABEL,
        PHASE5A_ENABLED_SURFACES, PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
        PHASE5A_REQUIRED_SECURE_COMPARTMENTS, PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL,
        PHASE5B_ENABLED_SURFACES, PHASE5B_FORBIDDEN_SECURE_SURFACE_AUTHORITY_FLAGS,
        PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL, PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS,
        PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn platform_contract() -> NativePlatformSealerContractDescriptorV1 {
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
        .expect("valid platform sealer contract descriptor")
    }

    fn valid_draft() -> NativeSecureSurfaceContractDraftV1 {
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
    fn phase5b_label_and_posture_are_locked() {
        let posture = native_passport_secure_surface_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE5B_LABEL,
            "NATIVE_PASSPORT_PHASE5B_SECURE_SURFACE_COMPARTMENT_CONTRACT"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE5B_LABEL);
        assert!(posture.secure_surface_contract_dtos_added);
        assert!(!posture.platform_sealer_implementation_added);
        assert!(!posture.secret_storage_added);
        assert!(!posture.material_export_added);
        assert!(!posture.encryption_runtime_added);
        assert!(!posture.decryption_runtime_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.vault_runtime_added);
        assert!(!posture.signing_runtime_added);
        assert!(!posture.signature_verification_runtime_added);
        assert!(!posture.capability_issuance_added);
        assert!(!posture.runtime_authority_changed);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "platform_sealer_implementation",
            "secret_storage",
            "material_export",
            "vault_unlock",
            "encryption_runtime",
            "decryption_runtime",
            "runtime_io",
            "signing_runtime",
            "signature_verification_runtime",
            "capability_issuance",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE5B_FORBIDDEN_SECURE_SURFACE_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase5b_surfaces_extend_phase5a_without_back_mutating_it() {
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
    fn phase5b_reviews_secure_surface_contract_draft_into_contract_only_descriptor() {
        let platform_contract = platform_contract();
        let descriptor =
            review_native_secure_surface_contract_draft(&platform_contract, valid_draft())
                .expect("valid secure-surface contract draft should review");

        assert_eq!(
            descriptor.contract_domain,
            PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN
        );
        assert_eq!(
            descriptor.platform_family,
            NativePlatformFamily::MacosKeychain
        );
        assert_eq!(
            descriptor.platform_contract_domain,
            PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN
        );
        assert_eq!(
            descriptor.compartment_bindings,
            PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS
        );
        assert!(descriptor.contract_only);

        validate_native_secure_surface_contract_descriptor(&platform_contract, &descriptor)
            .expect("reviewed secure-surface descriptor should validate");
    }

    #[test]
    fn phase5b_required_bindings_lock_labels_domains_and_compartment_order() {
        assert_eq!(
            PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS,
            &[
                NativeSecureSurfaceCompartmentBindingV1 {
                    compartment: NativeSecureCompartment::RecoveryRoot,
                    compartment_label: PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL,
                    binding_domain: PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
                    platform_contract_domain: PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
                },
                NativeSecureSurfaceCompartmentBindingV1 {
                    compartment: NativeSecureCompartment::DeviceKey,
                    compartment_label: PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL,
                    binding_domain: PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
                    platform_contract_domain: PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
                },
            ]
        );

        assert_eq!(
            PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL,
            "passport_root_compartment"
        );
        assert_eq!(PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL, "device_compartment");
    }

    #[test]
    fn phase5b_rejects_domain_platform_family_missing_duplicate_and_label_boundaries() {
        let platform_contract = platform_contract();

        let mut bad_domain = valid_draft();
        bad_domain.contract_domain = "native-passport/secure-surface-compartment/v0";
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, bad_domain),
            Err(NativeSecureSurfaceContractReviewError::ContractDomainMismatch)
        );

        let mut bad_platform_domain = valid_draft();
        bad_platform_domain.platform_contract_domain =
            "native-passport/platform-sealer-contract/v0";
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, bad_platform_domain),
            Err(NativeSecureSurfaceContractReviewError::PlatformContractDomainMismatch)
        );

        let mut bad_family = valid_draft();
        bad_family.platform_family = NativePlatformFamily::AndroidKeystore;
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, bad_family),
            Err(NativeSecureSurfaceContractReviewError::PlatformFamilyMismatch)
        );

        let mut missing = valid_draft();
        missing.requested_bindings.clear();
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, missing),
            Err(NativeSecureSurfaceContractReviewError::MissingBindings)
        );

        let mut duplicate = valid_draft();
        duplicate
            .requested_bindings
            .push(PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS[0]);
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, duplicate),
            Err(NativeSecureSurfaceContractReviewError::DuplicateCompartmentBinding)
        );

        let mut missing_required = valid_draft();
        missing_required.requested_bindings = vec![PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS[0]];
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, missing_required),
            Err(NativeSecureSurfaceContractReviewError::MissingRequiredCompartmentBinding)
        );

        let mut bad_label = valid_draft();
        bad_label.requested_bindings[0].compartment_label = "wrong_root_compartment";
        assert_eq!(
            review_native_secure_surface_contract_draft(&platform_contract, bad_label),
            Err(NativeSecureSurfaceContractReviewError::CompartmentLabelMismatch)
        );
    }

    #[test]
    fn phase5b_descriptor_validator_rejects_contract_drift() {
        let platform_contract = platform_contract();
        let descriptor =
            review_native_secure_surface_contract_draft(&platform_contract, valid_draft())
                .expect("valid descriptor");

        let mut bad_domain = descriptor.clone();
        bad_domain.contract_domain = "native-passport/secure-surface-compartment/v0";
        assert_eq!(
            validate_native_secure_surface_contract_descriptor(&platform_contract, &bad_domain),
            Err(NativeSecureSurfaceContractReviewError::ContractDomainMismatch)
        );

        let mut bad_platform_domain = descriptor.clone();
        bad_platform_domain.platform_contract_domain =
            "native-passport/platform-sealer-contract/v0";
        assert_eq!(
            validate_native_secure_surface_contract_descriptor(
                &platform_contract,
                &bad_platform_domain
            ),
            Err(NativeSecureSurfaceContractReviewError::PlatformContractDomainMismatch)
        );

        let android_platform_contract = NativePlatformSealerContractDescriptorV1 {
            platform_family: NativePlatformFamily::AndroidKeystore,
            ..platform_contract.clone()
        };
        assert_eq!(
            validate_native_secure_surface_contract_descriptor(
                &android_platform_contract,
                &descriptor
            ),
            Err(NativeSecureSurfaceContractReviewError::PlatformFamilyMismatch)
        );

        let mut bad_label = descriptor.clone();
        bad_label.compartment_bindings[1].compartment_label = "wrong_device_compartment";
        assert_eq!(
            validate_native_secure_surface_contract_descriptor(&platform_contract, &bad_label),
            Err(NativeSecureSurfaceContractReviewError::CompartmentLabelMismatch)
        );
    }

    #[test]
    fn phase5b_rejects_implementation_storage_export_vault_crypto_io_and_wallet_ledger_flags() {
        let platform_contract = platform_contract();

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
            let mut draft = valid_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_secure_surface_contract_draft(&platform_contract, draft),
                Err(NativeSecureSurfaceContractReviewError::UnsafeSecureSurfaceAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase5b_source_remains_contract_only_without_sealer_vault_crypto_io_routes_or_mutation() {
        let source = fs::read_to_string(repo_file("src/native/secure_surface.rs")).expect("source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeSecureSurfaceContractDescriptorV1"));
        assert!(source.contains("NativeSecureSurfaceContractDraftV1"));
        assert!(source.contains("review_native_secure_surface_contract_draft"));
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
                !source.contains(forbidden_runtime_pattern),
                "Phase 5B secure-surface contract source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
