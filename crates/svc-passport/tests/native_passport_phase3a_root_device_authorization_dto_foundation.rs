#[cfg(not(feature = "native-passport"))]
#[test]
fn phase3a_root_device_authorization_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native Passport root/device authorization DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_authorization_posture, validate_device_authorization_grant,
        DeviceAuthorizationGrantV1, DeviceAuthorizationReviewError, DeviceClass, DeviceIdV1,
        Ed25519PublicKeyHex, HandleV1, NativePassportScope, NativePassportSurface, PassportIdV1,
        RootPassportDescriptorV1, NATIVE_PASSPORT_PHASE3A_LABEL, PHASE2B_ENABLED_SURFACES,
        PHASE3A_ALLOWED_DEVICE_SCOPES, PHASE3A_ENABLED_SURFACES, PHASE3A_FORBIDDEN_AUTHORITY_FLAGS,
    };

    const PASSPORT_ID: &str = "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
    const DEVICE_ID: &str =
        "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";
    const ROOT_PUBLIC_KEY: &str =
        "3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909";
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

    fn valid_grant() -> DeviceAuthorizationGrantV1 {
        DeviceAuthorizationGrantV1 {
            root: root_descriptor(),
            device_id: DeviceIdV1::parse(DEVICE_ID).unwrap(),
            device_public_key: Ed25519PublicKeyHex::parse(DEVICE_PUBLIC_KEY).unwrap(),
            device_class: DeviceClass::TvReadOnly,
            allowed_scopes: PHASE3A_ALLOWED_DEVICE_SCOPES.to_vec(),
            can_sign_request_proofs: true,
            can_unlock_root: false,
            can_authorize_devices: false,
            can_issue_capabilities: false,
            can_mutate_wallet_or_ledger: false,
        }
    }

    #[test]
    fn phase3a_label_and_posture_are_locked() {
        let posture = native_passport_authorization_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE3A_LABEL,
            "NATIVE_PASSPORT_PHASE3A_ROOT_DEVICE_AUTHORIZATION_DTO_FOUNDATION"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE3A_LABEL);
        assert!(posture.root_device_authorization_dtos_added);
        assert!(!posture.signature_verification_runtime_added);
        assert!(!posture.signing_runtime_added);
        assert!(!posture.vault_runtime_added);
        assert!(!posture.capability_issuance_added);
        assert!(!posture.runtime_authority_changed);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "root_unlock",
            "device_issuance",
            "capability_issuance",
            "wallet_spend",
            "ledger_mutation",
            "vault_decrypt",
            "native_secret_custody",
            "signing_runtime",
            "signature_verification_runtime",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE3A_FORBIDDEN_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase3a_surfaces_extend_phase2_without_back_mutating_phase2b() {
        assert_eq!(
            PHASE2B_ENABLED_SURFACES,
            &[
                NativePassportSurface::FeaturePosture,
                NativePassportSurface::Phase0Contracts,
                NativePassportSurface::NativeDtoTypes,
                NativePassportSurface::NativeStatusRedaction,
                NativePassportSurface::NativeUsernameHandles,
            ]
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
    }

    #[test]
    fn phase3a_root_descriptor_keeps_root_public_identity_separate_from_handle() {
        let root = root_descriptor();

        assert_eq!(root.passport_id.as_str(), PASSPORT_ID);
        assert_eq!(root.root_public_key.as_str(), ROOT_PUBLIC_KEY);
        assert_eq!(
            root.optional_handle.expect("optional handle").as_str(),
            "@creator_007"
        );
    }

    #[test]
    fn phase3a_validates_read_only_device_authorization_grant_shape() {
        let grant = valid_grant();

        assert_eq!(grant.device_id.as_str(), DEVICE_ID);
        assert_eq!(grant.device_public_key.as_str(), DEVICE_PUBLIC_KEY);
        assert_eq!(grant.device_class, DeviceClass::TvReadOnly);
        assert_eq!(grant.allowed_scopes, PHASE3A_ALLOWED_DEVICE_SCOPES);
        assert!(grant.can_sign_request_proofs);

        validate_device_authorization_grant(&grant).expect("valid grant should pass");
    }

    #[test]
    fn phase3a_rejects_missing_duplicate_and_collapsed_authorization_shapes() {
        let mut missing_scopes = valid_grant();
        missing_scopes.allowed_scopes.clear();
        assert_eq!(
            validate_device_authorization_grant(&missing_scopes),
            Err(DeviceAuthorizationReviewError::MissingScopes)
        );

        let mut duplicate_scope = valid_grant();
        duplicate_scope
            .allowed_scopes
            .push(NativePassportScope::IdentityRead);
        assert_eq!(
            validate_device_authorization_grant(&duplicate_scope),
            Err(DeviceAuthorizationReviewError::DuplicateScope)
        );

        let mut collapsed_key = valid_grant();
        collapsed_key.device_public_key = collapsed_key.root.root_public_key.clone();
        assert_eq!(
            validate_device_authorization_grant(&collapsed_key),
            Err(DeviceAuthorizationReviewError::RootDeviceKeyCollision)
        );
    }

    #[test]
    fn phase3a_rejects_authority_flags_without_running_crypto_or_vault_logic() {
        let mut root_unlock = valid_grant();
        root_unlock.can_unlock_root = true;
        assert_eq!(
            validate_device_authorization_grant(&root_unlock),
            Err(DeviceAuthorizationReviewError::UnsafeAuthorityFlag)
        );

        let mut device_issuance = valid_grant();
        device_issuance.can_authorize_devices = true;
        assert_eq!(
            validate_device_authorization_grant(&device_issuance),
            Err(DeviceAuthorizationReviewError::UnsafeAuthorityFlag)
        );

        let mut capability_issuance = valid_grant();
        capability_issuance.can_issue_capabilities = true;
        assert_eq!(
            validate_device_authorization_grant(&capability_issuance),
            Err(DeviceAuthorizationReviewError::UnsafeAuthorityFlag)
        );

        let mut wallet_or_ledger = valid_grant();
        wallet_or_ledger.can_mutate_wallet_or_ledger = true;
        assert_eq!(
            validate_device_authorization_grant(&wallet_or_ledger),
            Err(DeviceAuthorizationReviewError::UnsafeAuthorityFlag)
        );
    }

    #[test]
    fn phase3a_source_does_not_add_routes_signing_vault_capability_or_wallet_ledger_runtime() {
        let source = fs::read_to_string(repo_file("src/native/authorization.rs"))
            .expect("authorization source should be readable");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native mod should read");

        assert!(source.contains("DeviceAuthorizationGrantV1"));
        assert!(source.contains("RootPassportDescriptorV1"));
        assert!(native_mod.contains("RootDeviceAuthorizationDtos"));

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
                "authorization DTO source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
