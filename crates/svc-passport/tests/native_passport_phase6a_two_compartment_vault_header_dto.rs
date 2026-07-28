#[cfg(not(feature = "native-passport"))]
#[test]
fn phase6a_vault_header_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native vault header DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_vault_header_posture, review_native_platform_sealer_contract_draft,
        review_native_secure_surface_contract_draft,
        review_native_two_compartment_vault_header_draft,
        validate_native_two_compartment_vault_header_descriptor, NativePassportSurface,
        NativePlatformFamily, NativePlatformSealerContractDraftV1, NativeSecureCompartment,
        NativeSecureSurfaceContractDescriptorV1, NativeSecureSurfaceContractDraftV1,
        NativeTwoCompartmentVaultHeaderDraftV1, NativeVaultHeaderReviewError,
        NATIVE_PASSPORT_PHASE6A_LABEL, PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
        PHASE5A_REQUIRED_SECURE_COMPARTMENTS, PHASE5B_ENABLED_SURFACES,
        PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS, PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
        PHASE6A_AEAD_ALGORITHM_LABEL, PHASE6A_AEAD_NONCE_LEN, PHASE6A_AEAD_TAG_LEN,
        PHASE6A_AUTHENTICATED_HEADER_LABEL, PHASE6A_ENABLED_SURFACES,
        PHASE6A_FORBIDDEN_VAULT_HEADER_AUTHORITY_FLAGS, PHASE6A_KDF_ALGORITHM_LABEL,
        PHASE6A_KDF_SALT_LEN, PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS,
        PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN, PHASE6A_VAULT_HEADER_VERSION,
    };

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
            .expect("valid platform contract");

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
        .expect("valid secure surface contract")
    }

    fn valid_draft() -> NativeTwoCompartmentVaultHeaderDraftV1 {
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

    #[test]
    fn phase6a_label_and_posture_are_locked() {
        let posture = native_passport_vault_header_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE6A_LABEL,
            "NATIVE_PASSPORT_PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DTO"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE6A_LABEL);
        assert!(posture.vault_header_dtos_added);
        assert!(!posture.pin_unlock_added);
        assert!(!posture.pin_derivation_runtime_added);
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
            "pin_unlock",
            "pin_derivation_runtime",
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
            assert!(PHASE6A_FORBIDDEN_VAULT_HEADER_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase6a_surfaces_extend_phase5b_without_back_mutating_it() {
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
    }

    #[test]
    fn phase6a_reviews_two_compartment_vault_header_draft_into_header_only_descriptor() {
        let secure_surface = secure_surface();
        let descriptor =
            review_native_two_compartment_vault_header_draft(&secure_surface, valid_draft())
                .expect("valid two-compartment vault header should review");

        assert_eq!(
            descriptor.header_domain,
            PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN
        );
        assert_eq!(descriptor.header_version, PHASE6A_VAULT_HEADER_VERSION);
        assert_eq!(
            descriptor.platform_family,
            NativePlatformFamily::MacosKeychain
        );
        assert_eq!(
            descriptor.secure_surface_contract_domain,
            PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN
        );
        assert_eq!(
            descriptor.compartment_headers,
            PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS
        );
        assert!(descriptor.header_only);

        validate_native_two_compartment_vault_header_descriptor(&secure_surface, &descriptor)
            .expect("reviewed vault header descriptor should validate");
    }

    #[test]
    fn phase6a_header_labels_algorithms_and_lengths_are_locked() {
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
    }

    #[test]
    fn phase6a_rejects_domain_version_platform_surface_missing_duplicate_and_required_boundaries() {
        let secure_surface = secure_surface();

        let mut bad_domain = valid_draft();
        bad_domain.header_domain = "native-passport/two-compartment-vault-header/v0";
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, bad_domain),
            Err(NativeVaultHeaderReviewError::HeaderDomainMismatch)
        );

        let mut bad_version = valid_draft();
        bad_version.header_version = 2;
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, bad_version),
            Err(NativeVaultHeaderReviewError::HeaderVersionMismatch)
        );

        let mut bad_platform = valid_draft();
        bad_platform.platform_family = NativePlatformFamily::AndroidKeystore;
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, bad_platform),
            Err(NativeVaultHeaderReviewError::PlatformFamilyMismatch)
        );

        let mut bad_surface = valid_draft();
        bad_surface.secure_surface_contract_domain =
            "native-passport/secure-surface-compartment/v0";
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, bad_surface),
            Err(NativeVaultHeaderReviewError::SecureSurfaceContractDomainMismatch)
        );

        let mut missing = valid_draft();
        missing.requested_compartment_headers.clear();
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, missing),
            Err(NativeVaultHeaderReviewError::MissingCompartmentHeaders)
        );

        let mut duplicate = valid_draft();
        duplicate
            .requested_compartment_headers
            .push(PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS[0]);
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, duplicate),
            Err(NativeVaultHeaderReviewError::DuplicateCompartmentHeader)
        );

        let mut missing_required = valid_draft();
        missing_required.requested_compartment_headers =
            vec![PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS[0]];
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, missing_required),
            Err(NativeVaultHeaderReviewError::MissingRequiredCompartmentHeader)
        );
    }

    #[test]
    fn phase6a_rejects_label_algorithm_length_and_authenticated_header_drift() {
        let secure_surface = secure_surface();

        let mut bad_label = valid_draft();
        bad_label.requested_compartment_headers[0].compartment_label = "wrong_root_compartment";
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, bad_label),
            Err(NativeVaultHeaderReviewError::CompartmentLabelMismatch)
        );

        let mut bad_algorithm = valid_draft();
        bad_algorithm.requested_compartment_headers[0].kdf_algorithm_label = "pbkdf2";
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, bad_algorithm),
            Err(NativeVaultHeaderReviewError::AlgorithmLabelMismatch)
        );

        let mut bad_length = valid_draft();
        bad_length.requested_compartment_headers[0].aead_nonce_len = 12;
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, bad_length),
            Err(NativeVaultHeaderReviewError::HeaderLengthMismatch)
        );

        let mut bad_auth_header = valid_draft();
        bad_auth_header.requested_compartment_headers[0].authenticated_header_label =
            "unauthenticated-header";
        assert_eq!(
            review_native_two_compartment_vault_header_draft(&secure_surface, bad_auth_header),
            Err(NativeVaultHeaderReviewError::AuthenticatedHeaderLabelMismatch)
        );
    }

    #[test]
    fn phase6a_rejects_pin_vault_sealer_storage_crypto_io_and_wallet_ledger_flags() {
        let secure_surface = secure_surface();

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
                draft.stores_secret_material = true
            },
            |draft: &mut NativeTwoCompartmentVaultHeaderDraftV1| draft.exports_material = true,
            |draft: &mut NativeTwoCompartmentVaultHeaderDraftV1| {
                draft.requests_encryption_or_decryption = true
            },
            |draft: &mut NativeTwoCompartmentVaultHeaderDraftV1| draft.requests_runtime_io = true,
            |draft: &mut NativeTwoCompartmentVaultHeaderDraftV1| {
                draft.requests_wallet_or_ledger_mutation = true
            },
        ] {
            let mut draft = valid_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_two_compartment_vault_header_draft(&secure_surface, draft),
                Err(NativeVaultHeaderReviewError::UnsafeVaultHeaderAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase6a_source_remains_header_only_without_pin_sealer_vault_crypto_io_routes_or_mutation() {
        let source = fs::read_to_string(repo_file("src/native/vault.rs")).expect("vault source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeTwoCompartmentVaultHeaderDescriptorV1"));
        assert!(source.contains("NativeTwoCompartmentVaultHeaderDraftV1"));
        assert!(source.contains("review_native_two_compartment_vault_header_draft"));
        assert!(native_mod.contains("TwoCompartmentVaultHeaderDto"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "unlock_pin(",
            "derive_pin(",
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
                "Phase 6A vault header source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
