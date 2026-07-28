#[cfg(not(feature = "native-passport"))]
#[test]
fn phase5a_platform_sealer_contract_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native platform sealer contract DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_platform_sealer_posture, review_native_platform_sealer_contract_draft,
        validate_native_platform_sealer_contract_descriptor, NativePassportSurface,
        NativePlatformFamily, NativePlatformSealerContractDescriptorV1,
        NativePlatformSealerContractDraftV1, NativePlatformSealerContractReviewError,
        NativeSecureCompartment, NATIVE_PASSPORT_PHASE5A_LABEL, PHASE4B_ENABLED_SURFACES,
        PHASE5A_ENABLED_SURFACES, PHASE5A_FORBIDDEN_PLATFORM_SEALER_AUTHORITY_FLAGS,
        PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN, PHASE5A_REQUIRED_SECURE_COMPARTMENTS,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn valid_draft() -> NativePlatformSealerContractDraftV1 {
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

    #[test]
    fn phase5a_label_and_posture_are_locked() {
        let posture = native_passport_platform_sealer_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE5A_LABEL,
            "NATIVE_PASSPORT_PHASE5A_PLATFORM_SEALER_CONTRACT_DTO"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE5A_LABEL);
        assert!(posture.platform_sealer_contract_dtos_added);
        assert!(!posture.platform_sealer_implementation_added);
        assert!(!posture.secret_storage_added);
        assert!(!posture.material_export_added);
        assert!(!posture.encryption_runtime_added);
        assert!(!posture.decryption_runtime_added);
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
            "signing_runtime",
            "signature_verification_runtime",
            "capability_issuance",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE5A_FORBIDDEN_PLATFORM_SEALER_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase5a_surfaces_extend_phase4b_without_back_mutating_it() {
        assert_eq!(
            PHASE4B_ENABLED_SURFACES,
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
            ]
        );

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
    }

    #[test]
    fn phase5a_reviews_contract_draft_into_contract_only_descriptor() {
        let descriptor = review_native_platform_sealer_contract_draft(valid_draft())
            .expect("valid platform sealer contract draft should review");

        assert_eq!(
            descriptor.contract_domain,
            PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN
        );
        assert_eq!(
            descriptor.platform_family,
            NativePlatformFamily::MacosKeychain
        );
        assert_eq!(
            descriptor.secure_compartments,
            PHASE5A_REQUIRED_SECURE_COMPARTMENTS
        );
        assert!(descriptor.contract_only);

        validate_native_platform_sealer_contract_descriptor(&descriptor)
            .expect("reviewed contract descriptor should validate");
    }

    #[test]
    fn phase5a_rejects_domain_empty_and_duplicate_compartment_boundaries() {
        let mut bad_domain = valid_draft();
        bad_domain.contract_domain = "native-passport/platform-sealer-contract/v0";
        assert_eq!(
            review_native_platform_sealer_contract_draft(bad_domain),
            Err(NativePlatformSealerContractReviewError::ContractDomainMismatch)
        );

        let mut missing_compartments = valid_draft();
        missing_compartments.requested_compartments.clear();
        assert_eq!(
            review_native_platform_sealer_contract_draft(missing_compartments),
            Err(NativePlatformSealerContractReviewError::MissingCompartments)
        );

        let mut duplicate_compartment = valid_draft();
        duplicate_compartment
            .requested_compartments
            .push(NativeSecureCompartment::RecoveryRoot);
        assert_eq!(
            review_native_platform_sealer_contract_draft(duplicate_compartment),
            Err(NativePlatformSealerContractReviewError::DuplicateCompartment)
        );

        let bad_descriptor = NativePlatformSealerContractDescriptorV1 {
            contract_domain: "native-passport/platform-sealer-contract/v0",
            platform_family: NativePlatformFamily::AndroidKeystore,
            secure_compartments: PHASE5A_REQUIRED_SECURE_COMPARTMENTS.to_vec(),
            contract_only: true,
        };
        assert_eq!(
            validate_native_platform_sealer_contract_descriptor(&bad_descriptor),
            Err(NativePlatformSealerContractReviewError::ContractDomainMismatch)
        );
    }

    #[test]
    fn phase5a_rejects_implementation_storage_export_vault_crypto_and_wallet_ledger_flags() {
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
            let mut draft = valid_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_platform_sealer_contract_draft(draft),
                Err(NativePlatformSealerContractReviewError::UnsafePlatformSealerAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase5a_platform_family_and_compartment_labels_are_public_contract_only() {
        let platform_families = [
            NativePlatformFamily::MacosKeychain,
            NativePlatformFamily::IosKeychain,
            NativePlatformFamily::AndroidKeystore,
            NativePlatformFamily::WindowsDpapi,
            NativePlatformFamily::LinuxSecretService,
            NativePlatformFamily::UnknownLocal,
        ];

        assert_eq!(platform_families.len(), 6);
        assert_eq!(
            PHASE5A_REQUIRED_SECURE_COMPARTMENTS,
            &[
                NativeSecureCompartment::RecoveryRoot,
                NativeSecureCompartment::DeviceKey,
            ]
        );
    }

    #[test]
    fn phase5a_source_remains_contract_only_without_sealer_vault_crypto_routes_or_mutation() {
        let sealer_source =
            fs::read_to_string(repo_file("src/native/sealer.rs")).expect("sealer source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(sealer_source.contains("NativePlatformSealerContractDescriptorV1"));
        assert!(sealer_source.contains("NativePlatformSealerContractDraftV1"));
        assert!(sealer_source.contains("review_native_platform_sealer_contract_draft"));
        assert!(native_mod.contains("PlatformSealerContractDto"));

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
                "Phase 5A sealer contract source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
