#[cfg(not(feature = "native-passport"))]
#[test]
fn phase3b_pure_authorization_review_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native Passport pure authorization review"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        review_device_authorization_draft, validate_device_authorization_grant,
        DeviceAuthorizationGrantV1, DeviceAuthorizationReviewError, DeviceClass, DeviceIdV1,
        Ed25519PublicKeyHex, HandleV1, NativeDeviceAuthorizationDraftV1, NativePassportScope,
        NativePassportSurface, PassportIdV1, RootPassportDescriptorV1,
        NATIVE_PASSPORT_PHASE3B_LABEL, PHASE3A_ALLOWED_DEVICE_SCOPES, PHASE3A_ENABLED_SURFACES,
        PHASE3B_ENABLED_SURFACES,
    };

    const PASSPORT_ID: &str = "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
    const OTHER_PASSPORT_ID: &str = "passport:v1:main:ed25519:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DEVICE_ID: &str =
        "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";
    const ROOT_PUBLIC_KEY: &str =
        "3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909";
    const OTHER_ROOT_PUBLIC_KEY: &str =
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DEVICE_PUBLIC_KEY: &str =
        "2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn root_descriptor() -> RootPassportDescriptorV1 {
        RootPassportDescriptorV1 {
            passport_id: PassportIdV1::parse(PASSPORT_ID).unwrap(),
            root_public_key: Ed25519PublicKeyHex::parse(ROOT_PUBLIC_KEY).unwrap(),
            optional_handle: Some(HandleV1::parse("@creator_007").unwrap()),
        }
    }

    fn valid_draft() -> NativeDeviceAuthorizationDraftV1 {
        NativeDeviceAuthorizationDraftV1 {
            passport_id: PassportIdV1::parse(PASSPORT_ID).unwrap(),
            root_public_key: Ed25519PublicKeyHex::parse(ROOT_PUBLIC_KEY).unwrap(),
            device_id: DeviceIdV1::parse(DEVICE_ID).unwrap(),
            device_public_key: Ed25519PublicKeyHex::parse(DEVICE_PUBLIC_KEY).unwrap(),
            device_class: DeviceClass::TvReadOnly,
            requested_scopes: PHASE3A_ALLOWED_DEVICE_SCOPES.to_vec(),
        }
    }

    #[test]
    fn phase3b_label_and_surface_extension_are_locked() {
        assert_eq!(
            NATIVE_PASSPORT_PHASE3B_LABEL,
            "NATIVE_PASSPORT_PHASE3B_PURE_AUTHORIZATION_REVIEW"
        );

        assert_eq!(
            PHASE3A_ENABLED_SURFACES,
            &[
                NativePassportSurface::FeaturePosture,
                NativePassportSurface::Phase0Contracts,
                NativePassportSurface::NativeDtoTypes,
                NativePassportSurface::NativeStatusRedaction,
                NativePassportSurface::NativeUsernameHandles,
                NativePassportSurface::RootDeviceAuthorizationDtos,
            ]
        );

        assert_eq!(
            PHASE3B_ENABLED_SURFACES,
            &[
                NativePassportSurface::FeaturePosture,
                NativePassportSurface::Phase0Contracts,
                NativePassportSurface::NativeDtoTypes,
                NativePassportSurface::NativeStatusRedaction,
                NativePassportSurface::NativeUsernameHandles,
                NativePassportSurface::RootDeviceAuthorizationDtos,
                NativePassportSurface::PureAuthorizationReview,
            ]
        );
    }

    #[test]
    fn phase3b_valid_draft_reviews_into_safe_device_grant() {
        let root = root_descriptor();
        let grant = review_device_authorization_draft(&root, valid_draft())
            .expect("valid draft should review into grant");

        assert_eq!(grant.root, root);
        assert_eq!(grant.device_id.as_str(), DEVICE_ID);
        assert_eq!(grant.device_public_key.as_str(), DEVICE_PUBLIC_KEY);
        assert_eq!(grant.device_class, DeviceClass::TvReadOnly);
        assert_eq!(grant.allowed_scopes, PHASE3A_ALLOWED_DEVICE_SCOPES);
        assert!(grant.can_sign_request_proofs);
        assert!(!grant.can_unlock_root);
        assert!(!grant.can_authorize_devices);
        assert!(!grant.can_issue_capabilities);
        assert!(!grant.can_mutate_wallet_or_ledger);

        validate_device_authorization_grant(&grant).expect("reviewed grant should validate");
    }

    #[test]
    fn phase3b_rejects_passport_and_root_key_mismatches() {
        let root = root_descriptor();

        let mut passport_mismatch = valid_draft();
        passport_mismatch.passport_id = PassportIdV1::parse(OTHER_PASSPORT_ID).unwrap();
        assert_eq!(
            review_device_authorization_draft(&root, passport_mismatch),
            Err(DeviceAuthorizationReviewError::PassportMismatch)
        );

        let mut root_key_mismatch = valid_draft();
        root_key_mismatch.root_public_key =
            Ed25519PublicKeyHex::parse(OTHER_ROOT_PUBLIC_KEY).unwrap();
        assert_eq!(
            review_device_authorization_draft(&root, root_key_mismatch),
            Err(DeviceAuthorizationReviewError::RootPublicKeyMismatch)
        );
    }

    #[test]
    fn phase3b_rejects_missing_duplicate_and_collapsed_device_authorization() {
        let root = root_descriptor();

        let mut missing_scopes = valid_draft();
        missing_scopes.requested_scopes.clear();
        assert_eq!(
            review_device_authorization_draft(&root, missing_scopes),
            Err(DeviceAuthorizationReviewError::MissingScopes)
        );

        let mut duplicate_scope = valid_draft();
        duplicate_scope
            .requested_scopes
            .push(NativePassportScope::IdentityRead);
        assert_eq!(
            review_device_authorization_draft(&root, duplicate_scope),
            Err(DeviceAuthorizationReviewError::DuplicateScope)
        );

        let mut collapsed_key = valid_draft();
        collapsed_key.device_public_key = collapsed_key.root_public_key.clone();
        assert_eq!(
            review_device_authorization_draft(&root, collapsed_key),
            Err(DeviceAuthorizationReviewError::RootDeviceKeyCollision)
        );
    }

    #[test]
    fn phase3b_review_never_grants_unsafe_authority_flags() {
        let root = root_descriptor();
        let grant = review_device_authorization_draft(&root, valid_draft()).unwrap();

        assert!(!grant.can_unlock_root);
        assert!(!grant.can_authorize_devices);
        assert!(!grant.can_issue_capabilities);
        assert!(!grant.can_mutate_wallet_or_ledger);

        let mut unsafe_grant: DeviceAuthorizationGrantV1 = grant.clone();
        unsafe_grant.can_issue_capabilities = true;
        assert_eq!(
            validate_device_authorization_grant(&unsafe_grant),
            Err(DeviceAuthorizationReviewError::UnsafeAuthorityFlag)
        );
    }

    #[test]
    fn phase3b_source_stays_pure_review_without_crypto_vault_routes_or_mutation() {
        let source = fs::read_to_string(repo_file("src/native/authorization.rs"))
            .expect("authorization source should be readable");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native mod should read");

        assert!(source.contains("review_device_authorization_draft"));
        assert!(source.contains("PassportMismatch"));
        assert!(source.contains("RootPublicKeyMismatch"));
        assert!(native_mod.contains("PureAuthorizationReview"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
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
                "pure authorization review source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
