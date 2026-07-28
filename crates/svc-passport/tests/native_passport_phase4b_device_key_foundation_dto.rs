#[cfg(not(feature = "native-passport"))]
#[test]
fn phase4b_device_key_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native Passport device-key DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_device_key_posture, review_native_device_key_draft,
        review_native_recovery_root_draft, validate_native_device_key_descriptor, DeviceClass,
        DeviceIdV1, Ed25519PublicKeyHex, HandleV1, NativeDeviceKeyDescriptorV1,
        NativeDeviceKeyDraftV1, NativeDeviceKeyPurpose, NativeDeviceKeyReviewError,
        NativePassportSurface, NativeRecoveryRootDescriptorV1, NativeRecoveryRootDraftV1,
        NativeRecoveryRootScope, PassportIdV1, NATIVE_PASSPORT_PHASE4B_LABEL,
        PHASE4A_ENABLED_SURFACES, PHASE4A_RECOVERY_ROOT_DOMAIN,
        PHASE4B_ALLOWED_DEVICE_KEY_PURPOSES, PHASE4B_DEVICE_KEY_DOMAIN, PHASE4B_ENABLED_SURFACES,
        PHASE4B_FORBIDDEN_DEVICE_KEY_AUTHORITY_FLAGS,
    };

    const PASSPORT_ID: &str = "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
    const OTHER_PASSPORT_ID: &str = "passport:v1:main:ed25519:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DEVICE_ID: &str =
        "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";
    const RECOVERY_ROOT_PUBLIC_KEY: &str =
        "3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909";
    const DEVICE_PUBLIC_KEY: &str =
        "2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn recovery_root() -> NativeRecoveryRootDescriptorV1 {
        review_native_recovery_root_draft(NativeRecoveryRootDraftV1 {
            passport_id: PassportIdV1::parse(PASSPORT_ID).unwrap(),
            recovery_root_public_key: Ed25519PublicKeyHex::parse(RECOVERY_ROOT_PUBLIC_KEY).unwrap(),
            optional_handle: Some(HandleV1::parse("@creator_007").unwrap()),
            recovery_domain: PHASE4A_RECOVERY_ROOT_DOMAIN,
            requested_scopes: vec![
                NativeRecoveryRootScope::RecoverPassport,
                NativeRecoveryRootScope::EnrollDevice,
                NativeRecoveryRootScope::RevokeDevice,
                NativeRecoveryRootScope::RotateRoot,
            ],
            contains_recovery_material: false,
            exports_recovery_material: false,
            requests_vault_unlock: false,
            requests_wallet_or_ledger_mutation: false,
        })
        .expect("valid recovery root descriptor")
    }

    fn valid_draft() -> NativeDeviceKeyDraftV1 {
        NativeDeviceKeyDraftV1 {
            passport_id: PassportIdV1::parse(PASSPORT_ID).unwrap(),
            device_id: DeviceIdV1::parse(DEVICE_ID).unwrap(),
            device_public_key: Ed25519PublicKeyHex::parse(DEVICE_PUBLIC_KEY).unwrap(),
            device_class: DeviceClass::TvReadOnly,
            device_key_domain: PHASE4B_DEVICE_KEY_DOMAIN,
            requested_purposes: PHASE4B_ALLOWED_DEVICE_KEY_PURPOSES.to_vec(),
            requests_device_key_generation: false,
            contains_device_secret_material: false,
            exports_device_secret_material: false,
            requests_signing_runtime: false,
            requests_vault_unlock: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase4b_label_and_posture_are_locked() {
        let posture = native_passport_device_key_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE4B_LABEL,
            "NATIVE_PASSPORT_PHASE4B_DEVICE_KEY_FOUNDATION_DTO"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE4B_LABEL);
        assert!(posture.device_key_dtos_added);
        assert!(!posture.device_key_generation_added);
        assert!(!posture.device_secret_storage_added);
        assert!(!posture.platform_sealer_added);
        assert!(!posture.signing_runtime_added);
        assert!(!posture.signature_verification_runtime_added);
        assert!(!posture.vault_runtime_added);
        assert!(!posture.capability_issuance_added);
        assert!(!posture.runtime_authority_changed);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "generate_device_key",
            "store_device_secret",
            "export_device_secret",
            "platform_sealer",
            "unlock_vault",
            "signing_runtime",
            "signature_verification_runtime",
            "capability_issuance",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE4B_FORBIDDEN_DEVICE_KEY_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase4b_surfaces_extend_phase4a_without_back_mutating_it() {
        assert_eq!(
            PHASE4A_ENABLED_SURFACES,
            &[
                NativePassportSurface::FeaturePosture,
                NativePassportSurface::Phase0Contracts,
                NativePassportSurface::NativeDtoTypes,
                NativePassportSurface::NativeStatusRedaction,
                NativePassportSurface::NativeUsernameHandles,
                NativePassportSurface::RootDeviceAuthorizationDtos,
                NativePassportSurface::PureAuthorizationReview,
                NativePassportSurface::NativeRecoveryRootDto,
            ]
        );

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
    }

    #[test]
    fn phase4b_reviews_public_device_key_draft_into_descriptor() {
        let recovery_root = recovery_root();
        let descriptor = review_native_device_key_draft(&recovery_root, valid_draft())
            .expect("valid public device-key draft should review");

        assert_eq!(descriptor.passport_id.as_str(), PASSPORT_ID);
        assert_eq!(descriptor.device_id.as_str(), DEVICE_ID);
        assert_eq!(descriptor.device_public_key.as_str(), DEVICE_PUBLIC_KEY);
        assert_eq!(descriptor.device_class, DeviceClass::TvReadOnly);
        assert_eq!(descriptor.device_key_domain, PHASE4B_DEVICE_KEY_DOMAIN);
        assert_eq!(
            descriptor.allowed_purposes,
            PHASE4B_ALLOWED_DEVICE_KEY_PURPOSES
        );

        validate_native_device_key_descriptor(&recovery_root, &descriptor)
            .expect("reviewed descriptor should validate");
    }

    #[test]
    fn phase4b_rejects_passport_domain_empty_duplicate_and_key_collision_boundaries() {
        let recovery_root = recovery_root();

        let mut passport_mismatch = valid_draft();
        passport_mismatch.passport_id = PassportIdV1::parse(OTHER_PASSPORT_ID).unwrap();
        assert_eq!(
            review_native_device_key_draft(&recovery_root, passport_mismatch),
            Err(NativeDeviceKeyReviewError::PassportMismatch)
        );

        let mut bad_domain = valid_draft();
        bad_domain.device_key_domain = "native-passport/device-key/v0";
        assert_eq!(
            review_native_device_key_draft(&recovery_root, bad_domain),
            Err(NativeDeviceKeyReviewError::DeviceKeyDomainMismatch)
        );

        let mut missing_purposes = valid_draft();
        missing_purposes.requested_purposes.clear();
        assert_eq!(
            review_native_device_key_draft(&recovery_root, missing_purposes),
            Err(NativeDeviceKeyReviewError::MissingPurposes)
        );

        let mut duplicate_purpose = valid_draft();
        duplicate_purpose
            .requested_purposes
            .push(NativeDeviceKeyPurpose::AuthenticateDevice);
        assert_eq!(
            review_native_device_key_draft(&recovery_root, duplicate_purpose),
            Err(NativeDeviceKeyReviewError::DuplicatePurpose)
        );

        let mut key_collision = valid_draft();
        key_collision.device_public_key =
            Ed25519PublicKeyHex::parse(recovery_root.recovery_root_public_key.as_str()).unwrap();
        assert_eq!(
            review_native_device_key_draft(&recovery_root, key_collision),
            Err(NativeDeviceKeyReviewError::RecoveryRootDeviceKeyCollision)
        );
    }

    #[test]
    fn phase4b_descriptor_validator_rejects_mismatch_domain_duplicate_and_key_collision() {
        let recovery_root = recovery_root();

        let mut bad_passport = review_native_device_key_draft(&recovery_root, valid_draft())
            .expect("valid descriptor");
        bad_passport.passport_id = PassportIdV1::parse(OTHER_PASSPORT_ID).unwrap();
        assert_eq!(
            validate_native_device_key_descriptor(&recovery_root, &bad_passport),
            Err(NativeDeviceKeyReviewError::PassportMismatch)
        );

        let mut bad_domain = review_native_device_key_draft(&recovery_root, valid_draft())
            .expect("valid descriptor");
        bad_domain.device_key_domain = "native-passport/device-key/v0";
        assert_eq!(
            validate_native_device_key_descriptor(&recovery_root, &bad_domain),
            Err(NativeDeviceKeyReviewError::DeviceKeyDomainMismatch)
        );

        let mut duplicate = review_native_device_key_draft(&recovery_root, valid_draft())
            .expect("valid descriptor");
        duplicate
            .allowed_purposes
            .push(NativeDeviceKeyPurpose::AuthenticateDevice);
        assert_eq!(
            validate_native_device_key_descriptor(&recovery_root, &duplicate),
            Err(NativeDeviceKeyReviewError::DuplicatePurpose)
        );

        let collision = NativeDeviceKeyDescriptorV1 {
            passport_id: PassportIdV1::parse(PASSPORT_ID).unwrap(),
            device_id: DeviceIdV1::parse(DEVICE_ID).unwrap(),
            device_public_key: Ed25519PublicKeyHex::parse(RECOVERY_ROOT_PUBLIC_KEY).unwrap(),
            device_class: DeviceClass::TvReadOnly,
            device_key_domain: PHASE4B_DEVICE_KEY_DOMAIN,
            allowed_purposes: PHASE4B_ALLOWED_DEVICE_KEY_PURPOSES.to_vec(),
        };
        assert_eq!(
            validate_native_device_key_descriptor(&recovery_root, &collision),
            Err(NativeDeviceKeyReviewError::RecoveryRootDeviceKeyCollision)
        );
    }

    #[test]
    fn phase4b_rejects_generation_secret_export_signing_vault_and_wallet_ledger_flags() {
        let recovery_root = recovery_root();

        for mutate in [
            |draft: &mut NativeDeviceKeyDraftV1| draft.requests_device_key_generation = true,
            |draft: &mut NativeDeviceKeyDraftV1| draft.contains_device_secret_material = true,
            |draft: &mut NativeDeviceKeyDraftV1| draft.exports_device_secret_material = true,
            |draft: &mut NativeDeviceKeyDraftV1| draft.requests_signing_runtime = true,
            |draft: &mut NativeDeviceKeyDraftV1| draft.requests_vault_unlock = true,
            |draft: &mut NativeDeviceKeyDraftV1| draft.requests_wallet_or_ledger_mutation = true,
        ] {
            let mut draft = valid_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_device_key_draft(&recovery_root, draft),
                Err(NativeDeviceKeyReviewError::UnsafeDeviceKeyAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase4b_source_remains_dto_only_without_generation_sealer_vault_crypto_routes_or_mutation() {
        let device_key_source =
            fs::read_to_string(repo_file("src/native/device_key.rs")).expect("device key source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native mod source");

        assert!(device_key_source.contains("NativeDeviceKeyDescriptorV1"));
        assert!(device_key_source.contains("NativeDeviceKeyDraftV1"));
        assert!(device_key_source.contains("review_native_device_key_draft"));
        assert!(native_mod.contains("NativeDeviceKeyDto"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "generate_device_key(",
            "store_device_secret(",
            "platform_seal(",
            "platform_unseal(",
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
                !device_key_source.contains(forbidden_runtime_pattern),
                "Phase 4B device-key DTO source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
