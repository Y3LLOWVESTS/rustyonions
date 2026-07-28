#[cfg(not(feature = "native-passport"))]
#[test]
fn phase6c_vault_contract_acceptance_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native vault contract acceptance"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_pin_unlock_posture, native_passport_vault_header_posture,
        review_native_pin_unlock_contract_draft, review_native_platform_sealer_contract_draft,
        review_native_secure_surface_contract_draft,
        review_native_two_compartment_vault_header_draft,
        validate_native_pin_unlock_contract_descriptor,
        validate_native_two_compartment_vault_header_descriptor, NativePassportSurface,
        NativePinPolicyV1, NativePinUnlockContractDraftV1, NativePinUnlockContractReviewError,
        NativePlatformFamily, NativePlatformSealerContractDraftV1, NativeSecureCompartment,
        NativeSecureSurfaceContractDescriptorV1, NativeSecureSurfaceContractDraftV1,
        NativeTwoCompartmentVaultHeaderDescriptorV1, NativeTwoCompartmentVaultHeaderDraftV1,
        NativeVaultHeaderReviewError, NATIVE_PASSPORT_PHASE6A_LABEL, NATIVE_PASSPORT_PHASE6B_LABEL,
        PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN, PHASE5A_REQUIRED_SECURE_COMPARTMENTS,
        PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS, PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
        PHASE6A_AEAD_ALGORITHM_LABEL, PHASE6A_AEAD_NONCE_LEN, PHASE6A_AEAD_TAG_LEN,
        PHASE6A_AUTHENTICATED_HEADER_LABEL, PHASE6A_ENABLED_SURFACES, PHASE6A_KDF_ALGORITHM_LABEL,
        PHASE6A_KDF_SALT_LEN, PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS,
        PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN, PHASE6A_VAULT_HEADER_VERSION,
        PHASE6B_COOLDOWN_SECONDS, PHASE6B_ENABLED_SURFACES, PHASE6B_MAX_PIN_LENGTH,
        PHASE6B_MAX_UNLOCK_ATTEMPTS, PHASE6B_MIN_PIN_LENGTH, PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN,
        PHASE6B_PIN_UNLOCK_CONTRACT_VERSION, PHASE6B_REQUIRED_PIN_POLICY,
    };

    const PHASE6C_LABEL: &str = "NATIVE_PASSPORT_PHASE6C_VAULT_CONTRACT_ACCEPTANCE";

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn secure_surface() -> NativeSecureSurfaceContractDescriptorV1 {
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
            .expect("valid platform sealer contract");

        review_native_secure_surface_contract_draft(
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
        .expect("valid secure-surface contract")
    }

    fn vault_header_draft() -> NativeTwoCompartmentVaultHeaderDraftV1 {
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
        }
    }

    fn vault_header() -> NativeTwoCompartmentVaultHeaderDescriptorV1 {
        let secure_surface = secure_surface();
        review_native_two_compartment_vault_header_draft(&secure_surface, vault_header_draft())
            .expect("valid vault header")
    }

    fn pin_unlock_draft() -> NativePinUnlockContractDraftV1 {
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
    fn phase6c_acceptance_label_and_phase6_predecessor_labels_are_locked() {
        assert_eq!(
            PHASE6C_LABEL,
            "NATIVE_PASSPORT_PHASE6C_VAULT_CONTRACT_ACCEPTANCE"
        );
        assert_eq!(
            NATIVE_PASSPORT_PHASE6A_LABEL,
            "NATIVE_PASSPORT_PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DTO"
        );
        assert_eq!(
            NATIVE_PASSPORT_PHASE6B_LABEL,
            "NATIVE_PASSPORT_PHASE6B_PIN_UNLOCK_CONTRACT_DTO"
        );
    }

    #[test]
    fn phase6c_accepts_phase6_postures_without_runtime_authority() {
        let vault_posture = native_passport_vault_header_posture();
        let pin_posture = native_passport_pin_unlock_posture();

        assert!(vault_posture.vault_header_dtos_added);
        assert!(!vault_posture.pin_unlock_added);
        assert!(!vault_posture.pin_derivation_runtime_added);
        assert!(!vault_posture.vault_runtime_added);
        assert!(!vault_posture.platform_sealer_implementation_added);
        assert!(!vault_posture.secret_storage_added);
        assert!(!vault_posture.material_export_added);
        assert!(!vault_posture.encryption_runtime_added);
        assert!(!vault_posture.decryption_runtime_added);
        assert!(!vault_posture.runtime_io_added);
        assert!(!vault_posture.signing_runtime_added);
        assert!(!vault_posture.signature_verification_runtime_added);
        assert!(!vault_posture.capability_issuance_added);
        assert!(!vault_posture.runtime_authority_changed);
        assert!(!vault_posture.native_secret_implementation_added);

        assert!(pin_posture.pin_unlock_contract_dtos_added);
        assert!(!pin_posture.pin_validation_runtime_added);
        assert!(!pin_posture.pin_derivation_runtime_added);
        assert!(!pin_posture.pin_secret_storage_added);
        assert!(!pin_posture.vault_unlock_added);
        assert!(!pin_posture.vault_runtime_added);
        assert!(!pin_posture.platform_sealer_implementation_added);
        assert!(!pin_posture.secret_storage_added);
        assert!(!pin_posture.material_export_added);
        assert!(!pin_posture.encryption_runtime_added);
        assert!(!pin_posture.decryption_runtime_added);
        assert!(!pin_posture.runtime_io_added);
        assert!(!pin_posture.signing_runtime_added);
        assert!(!pin_posture.signature_verification_runtime_added);
        assert!(!pin_posture.capability_issuance_added);
        assert!(!pin_posture.runtime_authority_changed);
        assert!(!pin_posture.native_secret_implementation_added);
    }

    #[test]
    fn phase6c_accepts_phase6_surface_chain_without_back_mutating_phase6a() {
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
    fn phase6c_accepts_secure_surface_to_vault_header_to_pin_contract_flow() {
        let secure_surface = secure_surface();
        let vault_header =
            review_native_two_compartment_vault_header_draft(&secure_surface, vault_header_draft())
                .expect("valid vault header should review");
        let pin_contract =
            review_native_pin_unlock_contract_draft(&vault_header, pin_unlock_draft())
                .expect("valid PIN contract should review");

        assert_eq!(
            vault_header.header_domain,
            PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN
        );
        assert_eq!(vault_header.header_version, PHASE6A_VAULT_HEADER_VERSION);
        assert_eq!(
            vault_header.compartment_headers,
            PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS
        );
        assert!(vault_header.header_only);

        assert_eq!(
            pin_contract.contract_domain,
            PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN
        );
        assert_eq!(
            pin_contract.contract_version,
            PHASE6B_PIN_UNLOCK_CONTRACT_VERSION
        );
        assert_eq!(pin_contract.pin_policy, PHASE6B_REQUIRED_PIN_POLICY);
        assert!(pin_contract.contract_only);

        validate_native_two_compartment_vault_header_descriptor(&secure_surface, &vault_header)
            .expect("vault header descriptor should validate");
        validate_native_pin_unlock_contract_descriptor(&vault_header, &pin_contract)
            .expect("PIN contract descriptor should validate");
    }

    #[test]
    fn phase6c_accepts_locked_header_and_pin_policy_metadata() {
        assert_eq!(PHASE6A_VAULT_HEADER_VERSION, 1);
        assert_eq!(PHASE6A_KDF_ALGORITHM_LABEL, "argon2id");
        assert_eq!(PHASE6A_AEAD_ALGORITHM_LABEL, "xchacha20poly1305");
        assert_eq!(
            PHASE6A_AUTHENTICATED_HEADER_LABEL,
            "vault-header-v1-authenticated"
        );
        assert_eq!(PHASE6A_KDF_SALT_LEN, 16);
        assert_eq!(PHASE6A_AEAD_NONCE_LEN, 24);
        assert_eq!(PHASE6A_AEAD_TAG_LEN, 16);

        assert_eq!(
            PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS[0].compartment,
            NativeSecureCompartment::RecoveryRoot
        );
        assert_eq!(
            PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS[1].compartment,
            NativeSecureCompartment::DeviceKey
        );

        assert_eq!(
            PHASE6B_REQUIRED_PIN_POLICY,
            NativePinPolicyV1 {
                min_pin_length: PHASE6B_MIN_PIN_LENGTH,
                max_pin_length: PHASE6B_MAX_PIN_LENGTH,
                max_unlock_attempts: PHASE6B_MAX_UNLOCK_ATTEMPTS,
                cooldown_seconds: PHASE6B_COOLDOWN_SECONDS,
                kdf_algorithm_label: PHASE6A_KDF_ALGORITHM_LABEL,
            }
        );
    }

    #[test]
    fn phase6c_accepts_vault_header_rejection_boundaries() {
        let secure_surface = secure_surface();

        let mut bad_domain = vault_header_draft();
        bad_domain.header_domain = "native-passport/two-compartment-vault-header/v0";
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, bad_domain),
            Err(NativeVaultHeaderReviewError::HeaderDomainMismatch)
        );

        let mut bad_version = vault_header_draft();
        bad_version.header_version = 2;
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, bad_version),
            Err(NativeVaultHeaderReviewError::HeaderVersionMismatch)
        );

        let mut missing = vault_header_draft();
        missing.requested_compartment_headers.clear();
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, missing),
            Err(NativeVaultHeaderReviewError::MissingCompartmentHeaders)
        );

        let mut duplicate = vault_header_draft();
        duplicate
            .requested_compartment_headers
            .push(PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS[0]);
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, duplicate),
            Err(NativeVaultHeaderReviewError::DuplicateCompartmentHeader)
        );

        let mut bad_algorithm = vault_header_draft();
        bad_algorithm.requested_compartment_headers[0].aead_algorithm_label = "aes-gcm";
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, bad_algorithm),
            Err(NativeVaultHeaderReviewError::AlgorithmLabelMismatch)
        );

        for mutate in [
            |draft: &mut NativeTwoCompartmentVaultHeaderDraftV1| draft.requests_pin_unlock = true,
            |draft: &mut NativeTwoCompartmentVaultHeaderDraftV1| {
                draft.requests_pin_derivation_runtime = true
            },
            |draft: &mut NativeTwoCompartmentVaultHeaderDraftV1| {
                draft.includes_vault_runtime = true
            },
            |draft: &mut NativeTwoCompartmentVaultHeaderDraftV1| {
                draft.includes_platform_sealer_implementation = true
            },
            |draft: &mut NativeTwoCompartmentVaultHeaderDraftV1| {
                draft.requests_encryption_or_decryption = true
            },
            |draft: &mut NativeTwoCompartmentVaultHeaderDraftV1| {
                draft.requests_wallet_or_ledger_mutation = true
            },
        ] {
            let mut draft = vault_header_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_two_compartment_vault_header_draft(&secure_surface, draft),
                Err(NativeVaultHeaderReviewError::UnsafeVaultHeaderAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase6c_accepts_pin_contract_rejection_boundaries() {
        let vault_header = vault_header();

        let mut bad_domain = pin_unlock_draft();
        bad_domain.contract_domain = "native-passport/pin-unlock-contract/v0";
        assert_eq!(
            review_native_pin_unlock_contract_draft(&vault_header, bad_domain),
            Err(NativePinUnlockContractReviewError::ContractDomainMismatch)
        );

        let mut bad_version = pin_unlock_draft();
        bad_version.contract_version = 2;
        assert_eq!(
            review_native_pin_unlock_contract_draft(&vault_header, bad_version),
            Err(NativePinUnlockContractReviewError::ContractVersionMismatch)
        );

        let mut bad_header_domain = pin_unlock_draft();
        bad_header_domain.vault_header_domain = "native-passport/two-compartment-vault-header/v0";
        assert_eq!(
            review_native_pin_unlock_contract_draft(&vault_header, bad_header_domain),
            Err(NativePinUnlockContractReviewError::VaultHeaderDomainMismatch)
        );

        let mut bad_policy = pin_unlock_draft();
        bad_policy.requested_policy.max_pin_length = 4;
        assert_eq!(
            review_native_pin_unlock_contract_draft(&vault_header, bad_policy),
            Err(NativePinUnlockContractReviewError::PinPolicyMismatch)
        );

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
                draft.requests_encryption_or_decryption = true
            },
            |draft: &mut NativePinUnlockContractDraftV1| draft.requests_runtime_io = true,
            |draft: &mut NativePinUnlockContractDraftV1| {
                draft.requests_wallet_or_ledger_mutation = true
            },
        ] {
            let mut draft = pin_unlock_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_pin_unlock_contract_draft(&vault_header, draft),
                Err(NativePinUnlockContractReviewError::UnsafePinUnlockAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase6c_sources_remain_contract_only_without_pin_crypto_vault_io_routes_or_mutation() {
        let vault_source =
            fs::read_to_string(repo_file("src/native/vault.rs")).expect("vault source");
        let pin_source = fs::read_to_string(repo_file("src/native/pin.rs")).expect("pin source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(vault_source.contains("review_native_two_compartment_vault_header_draft"));
        assert!(pin_source.contains("review_native_pin_unlock_contract_draft"));
        assert!(native_mod.contains("TwoCompartmentVaultHeaderDto"));
        assert!(native_mod.contains("PinUnlockContractDto"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
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
                !vault_source.contains(forbidden_runtime_pattern),
                "Phase 6 vault header source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
            assert!(
                !pin_source.contains(forbidden_runtime_pattern),
                "Phase 6 PIN contract source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
