#[cfg(not(feature = "native-passport"))]
#[test]
fn phase4c_native_root_device_acceptance_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native Passport Phase 4 acceptance"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_device_key_posture, native_passport_recovery_root_posture,
        review_native_device_key_draft, review_native_recovery_root_draft,
        validate_native_device_key_descriptor, validate_native_recovery_root_descriptor,
        DeviceClass, DeviceIdV1, Ed25519PublicKeyHex, HandleV1, NativeDeviceKeyDraftV1,
        NativeDeviceKeyPurpose, NativeDeviceKeyReviewError, NativePassportSurface,
        NativeRecoveryRootDescriptorV1, NativeRecoveryRootDraftV1, NativeRecoveryRootReviewError,
        NativeRecoveryRootScope, PassportIdV1, NATIVE_PASSPORT_PHASE4A_LABEL,
        NATIVE_PASSPORT_PHASE4B_LABEL, PHASE4A_ALLOWED_RECOVERY_ROOT_SCOPES,
        PHASE4A_ENABLED_SURFACES, PHASE4A_RECOVERY_ROOT_DOMAIN,
        PHASE4B_ALLOWED_DEVICE_KEY_PURPOSES, PHASE4B_DEVICE_KEY_DOMAIN, PHASE4B_ENABLED_SURFACES,
    };

    const PHASE4C_LABEL: &str = "NATIVE_PASSPORT_PHASE4C_NATIVE_ROOT_DEVICE_ACCEPTANCE";

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

    fn recovery_root_draft() -> NativeRecoveryRootDraftV1 {
        NativeRecoveryRootDraftV1 {
            passport_id: PassportIdV1::parse(PASSPORT_ID).unwrap(),
            recovery_root_public_key: Ed25519PublicKeyHex::parse(RECOVERY_ROOT_PUBLIC_KEY).unwrap(),
            optional_handle: Some(HandleV1::parse("@creator_007").unwrap()),
            recovery_domain: PHASE4A_RECOVERY_ROOT_DOMAIN,
            requested_scopes: PHASE4A_ALLOWED_RECOVERY_ROOT_SCOPES.to_vec(),
            contains_recovery_material: false,
            exports_recovery_material: false,
            requests_vault_unlock: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    fn recovery_root() -> NativeRecoveryRootDescriptorV1 {
        review_native_recovery_root_draft(recovery_root_draft())
            .expect("valid recovery-root draft should review")
    }

    fn device_key_draft() -> NativeDeviceKeyDraftV1 {
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
    fn phase4c_acceptance_label_and_phase4_predecessor_labels_are_locked() {
        assert_eq!(
            PHASE4C_LABEL,
            "NATIVE_PASSPORT_PHASE4C_NATIVE_ROOT_DEVICE_ACCEPTANCE"
        );
        assert_eq!(
            NATIVE_PASSPORT_PHASE4A_LABEL,
            "NATIVE_PASSPORT_PHASE4A_NATIVE_RECOVERY_ROOT_DTO"
        );
        assert_eq!(
            NATIVE_PASSPORT_PHASE4B_LABEL,
            "NATIVE_PASSPORT_PHASE4B_DEVICE_KEY_FOUNDATION_DTO"
        );
    }

    #[test]
    fn phase4c_accepts_recovery_and_device_postures_without_runtime_authority() {
        let recovery_posture = native_passport_recovery_root_posture();
        let device_key_posture = native_passport_device_key_posture();

        assert!(recovery_posture.recovery_root_dtos_added);
        assert!(!recovery_posture.recovery_root_generation_added);
        assert!(!recovery_posture.recovery_secret_storage_added);
        assert!(!recovery_posture.platform_sealer_added);
        assert!(!recovery_posture.signing_runtime_added);
        assert!(!recovery_posture.signature_verification_runtime_added);
        assert!(!recovery_posture.vault_runtime_added);
        assert!(!recovery_posture.capability_issuance_added);
        assert!(!recovery_posture.runtime_authority_changed);
        assert!(!recovery_posture.native_secret_implementation_added);

        assert!(device_key_posture.device_key_dtos_added);
        assert!(!device_key_posture.device_key_generation_added);
        assert!(!device_key_posture.device_secret_storage_added);
        assert!(!device_key_posture.platform_sealer_added);
        assert!(!device_key_posture.signing_runtime_added);
        assert!(!device_key_posture.signature_verification_runtime_added);
        assert!(!device_key_posture.vault_runtime_added);
        assert!(!device_key_posture.capability_issuance_added);
        assert!(!device_key_posture.runtime_authority_changed);
        assert!(!device_key_posture.native_secret_implementation_added);
    }

    #[test]
    fn phase4c_accepts_phase4_surface_chain_without_back_mutating_phase4a() {
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
    fn phase4c_accepts_public_recovery_root_to_device_key_flow() {
        let recovery_root = recovery_root();
        let device_key = review_native_device_key_draft(&recovery_root, device_key_draft())
            .expect("valid device-key draft should review");

        assert_eq!(recovery_root.passport_id.as_str(), PASSPORT_ID);
        assert_eq!(
            recovery_root.recovery_root_public_key.as_str(),
            RECOVERY_ROOT_PUBLIC_KEY
        );
        assert_eq!(
            recovery_root.allowed_scopes,
            PHASE4A_ALLOWED_RECOVERY_ROOT_SCOPES
        );

        assert_eq!(device_key.passport_id.as_str(), PASSPORT_ID);
        assert_eq!(device_key.device_id.as_str(), DEVICE_ID);
        assert_eq!(device_key.device_public_key.as_str(), DEVICE_PUBLIC_KEY);
        assert_eq!(
            device_key.allowed_purposes,
            PHASE4B_ALLOWED_DEVICE_KEY_PURPOSES
        );

        validate_native_recovery_root_descriptor(&recovery_root)
            .expect("reviewed recovery root descriptor should validate");
        validate_native_device_key_descriptor(&recovery_root, &device_key)
            .expect("reviewed device-key descriptor should validate");
    }

    #[test]
    fn phase4c_accepts_recovery_root_rejection_boundaries() {
        let mut bad_domain = recovery_root_draft();
        bad_domain.recovery_domain = "native-passport/recovery-root/v0";
        assert_eq!(
            review_native_recovery_root_draft(bad_domain),
            Err(NativeRecoveryRootReviewError::RecoveryDomainMismatch)
        );

        let mut missing_scopes = recovery_root_draft();
        missing_scopes.requested_scopes.clear();
        assert_eq!(
            review_native_recovery_root_draft(missing_scopes),
            Err(NativeRecoveryRootReviewError::MissingScopes)
        );

        let mut duplicate_scope = recovery_root_draft();
        duplicate_scope
            .requested_scopes
            .push(NativeRecoveryRootScope::RecoverPassport);
        assert_eq!(
            review_native_recovery_root_draft(duplicate_scope),
            Err(NativeRecoveryRootReviewError::DuplicateScope)
        );

        for mutate in [
            |draft: &mut NativeRecoveryRootDraftV1| draft.contains_recovery_material = true,
            |draft: &mut NativeRecoveryRootDraftV1| draft.exports_recovery_material = true,
            |draft: &mut NativeRecoveryRootDraftV1| draft.requests_vault_unlock = true,
            |draft: &mut NativeRecoveryRootDraftV1| draft.requests_wallet_or_ledger_mutation = true,
        ] {
            let mut draft = recovery_root_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_recovery_root_draft(draft),
                Err(NativeRecoveryRootReviewError::UnsafeRecoveryAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase4c_accepts_device_key_rejection_boundaries() {
        let recovery_root = recovery_root();

        let mut passport_mismatch = device_key_draft();
        passport_mismatch.passport_id = PassportIdV1::parse(OTHER_PASSPORT_ID).unwrap();
        assert_eq!(
            review_native_device_key_draft(&recovery_root, passport_mismatch),
            Err(NativeDeviceKeyReviewError::PassportMismatch)
        );

        let mut bad_domain = device_key_draft();
        bad_domain.device_key_domain = "native-passport/device-key/v0";
        assert_eq!(
            review_native_device_key_draft(&recovery_root, bad_domain),
            Err(NativeDeviceKeyReviewError::DeviceKeyDomainMismatch)
        );

        let mut missing_purposes = device_key_draft();
        missing_purposes.requested_purposes.clear();
        assert_eq!(
            review_native_device_key_draft(&recovery_root, missing_purposes),
            Err(NativeDeviceKeyReviewError::MissingPurposes)
        );

        let mut duplicate_purpose = device_key_draft();
        duplicate_purpose
            .requested_purposes
            .push(NativeDeviceKeyPurpose::AuthenticateDevice);
        assert_eq!(
            review_native_device_key_draft(&recovery_root, duplicate_purpose),
            Err(NativeDeviceKeyReviewError::DuplicatePurpose)
        );

        let mut key_collision = device_key_draft();
        key_collision.device_public_key =
            Ed25519PublicKeyHex::parse(recovery_root.recovery_root_public_key.as_str()).unwrap();
        assert_eq!(
            review_native_device_key_draft(&recovery_root, key_collision),
            Err(NativeDeviceKeyReviewError::RecoveryRootDeviceKeyCollision)
        );

        for mutate in [
            |draft: &mut NativeDeviceKeyDraftV1| draft.requests_device_key_generation = true,
            |draft: &mut NativeDeviceKeyDraftV1| draft.contains_device_secret_material = true,
            |draft: &mut NativeDeviceKeyDraftV1| draft.exports_device_secret_material = true,
            |draft: &mut NativeDeviceKeyDraftV1| draft.requests_signing_runtime = true,
            |draft: &mut NativeDeviceKeyDraftV1| draft.requests_vault_unlock = true,
            |draft: &mut NativeDeviceKeyDraftV1| draft.requests_wallet_or_ledger_mutation = true,
        ] {
            let mut draft = device_key_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_device_key_draft(&recovery_root, draft),
                Err(NativeDeviceKeyReviewError::UnsafeDeviceKeyAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase4c_sources_remain_dto_review_only_without_generation_sealer_crypto_routes_or_mutation()
    {
        let recovery_source =
            fs::read_to_string(repo_file("src/native/recovery.rs")).expect("recovery source");
        let device_key_source =
            fs::read_to_string(repo_file("src/native/device_key.rs")).expect("device-key source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(recovery_source.contains("review_native_recovery_root_draft"));
        assert!(device_key_source.contains("review_native_device_key_draft"));
        assert!(native_mod.contains("NativeRecoveryRootDto"));
        assert!(native_mod.contains("NativeDeviceKeyDto"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "generate_recovery_root(",
            "generate_device_key(",
            "store_recovery_secret(",
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
                !recovery_source.contains(forbidden_runtime_pattern),
                "Phase 4 recovery source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
            assert!(
                !device_key_source.contains(forbidden_runtime_pattern),
                "Phase 4 device-key source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
