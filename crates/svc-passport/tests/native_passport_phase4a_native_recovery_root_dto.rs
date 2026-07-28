#[cfg(not(feature = "native-passport"))]
#[test]
fn phase4a_native_recovery_root_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native Passport recovery-root DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_recovery_root_posture, review_native_recovery_root_draft,
        validate_native_recovery_root_descriptor, Ed25519PublicKeyHex, HandleV1,
        NativePassportSurface, NativeRecoveryRootDescriptorV1, NativeRecoveryRootDraftV1,
        NativeRecoveryRootReviewError, NativeRecoveryRootScope, PassportIdV1,
        NATIVE_PASSPORT_PHASE4A_LABEL, PHASE3B_ENABLED_SURFACES,
        PHASE4A_ALLOWED_RECOVERY_ROOT_SCOPES, PHASE4A_ENABLED_SURFACES,
        PHASE4A_FORBIDDEN_RECOVERY_ROOT_AUTHORITY_FLAGS, PHASE4A_RECOVERY_ROOT_DOMAIN,
    };

    const PASSPORT_ID: &str = "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
    const RECOVERY_ROOT_PUBLIC_KEY: &str =
        "3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909";

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn valid_draft() -> NativeRecoveryRootDraftV1 {
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

    #[test]
    fn phase4a_label_and_posture_are_locked() {
        let posture = native_passport_recovery_root_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE4A_LABEL,
            "NATIVE_PASSPORT_PHASE4A_NATIVE_RECOVERY_ROOT_DTO"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE4A_LABEL);
        assert!(posture.recovery_root_dtos_added);
        assert!(!posture.recovery_root_generation_added);
        assert!(!posture.recovery_secret_storage_added);
        assert!(!posture.platform_sealer_added);
        assert!(!posture.signing_runtime_added);
        assert!(!posture.signature_verification_runtime_added);
        assert!(!posture.vault_runtime_added);
        assert!(!posture.capability_issuance_added);
        assert!(!posture.runtime_authority_changed);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "generate_recovery_root",
            "store_recovery_secret",
            "export_recovery_material",
            "unlock_vault",
            "platform_sealer",
            "signing_runtime",
            "signature_verification_runtime",
            "capability_issuance",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE4A_FORBIDDEN_RECOVERY_ROOT_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase4a_surfaces_extend_phase3b_without_back_mutating_it() {
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
    }

    #[test]
    fn phase4a_reviews_public_recovery_root_draft_into_descriptor() {
        let descriptor = review_native_recovery_root_draft(valid_draft())
            .expect("valid public recovery-root draft should review");

        assert_eq!(descriptor.passport_id.as_str(), PASSPORT_ID);
        assert_eq!(
            descriptor.recovery_root_public_key.as_str(),
            RECOVERY_ROOT_PUBLIC_KEY
        );
        assert_eq!(
            descriptor
                .optional_handle
                .as_ref()
                .expect("optional handle")
                .as_str(),
            "@creator_007"
        );
        assert_eq!(descriptor.recovery_domain, PHASE4A_RECOVERY_ROOT_DOMAIN);
        assert_eq!(
            descriptor.allowed_scopes,
            PHASE4A_ALLOWED_RECOVERY_ROOT_SCOPES
        );

        validate_native_recovery_root_descriptor(&descriptor)
            .expect("reviewed descriptor should validate");
    }

    #[test]
    fn phase4a_rejects_domain_empty_scope_and_duplicate_scope_boundaries() {
        let mut bad_domain = valid_draft();
        bad_domain.recovery_domain = "native-passport/recovery-root/v0";
        assert_eq!(
            review_native_recovery_root_draft(bad_domain),
            Err(NativeRecoveryRootReviewError::RecoveryDomainMismatch)
        );

        let mut missing_scopes = valid_draft();
        missing_scopes.requested_scopes.clear();
        assert_eq!(
            review_native_recovery_root_draft(missing_scopes),
            Err(NativeRecoveryRootReviewError::MissingScopes)
        );

        let mut duplicate_scope = valid_draft();
        duplicate_scope
            .requested_scopes
            .push(NativeRecoveryRootScope::RecoverPassport);
        assert_eq!(
            review_native_recovery_root_draft(duplicate_scope),
            Err(NativeRecoveryRootReviewError::DuplicateScope)
        );

        let bad_descriptor = NativeRecoveryRootDescriptorV1 {
            passport_id: PassportIdV1::parse(PASSPORT_ID).unwrap(),
            recovery_root_public_key: Ed25519PublicKeyHex::parse(RECOVERY_ROOT_PUBLIC_KEY).unwrap(),
            optional_handle: None,
            recovery_domain: "native-passport/recovery-root/v0",
            allowed_scopes: PHASE4A_ALLOWED_RECOVERY_ROOT_SCOPES.to_vec(),
        };
        assert_eq!(
            validate_native_recovery_root_descriptor(&bad_descriptor),
            Err(NativeRecoveryRootReviewError::RecoveryDomainMismatch)
        );
    }

    #[test]
    fn phase4a_rejects_secret_storage_export_vault_and_wallet_ledger_flags() {
        for mutate in [
            |draft: &mut NativeRecoveryRootDraftV1| draft.contains_recovery_material = true,
            |draft: &mut NativeRecoveryRootDraftV1| draft.exports_recovery_material = true,
            |draft: &mut NativeRecoveryRootDraftV1| draft.requests_vault_unlock = true,
            |draft: &mut NativeRecoveryRootDraftV1| draft.requests_wallet_or_ledger_mutation = true,
        ] {
            let mut draft = valid_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_recovery_root_draft(draft),
                Err(NativeRecoveryRootReviewError::UnsafeRecoveryAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase4a_source_remains_dto_only_without_generation_sealer_vault_crypto_routes_or_mutation() {
        let recovery_source =
            fs::read_to_string(repo_file("src/native/recovery.rs")).expect("recovery source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native mod source");

        assert!(recovery_source.contains("NativeRecoveryRootDescriptorV1"));
        assert!(recovery_source.contains("NativeRecoveryRootDraftV1"));
        assert!(recovery_source.contains("review_native_recovery_root_draft"));
        assert!(native_mod.contains("NativeRecoveryRootDto"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "generate_recovery_root(",
            "store_recovery_secret(",
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
                "Phase 4A recovery DTO source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
