#[cfg(not(feature = "native-passport"))]
#[test]
fn phase6b_pin_unlock_contract_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native PIN unlock contract DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_pin_unlock_posture, review_native_pin_unlock_contract_draft,
        review_native_platform_sealer_contract_draft, review_native_secure_surface_contract_draft,
        review_native_two_compartment_vault_header_draft,
        validate_native_pin_unlock_contract_descriptor, NativePassportSurface, NativePinPolicyV1,
        NativePinUnlockContractDraftV1, NativePinUnlockContractReviewError, NativePlatformFamily,
        NativePlatformSealerContractDraftV1, NativeSecureSurfaceContractDraftV1,
        NativeTwoCompartmentVaultHeaderDescriptorV1, NativeTwoCompartmentVaultHeaderDraftV1,
        NATIVE_PASSPORT_PHASE6B_LABEL, PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
        PHASE5A_REQUIRED_SECURE_COMPARTMENTS, PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS,
        PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN, PHASE6A_ENABLED_SURFACES,
        PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS, PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN,
        PHASE6A_VAULT_HEADER_VERSION, PHASE6B_COOLDOWN_SECONDS, PHASE6B_ENABLED_SURFACES,
        PHASE6B_FORBIDDEN_PIN_UNLOCK_AUTHORITY_FLAGS, PHASE6B_MAX_PIN_LENGTH,
        PHASE6B_MAX_UNLOCK_ATTEMPTS, PHASE6B_MIN_PIN_LENGTH, PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN,
        PHASE6B_PIN_UNLOCK_CONTRACT_VERSION, PHASE6B_REQUIRED_PIN_POLICY,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn vault_header() -> NativeTwoCompartmentVaultHeaderDescriptorV1 {
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

        review_native_two_compartment_vault_header_draft(
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
        .expect("valid vault header")
    }

    fn valid_draft() -> NativePinUnlockContractDraftV1 {
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
        }
    }

    #[test]
    fn phase6b_label_and_posture_are_locked() {
        let posture = native_passport_pin_unlock_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE6B_LABEL,
            "NATIVE_PASSPORT_PHASE6B_PIN_UNLOCK_CONTRACT_DTO"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE6B_LABEL);
        assert!(posture.pin_unlock_contract_dtos_added);
        assert!(!posture.pin_validation_runtime_added);
        assert!(!posture.pin_derivation_runtime_added);
        assert!(!posture.pin_secret_storage_added);
        assert!(!posture.vault_unlock_added);
        assert!(!posture.vault_runtime_added);
        assert!(!posture.platform_sealer_implementation_added);
        assert!(!posture.secret_storage_added);
        assert!(!posture.material_export_added);
        assert!(!posture.encryption_runtime_added);
        assert!(!posture.decryption_runtime_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.signing_runtime_added);
        assert!(!posture.signature_verification_runtime_added);
        assert!(!posture.capability_issuance_added);
        assert!(!posture.runtime_authority_changed);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "pin_validation_runtime",
            "pin_derivation_runtime",
            "pin_secret_storage",
            "vault_unlock",
            "vault_runtime",
            "platform_sealer_implementation",
            "secret_storage",
            "material_export",
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
            assert!(PHASE6B_FORBIDDEN_PIN_UNLOCK_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase6b_surfaces_extend_phase6a_without_back_mutating_it() {
        assert_eq!(
            PHASE6A_ENABLED_SURFACES,
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
            ]
        );

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
    }

    #[test]
    fn phase6b_reviews_pin_unlock_contract_draft_into_contract_only_descriptor() {
        let vault_header = vault_header();
        let descriptor = review_native_pin_unlock_contract_draft(&vault_header, valid_draft())
            .expect("valid PIN unlock contract should review");

        assert_eq!(
            descriptor.contract_domain,
            PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN
        );
        assert_eq!(
            descriptor.contract_version,
            PHASE6B_PIN_UNLOCK_CONTRACT_VERSION
        );
        assert_eq!(
            descriptor.platform_family,
            NativePlatformFamily::MacosKeychain
        );
        assert_eq!(
            descriptor.vault_header_domain,
            PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN
        );
        assert_eq!(
            descriptor.vault_header_version,
            PHASE6A_VAULT_HEADER_VERSION
        );
        assert_eq!(descriptor.pin_policy, PHASE6B_REQUIRED_PIN_POLICY);
        assert!(descriptor.contract_only);

        validate_native_pin_unlock_contract_descriptor(&vault_header, &descriptor)
            .expect("reviewed PIN unlock contract descriptor should validate");
    }

    #[test]
    fn phase6b_public_pin_policy_bounds_are_locked_without_pin_material() {
        assert_eq!(PHASE6B_PIN_UNLOCK_CONTRACT_VERSION, 1);
        assert_eq!(PHASE6B_MIN_PIN_LENGTH, 6);
        assert_eq!(PHASE6B_MAX_PIN_LENGTH, 64);
        assert_eq!(PHASE6B_MAX_UNLOCK_ATTEMPTS, 10);
        assert_eq!(PHASE6B_COOLDOWN_SECONDS, 300);

        assert_eq!(
            PHASE6B_REQUIRED_PIN_POLICY,
            NativePinPolicyV1 {
                min_pin_length: PHASE6B_MIN_PIN_LENGTH,
                max_pin_length: PHASE6B_MAX_PIN_LENGTH,
                max_unlock_attempts: PHASE6B_MAX_UNLOCK_ATTEMPTS,
                cooldown_seconds: PHASE6B_COOLDOWN_SECONDS,
                kdf_algorithm_label: "argon2id",
            }
        );
    }

    #[test]
    fn phase6b_rejects_domain_version_platform_vault_and_policy_drift() {
        let vault_header = vault_header();

        let mut bad_domain = valid_draft();
        bad_domain.contract_domain = "native-passport/pin-unlock-contract/v0";
        assert_eq!(
            review_native_pin_unlock_contract_draft(&vault_header, bad_domain),
            Err(NativePinUnlockContractReviewError::ContractDomainMismatch)
        );

        let mut bad_version = valid_draft();
        bad_version.contract_version = 2;
        assert_eq!(
            review_native_pin_unlock_contract_draft(&vault_header, bad_version),
            Err(NativePinUnlockContractReviewError::ContractVersionMismatch)
        );

        let mut bad_platform = valid_draft();
        bad_platform.platform_family = NativePlatformFamily::AndroidKeystore;
        assert_eq!(
            review_native_pin_unlock_contract_draft(&vault_header, bad_platform),
            Err(NativePinUnlockContractReviewError::PlatformFamilyMismatch)
        );

        let mut bad_header_domain = valid_draft();
        bad_header_domain.vault_header_domain = "native-passport/two-compartment-vault-header/v0";
        assert_eq!(
            review_native_pin_unlock_contract_draft(&vault_header, bad_header_domain),
            Err(NativePinUnlockContractReviewError::VaultHeaderDomainMismatch)
        );

        let mut bad_header_version = valid_draft();
        bad_header_version.vault_header_version = 2;
        assert_eq!(
            review_native_pin_unlock_contract_draft(&vault_header, bad_header_version),
            Err(NativePinUnlockContractReviewError::VaultHeaderVersionMismatch)
        );

        let mut bad_policy = valid_draft();
        bad_policy.requested_policy.min_pin_length = 4;
        assert_eq!(
            review_native_pin_unlock_contract_draft(&vault_header, bad_policy),
            Err(NativePinUnlockContractReviewError::PinPolicyMismatch)
        );
    }

    #[test]
    fn phase6b_descriptor_validator_rejects_contract_drift() {
        let vault_header = vault_header();
        let descriptor = review_native_pin_unlock_contract_draft(&vault_header, valid_draft())
            .expect("valid descriptor");

        let mut bad_domain = descriptor.clone();
        bad_domain.contract_domain = "native-passport/pin-unlock-contract/v0";
        assert_eq!(
            validate_native_pin_unlock_contract_descriptor(&vault_header, &bad_domain),
            Err(NativePinUnlockContractReviewError::ContractDomainMismatch)
        );

        let mut bad_policy = descriptor.clone();
        bad_policy.pin_policy.max_unlock_attempts = 99;
        assert_eq!(
            validate_native_pin_unlock_contract_descriptor(&vault_header, &bad_policy),
            Err(NativePinUnlockContractReviewError::PinPolicyMismatch)
        );

        let mut android_vault = vault_header.clone();
        android_vault.platform_family = NativePlatformFamily::AndroidKeystore;
        assert_eq!(
            validate_native_pin_unlock_contract_descriptor(&android_vault, &descriptor),
            Err(NativePinUnlockContractReviewError::PlatformFamilyMismatch)
        );
    }

    #[test]
    fn phase6b_rejects_pin_derivation_vault_sealer_storage_crypto_io_and_wallet_ledger_flags() {
        let vault_header = vault_header();

        for mutate in [
            |draft: &mut NativePinUnlockContractDraftV1| {
                draft.requests_pin_validation_runtime = true
            },
            |draft: &mut NativePinUnlockContractDraftV1| {
                draft.requests_pin_derivation_runtime = true
            },
            |draft: &mut NativePinUnlockContractDraftV1| draft.stores_pin_secret_material = true,
            |draft: &mut NativePinUnlockContractDraftV1| draft.requests_vault_unlock = true,
            |draft: &mut NativePinUnlockContractDraftV1| draft.includes_vault_runtime = true,
            |draft: &mut NativePinUnlockContractDraftV1| {
                draft.includes_platform_sealer_implementation = true
            },
            |draft: &mut NativePinUnlockContractDraftV1| draft.stores_secret_material = true,
            |draft: &mut NativePinUnlockContractDraftV1| draft.exports_material = true,
            |draft: &mut NativePinUnlockContractDraftV1| {
                draft.requests_encryption_or_decryption = true
            },
            |draft: &mut NativePinUnlockContractDraftV1| draft.requests_runtime_io = true,
            |draft: &mut NativePinUnlockContractDraftV1| {
                draft.requests_wallet_or_ledger_mutation = true
            },
        ] {
            let mut draft = valid_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_pin_unlock_contract_draft(&vault_header, draft),
                Err(NativePinUnlockContractReviewError::UnsafePinUnlockAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase6b_source_remains_contract_only_without_pin_values_crypto_vault_io_routes_or_mutation()
    {
        let source = fs::read_to_string(repo_file("src/native/pin.rs")).expect("pin source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativePinUnlockContractDescriptorV1"));
        assert!(source.contains("NativePinUnlockContractDraftV1"));
        assert!(source.contains("review_native_pin_unlock_contract_draft"));
        assert!(native_mod.contains("PinUnlockContractDto"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "validate_pin(",
            "check_pin(",
            "derive_pin(",
            "unlock_vault(",
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
                "Phase 6B PIN contract source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
