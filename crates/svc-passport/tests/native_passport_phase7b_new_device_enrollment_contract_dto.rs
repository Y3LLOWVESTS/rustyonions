#[cfg(not(feature = "native-passport"))]
#[test]
fn phase7b_new_device_enrollment_contract_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native new-device enrollment contract DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_new_device_enrollment_posture,
        review_native_new_device_enrollment_contract_draft,
        validate_native_new_device_enrollment_contract_descriptor, DeviceClass,
        NativeEnrollmentIntent, NativeEnrollmentSource, NativeNewDeviceEnrollmentContractDraftV1,
        NativeNewDeviceEnrollmentContractReviewError, NativePassportSurface, NativePlatformFamily,
        NativeRestoreContractDescriptorV1, NATIVE_PASSPORT_PHASE7B_LABEL,
        PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN, PHASE6A_VAULT_HEADER_VERSION,
        PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN, PHASE6B_PIN_UNLOCK_CONTRACT_VERSION,
        PHASE7A_ALLOWED_RESTORE_SOURCES, PHASE7A_ENABLED_SURFACES,
        PHASE7A_REQUIRED_RESTORE_INTENTS, PHASE7A_RESTORE_CONTRACT_DOMAIN,
        PHASE7A_RESTORE_CONTRACT_VERSION, PHASE7B_ALLOWED_ENROLLMENT_SOURCES,
        PHASE7B_ALLOWED_TARGET_DEVICE_CLASSES, PHASE7B_ENABLED_SURFACES,
        PHASE7B_FORBIDDEN_ENROLLMENT_AUTHORITY_FLAGS,
        PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN,
        PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION, PHASE7B_REQUIRED_ENROLLMENT_INTENTS,
        PHASE7B_REQUIRED_ENROLLMENT_SOURCES,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn restore_contract() -> NativeRestoreContractDescriptorV1 {
        NativeRestoreContractDescriptorV1 {
            contract_domain: PHASE7A_RESTORE_CONTRACT_DOMAIN,
            contract_version: PHASE7A_RESTORE_CONTRACT_VERSION,
            platform_family: NativePlatformFamily::MacosKeychain,
            pin_contract_domain: PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN,
            pin_contract_version: PHASE6B_PIN_UNLOCK_CONTRACT_VERSION,
            vault_header_domain: PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN,
            vault_header_version: PHASE6A_VAULT_HEADER_VERSION,
            restore_intents: PHASE7A_REQUIRED_RESTORE_INTENTS.to_vec(),
            restore_sources: PHASE7A_ALLOWED_RESTORE_SOURCES.to_vec(),
            contract_only: true,
        }
    }

    fn valid_draft() -> NativeNewDeviceEnrollmentContractDraftV1 {
        NativeNewDeviceEnrollmentContractDraftV1 {
            contract_domain: PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN,
            contract_version: PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION,
            platform_family: NativePlatformFamily::MacosKeychain,
            restore_contract_domain: PHASE7A_RESTORE_CONTRACT_DOMAIN,
            restore_contract_version: PHASE7A_RESTORE_CONTRACT_VERSION,
            target_device_class: DeviceClass::DesktopReadOnly,
            requested_enrollment_intents: PHASE7B_REQUIRED_ENROLLMENT_INTENTS.to_vec(),
            requested_enrollment_sources: PHASE7B_ALLOWED_ENROLLMENT_SOURCES.to_vec(),
            requests_enrollment_execution: false,
            requests_device_key_generation: false,
            requests_device_authorization_signature: false,
            requests_signing_or_verification_runtime: false,
            requests_capability_issuance: false,
            requests_restore_execution: false,
            requests_key_derivation_runtime: false,
            requests_pin_validation_runtime: false,
            requests_pin_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            includes_platform_sealer_implementation: false,
            stores_secret_material: false,
            exports_secret_or_material: false,
            requests_encryption_or_decryption: false,
            requests_live_rpc: false,
            requests_runtime_io: false,
            adds_routes: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase7b_label_and_posture_are_locked() {
        let posture = native_passport_new_device_enrollment_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE7B_LABEL,
            "NATIVE_PASSPORT_PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DTO"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE7B_LABEL);
        assert!(posture.new_device_enrollment_contract_dtos_added);

        for authority_added in [
            posture.enrollment_execution_added,
            posture.device_key_generation_added,
            posture.device_authorization_signature_added,
            posture.signing_or_verification_runtime_added,
            posture.capability_issuance_added,
            posture.restore_execution_added,
            posture.key_derivation_runtime_added,
            posture.pin_validation_runtime_added,
            posture.pin_derivation_runtime_added,
            posture.vault_unlock_added,
            posture.vault_runtime_added,
            posture.platform_sealer_implementation_added,
            posture.secret_storage_added,
            posture.material_export_added,
            posture.encryption_runtime_added,
            posture.decryption_runtime_added,
            posture.live_rpc_added,
            posture.runtime_io_added,
            posture.routes_added,
            posture.wallet_or_ledger_mutation_added,
            posture.runtime_authority_changed,
            posture.native_secret_implementation_added,
        ] {
            assert!(!authority_added);
        }

        for forbidden in [
            "enrollment_execution",
            "device_key_generation",
            "device_authorization_signature",
            "signing_runtime",
            "signature_verification_runtime",
            "capability_issuance",
            "restore_execution",
            "root_key_derivation",
            "device_key_derivation",
            "pin_validation_runtime",
            "pin_derivation_runtime",
            "vault_unlock",
            "vault_runtime",
            "platform_sealer_implementation",
            "secret_storage",
            "secret_export",
            "material_export",
            "encryption_runtime",
            "decryption_runtime",
            "live_rpc",
            "runtime_io",
            "route_added",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE7B_FORBIDDEN_ENROLLMENT_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase7b_surfaces_extend_phase7a_without_back_mutating_it() {
        assert_eq!(
            PHASE7B_ENABLED_SURFACES.len(),
            PHASE7A_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE7B_ENABLED_SURFACES[..PHASE7A_ENABLED_SURFACES.len()],
            PHASE7A_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE7B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::NewDeviceEnrollmentContractDto)
        );
        assert_eq!(
            PHASE7A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::RestoreContractDto)
        );
    }

    #[test]
    fn phase7b_reviews_restore_bound_enrollment_draft_into_contract_only_descriptor() {
        let restore_contract = restore_contract();
        let descriptor =
            review_native_new_device_enrollment_contract_draft(&restore_contract, valid_draft())
                .expect("valid enrollment contract should review");

        assert_eq!(
            descriptor.contract_domain,
            PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN
        );
        assert_eq!(
            descriptor.contract_version,
            PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION
        );
        assert_eq!(
            descriptor.platform_family,
            NativePlatformFamily::MacosKeychain
        );
        assert_eq!(
            descriptor.restore_contract_domain,
            PHASE7A_RESTORE_CONTRACT_DOMAIN
        );
        assert_eq!(
            descriptor.restore_contract_version,
            PHASE7A_RESTORE_CONTRACT_VERSION
        );
        assert_eq!(descriptor.target_device_class, DeviceClass::DesktopReadOnly);
        assert_eq!(
            descriptor.enrollment_intents,
            PHASE7B_REQUIRED_ENROLLMENT_INTENTS
        );
        assert_eq!(
            descriptor.enrollment_sources,
            PHASE7B_ALLOWED_ENROLLMENT_SOURCES
        );
        assert!(descriptor.contract_only);

        validate_native_new_device_enrollment_contract_descriptor(&restore_contract, &descriptor)
            .expect("reviewed enrollment contract descriptor should validate");
    }

    #[test]
    fn phase7b_required_intents_sources_and_target_classes_are_locked() {
        assert_eq!(
            PHASE7B_REQUIRED_ENROLLMENT_INTENTS,
            &[
                NativeEnrollmentIntent::RequestDeviceEnrollment,
                NativeEnrollmentIntent::BindDeviceAuthorization,
                NativeEnrollmentIntent::PrepareDeviceCapability,
            ]
        );
        assert_eq!(
            PHASE7B_REQUIRED_ENROLLMENT_SOURCES,
            &[NativeEnrollmentSource::RestoreContract]
        );
        assert_eq!(
            PHASE7B_ALLOWED_ENROLLMENT_SOURCES,
            &[
                NativeEnrollmentSource::RestoreContract,
                NativeEnrollmentSource::LocalRootApproval,
                NativeEnrollmentSource::DelegatedDeviceInvitation,
            ]
        );
        assert_eq!(
            PHASE7B_ALLOWED_TARGET_DEVICE_CLASSES,
            &[
                DeviceClass::TvReadOnly,
                DeviceClass::DesktopReadOnly,
                DeviceClass::MobileReadOnly,
            ]
        );
    }

    #[test]
    fn phase7b_rejects_domain_version_platform_and_restore_linkage_drift() {
        let restore_contract = restore_contract();

        let mut bad_domain = valid_draft();
        bad_domain.contract_domain = "native-passport/new-device-enrollment-contract/v0";
        assert_eq!(
            review_native_new_device_enrollment_contract_draft(&restore_contract, bad_domain),
            Err(NativeNewDeviceEnrollmentContractReviewError::ContractDomainMismatch)
        );

        let mut bad_version = valid_draft();
        bad_version.contract_version = 2;
        assert_eq!(
            review_native_new_device_enrollment_contract_draft(&restore_contract, bad_version),
            Err(NativeNewDeviceEnrollmentContractReviewError::ContractVersionMismatch)
        );

        let mut bad_platform = valid_draft();
        bad_platform.platform_family = NativePlatformFamily::AndroidKeystore;
        assert_eq!(
            review_native_new_device_enrollment_contract_draft(&restore_contract, bad_platform),
            Err(NativeNewDeviceEnrollmentContractReviewError::PlatformFamilyMismatch)
        );

        let mut bad_restore_domain = valid_draft();
        bad_restore_domain.restore_contract_domain = "native-passport/restore-contract/v0";
        assert_eq!(
            review_native_new_device_enrollment_contract_draft(
                &restore_contract,
                bad_restore_domain,
            ),
            Err(NativeNewDeviceEnrollmentContractReviewError::RestoreContractDomainMismatch)
        );

        let mut bad_restore_version = valid_draft();
        bad_restore_version.restore_contract_version = 2;
        assert_eq!(
            review_native_new_device_enrollment_contract_draft(
                &restore_contract,
                bad_restore_version,
            ),
            Err(NativeNewDeviceEnrollmentContractReviewError::RestoreContractVersionMismatch)
        );
    }

    #[test]
    fn phase7b_rejects_empty_duplicate_and_missing_required_intents_and_sources() {
        let restore_contract = restore_contract();

        let mut missing_intents = valid_draft();
        missing_intents.requested_enrollment_intents.clear();
        assert_eq!(
            review_native_new_device_enrollment_contract_draft(&restore_contract, missing_intents,),
            Err(NativeNewDeviceEnrollmentContractReviewError::MissingEnrollmentIntents)
        );

        let mut duplicate_intent = valid_draft();
        duplicate_intent
            .requested_enrollment_intents
            .push(NativeEnrollmentIntent::RequestDeviceEnrollment);
        assert_eq!(
            review_native_new_device_enrollment_contract_draft(&restore_contract, duplicate_intent,),
            Err(NativeNewDeviceEnrollmentContractReviewError::DuplicateEnrollmentIntent)
        );

        let mut missing_required_intent = valid_draft();
        missing_required_intent.requested_enrollment_intents = vec![
            NativeEnrollmentIntent::RequestDeviceEnrollment,
            NativeEnrollmentIntent::BindDeviceAuthorization,
        ];
        assert_eq!(
            review_native_new_device_enrollment_contract_draft(
                &restore_contract,
                missing_required_intent,
            ),
            Err(NativeNewDeviceEnrollmentContractReviewError::MissingRequiredEnrollmentIntent)
        );

        let mut missing_sources = valid_draft();
        missing_sources.requested_enrollment_sources.clear();
        assert_eq!(
            review_native_new_device_enrollment_contract_draft(&restore_contract, missing_sources,),
            Err(NativeNewDeviceEnrollmentContractReviewError::MissingEnrollmentSources)
        );

        let mut duplicate_source = valid_draft();
        duplicate_source
            .requested_enrollment_sources
            .push(NativeEnrollmentSource::RestoreContract);
        assert_eq!(
            review_native_new_device_enrollment_contract_draft(&restore_contract, duplicate_source,),
            Err(NativeNewDeviceEnrollmentContractReviewError::DuplicateEnrollmentSource)
        );

        let mut missing_required_source = valid_draft();
        missing_required_source.requested_enrollment_sources = vec![
            NativeEnrollmentSource::LocalRootApproval,
            NativeEnrollmentSource::DelegatedDeviceInvitation,
        ];
        assert_eq!(
            review_native_new_device_enrollment_contract_draft(
                &restore_contract,
                missing_required_source,
            ),
            Err(NativeNewDeviceEnrollmentContractReviewError::MissingRequiredEnrollmentSource)
        );
    }

    #[test]
    fn phase7b_rejects_non_contract_restore_and_enrollment_descriptors() {
        let mut non_contract_restore = restore_contract();
        non_contract_restore.contract_only = false;
        assert_eq!(
            review_native_new_device_enrollment_contract_draft(
                &non_contract_restore,
                valid_draft(),
            ),
            Err(NativeNewDeviceEnrollmentContractReviewError::RestoreContractNotContractOnly)
        );

        let valid_restore_contract = restore_contract();
        let mut descriptor = review_native_new_device_enrollment_contract_draft(
            &valid_restore_contract,
            valid_draft(),
        )
        .expect("valid descriptor");
        descriptor.contract_only = false;

        assert_eq!(
            validate_native_new_device_enrollment_contract_descriptor(
                &valid_restore_contract,
                &descriptor,
            ),
            Err(NativeNewDeviceEnrollmentContractReviewError::EnrollmentContractNotContractOnly)
        );
    }

    #[test]
    fn phase7b_rejects_enrollment_key_signature_capability_restore_pin_vault_crypto_rpc_routes_and_mutation_flags(
    ) {
        let restore_contract = restore_contract();

        for mutate in [
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.requests_enrollment_execution = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.requests_device_key_generation = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.requests_device_authorization_signature = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.requests_signing_or_verification_runtime = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.requests_capability_issuance = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.requests_restore_execution = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.requests_key_derivation_runtime = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.requests_pin_validation_runtime = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.requests_pin_derivation_runtime = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.requests_vault_unlock = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.includes_vault_runtime = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.includes_platform_sealer_implementation = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.stores_secret_material = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.exports_secret_or_material = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.requests_encryption_or_decryption = true
            },
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| draft.requests_live_rpc = true,
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| draft.requests_runtime_io = true,
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| draft.adds_routes = true,
            |draft: &mut NativeNewDeviceEnrollmentContractDraftV1| {
                draft.requests_wallet_or_ledger_mutation = true
            },
        ] {
            let mut draft = valid_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_new_device_enrollment_contract_draft(&restore_contract, draft),
                Err(NativeNewDeviceEnrollmentContractReviewError::UnsafeEnrollmentAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase7b_source_remains_contract_only_without_enrollment_key_crypto_rpc_routes_or_mutation() {
        let source =
            fs::read_to_string(repo_file("src/native/enrollment.rs")).expect("enrollment source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeNewDeviceEnrollmentContractDescriptorV1"));
        assert!(source.contains("NativeNewDeviceEnrollmentContractDraftV1"));
        assert!(source.contains("review_native_new_device_enrollment_contract_draft"));
        assert!(native_mod.contains("NewDeviceEnrollmentContractDto"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "execute_enrollment(",
            "enroll_device(",
            "generate_device_key(",
            "derive_root_key(",
            "derive_device_key(",
            "authorize_device(",
            "sign(",
            "verify_signature(",
            "verify_proof(",
            "issue_capability(",
            "execute_restore(",
            "restore_passport(",
            "restore_device(",
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
            "rpc.submit(",
            "rpc.send(",
            "runtime_read(",
            "runtime_write(",
            "vault_decrypt(",
            "vault_encrypt(",
            "wallet.spend(",
            "ledger.write(",
            "mint_roc(",
            "burn_roc(",
            "mnemonic:",
            "seed_phrase:",
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
            "capability_token:",
        ] {
            assert!(
                !source.contains(forbidden_runtime_pattern),
                "Phase 7B enrollment contract source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
