#[cfg(not(feature = "native-passport"))]
#[test]
fn phase7d_acceptance_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native Passport Phase 7 acceptance surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_delegated_enrollment_posture,
        native_passport_new_device_enrollment_posture, native_passport_restore_posture,
        review_native_delegated_enrollment_contract_draft,
        review_native_new_device_enrollment_contract_draft,
        validate_native_delegated_enrollment_contract_descriptor,
        validate_native_new_device_enrollment_contract_descriptor, DeviceClass,
        NativeDelegatedEnrollmentContractDraftV1, NativeDelegatedEnrollmentContractReviewError,
        NativeNewDeviceEnrollmentContractDescriptorV1, NativeNewDeviceEnrollmentContractDraftV1,
        NativePassportSurface, NativePlatformFamily, NativeRestoreContractDescriptorV1,
        PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN, PHASE6A_VAULT_HEADER_VERSION,
        PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN, PHASE6B_PIN_UNLOCK_CONTRACT_VERSION,
        PHASE7A_ALLOWED_RESTORE_SOURCES, PHASE7A_ENABLED_SURFACES,
        PHASE7A_REQUIRED_RESTORE_INTENTS, PHASE7A_RESTORE_CONTRACT_DOMAIN,
        PHASE7A_RESTORE_CONTRACT_VERSION, PHASE7B_ALLOWED_ENROLLMENT_SOURCES,
        PHASE7B_ENABLED_SURFACES, PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN,
        PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION, PHASE7B_REQUIRED_ENROLLMENT_INTENTS,
        PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_DOMAIN,
        PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_VERSION, PHASE7C_ENABLED_SURFACES,
        PHASE7C_REQUIRED_APPROVING_AUTHORITIES, PHASE7C_REQUIRED_CHALLENGE_PLACEHOLDERS,
        PHASE7C_REQUIRED_DELEGATED_SOURCES, PHASE7C_REQUIRED_POLICY_METADATA,
        PHASE7C_REQUIRED_PROOF_PLACEHOLDERS,
    };

    const PHASE7D_ACCEPTANCE_LABEL: &str =
        "NATIVE_PASSPORT_PHASE7D_RESTORE_NEW_DEVICE_DELEGATED_ENROLLMENT_ACCEPTANCE";

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

    fn enrollment_draft() -> NativeNewDeviceEnrollmentContractDraftV1 {
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

    fn enrollment_contract(
        restore: &NativeRestoreContractDescriptorV1,
    ) -> NativeNewDeviceEnrollmentContractDescriptorV1 {
        review_native_new_device_enrollment_contract_draft(restore, enrollment_draft())
            .expect("valid Phase 7B enrollment contract")
    }

    fn delegated_draft() -> NativeDelegatedEnrollmentContractDraftV1 {
        NativeDelegatedEnrollmentContractDraftV1 {
            contract_domain: PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_DOMAIN,
            contract_version: PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_VERSION,
            platform_family: NativePlatformFamily::MacosKeychain,
            restore_contract_domain: PHASE7A_RESTORE_CONTRACT_DOMAIN,
            restore_contract_version: PHASE7A_RESTORE_CONTRACT_VERSION,
            enrollment_contract_domain: PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN,
            enrollment_contract_version: PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION,
            target_device_class: DeviceClass::DesktopReadOnly,
            requested_delegated_sources: PHASE7C_REQUIRED_DELEGATED_SOURCES.to_vec(),
            requested_approving_authorities: PHASE7C_REQUIRED_APPROVING_AUTHORITIES.to_vec(),
            requested_challenge_placeholders: PHASE7C_REQUIRED_CHALLENGE_PLACEHOLDERS.to_vec(),
            requested_proof_placeholders: PHASE7C_REQUIRED_PROOF_PLACEHOLDERS.to_vec(),
            requested_policy_metadata: PHASE7C_REQUIRED_POLICY_METADATA.to_vec(),
            requests_delegated_enrollment_execution: false,
            requests_invitation_signing: false,
            requests_invitation_verification: false,
            requests_challenge_runtime: false,
            requests_proof_signing_or_verification: false,
            requests_capability_issuance: false,
            requests_device_key_generation: false,
            requests_key_derivation_runtime: false,
            requests_restore_execution: false,
            requests_pin_validation_runtime: false,
            requests_pin_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            includes_platform_sealer_implementation: false,
            stores_invitation_or_secret_material: false,
            exports_secret_or_material: false,
            requests_encryption_or_decryption: false,
            requests_live_rpc: false,
            requests_runtime_io: false,
            adds_routes: false,
            requests_storage_mutation: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase7d_acceptance_label_is_locked() {
        assert_eq!(
            PHASE7D_ACCEPTANCE_LABEL,
            "NATIVE_PASSPORT_PHASE7D_RESTORE_NEW_DEVICE_DELEGATED_ENROLLMENT_ACCEPTANCE"
        );
    }

    #[test]
    fn phase7d_accepts_restore_new_device_and_delegated_surface_chain() {
        assert_eq!(
            PHASE7B_ENABLED_SURFACES.len(),
            PHASE7A_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            PHASE7C_ENABLED_SURFACES.len(),
            PHASE7B_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            PHASE7A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::RestoreContractDto)
        );
        assert_eq!(
            PHASE7B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::NewDeviceEnrollmentContractDto)
        );
        assert_eq!(
            PHASE7C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::DelegatedEnrollmentContractDto)
        );
    }

    #[test]
    fn phase7d_accepts_phase7_postures_as_contract_only() {
        let restore_posture = format!("{:?}", native_passport_restore_posture());
        let enrollment_posture = format!("{:?}", native_passport_new_device_enrollment_posture());
        let delegated_posture = format!("{:?}", native_passport_delegated_enrollment_posture());

        for posture in [&restore_posture, &enrollment_posture, &delegated_posture] {
            assert!(posture.contains("runtime_authority_changed: false"));
            assert!(posture.contains("native_secret_implementation_added: false"));
            assert!(posture.contains("runtime_io_added: false"));
            assert!(posture.contains("routes_added: false"));
            assert!(posture.contains("capability_issuance_added: false"));
        }

        assert!(restore_posture.contains("restore_contract_dtos_added: true"));
        assert!(enrollment_posture.contains("new_device_enrollment_contract_dtos_added: true"));
        assert!(delegated_posture.contains("delegated_enrollment_contract_dtos_added: true"));
    }

    #[test]
    fn phase7d_accepts_reviewed_restore_new_device_delegated_contract_flow() {
        let restore = restore_contract();
        assert!(restore.contract_only);
        assert_eq!(restore.contract_domain, PHASE7A_RESTORE_CONTRACT_DOMAIN);

        let enrollment = enrollment_contract(&restore);
        validate_native_new_device_enrollment_contract_descriptor(&restore, &enrollment)
            .expect("Phase 7B descriptor should validate against restore contract");

        let delegated = review_native_delegated_enrollment_contract_draft(
            &restore,
            &enrollment,
            delegated_draft(),
        )
        .expect("Phase 7C delegated draft should review");

        validate_native_delegated_enrollment_contract_descriptor(&restore, &enrollment, &delegated)
            .expect("Phase 7C descriptor should validate against restore and enrollment contracts");

        assert!(enrollment.contract_only);
        assert!(delegated.contract_only);
        assert_eq!(enrollment.target_device_class, DeviceClass::DesktopReadOnly);
        assert_eq!(delegated.target_device_class, DeviceClass::DesktopReadOnly);
        assert_eq!(
            delegated.enrollment_contract_domain,
            PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN
        );
        assert_eq!(
            delegated.delegated_sources,
            PHASE7C_REQUIRED_DELEGATED_SOURCES
        );
    }

    #[test]
    fn phase7d_accepts_reference_rejection_when_predecessor_contract_is_not_contract_only() {
        let restore = restore_contract();
        let mut enrollment = enrollment_contract(&restore);
        enrollment.contract_only = false;

        assert_eq!(
            review_native_delegated_enrollment_contract_draft(
                &restore,
                &enrollment,
                delegated_draft(),
            ),
            Err(NativeDelegatedEnrollmentContractReviewError::EnrollmentContractReferenceInvalid)
        );
    }

    #[test]
    fn phase7d_acceptance_sources_remain_contract_only_without_runtime_or_secret_material() {
        let restore_source =
            fs::read_to_string(repo_file("src/native/restore.rs")).expect("restore source");
        let enrollment_source =
            fs::read_to_string(repo_file("src/native/enrollment.rs")).expect("enrollment source");
        let delegated_source = fs::read_to_string(repo_file("src/native/delegated_enrollment.rs"))
            .expect("delegated source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        for required_token in [
            "RestoreContractDto",
            "NewDeviceEnrollmentContractDto",
            "DelegatedEnrollmentContractDto",
            "PHASE7A_ENABLED_SURFACES",
            "PHASE7B_ENABLED_SURFACES",
            "PHASE7C_ENABLED_SURFACES",
        ] {
            assert!(
                native_mod.contains(required_token),
                "native module should expose {required_token}"
            );
        }

        let combined =
            format!("{restore_source}\n{enrollment_source}\n{delegated_source}\n{native_mod}");

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "execute_restore(",
            "execute_enrollment(",
            "execute_delegated_enrollment(",
            "enroll_device(",
            "generate_device_key(",
            "derive_root_key(",
            "derive_device_key(",
            "authorize_device(",
            "sign(",
            "verify_signature(",
            "verify_proof(",
            "issue_capability(",
            "validate_pin(",
            "derive_pin(",
            "unlock_vault(",
            "platform_seal(",
            "platform_unseal(",
            "encrypt(",
            "decrypt(",
            "rpc.submit(",
            "rpc.send(",
            "runtime_read(",
            "runtime_write(",
            "storage.write(",
            "wallet.spend(",
            "ledger.write(",
            "mint_roc(",
            "burn_roc(",
            "mnemonic:",
            "seed_phrase:",
            "raw_pin:",
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
                !combined.contains(forbidden_runtime_pattern),
                "Phase 7 acceptance sources must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
