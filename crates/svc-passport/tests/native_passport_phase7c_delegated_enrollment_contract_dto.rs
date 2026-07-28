#[cfg(not(feature = "native-passport"))]
#[test]
fn phase7c_delegated_enrollment_contract_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native delegated enrollment DTOs"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_passport_delegated_enrollment_posture,
        review_native_delegated_enrollment_contract_draft,
        review_native_new_device_enrollment_contract_draft,
        validate_native_delegated_enrollment_contract_descriptor, DeviceClass,
        NativeDelegatedApprovingAuthority, NativeDelegatedChallengePlaceholder,
        NativeDelegatedEnrollmentContractDraftV1, NativeDelegatedEnrollmentContractReviewError,
        NativeDelegatedEnrollmentSource, NativeDelegatedPolicyMetadata,
        NativeDelegatedProofPlaceholder, NativeNewDeviceEnrollmentContractDescriptorV1,
        NativeNewDeviceEnrollmentContractDraftV1, NativePassportSurface, NativePlatformFamily,
        NativeRestoreContractDescriptorV1, NATIVE_PASSPORT_PHASE7C_LABEL,
        PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN, PHASE6A_VAULT_HEADER_VERSION,
        PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN, PHASE6B_PIN_UNLOCK_CONTRACT_VERSION,
        PHASE7A_ALLOWED_RESTORE_SOURCES, PHASE7A_REQUIRED_RESTORE_INTENTS,
        PHASE7A_RESTORE_CONTRACT_DOMAIN, PHASE7A_RESTORE_CONTRACT_VERSION,
        PHASE7B_ALLOWED_ENROLLMENT_SOURCES, PHASE7B_ENABLED_SURFACES,
        PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN,
        PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION, PHASE7B_REQUIRED_ENROLLMENT_INTENTS,
        PHASE7C_ALLOWED_TARGET_DEVICE_CLASSES, PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_DOMAIN,
        PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_VERSION, PHASE7C_ENABLED_SURFACES,
        PHASE7C_FORBIDDEN_DELEGATED_AUTHORITY_FLAGS, PHASE7C_REQUIRED_APPROVING_AUTHORITIES,
        PHASE7C_REQUIRED_CHALLENGE_PLACEHOLDERS, PHASE7C_REQUIRED_DELEGATED_SOURCES,
        PHASE7C_REQUIRED_POLICY_METADATA, PHASE7C_REQUIRED_PROOF_PLACEHOLDERS,
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
            .expect("valid new-device enrollment contract")
    }

    fn valid_draft() -> NativeDelegatedEnrollmentContractDraftV1 {
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
    fn phase7c_label_and_posture_are_locked() {
        let posture = native_passport_delegated_enrollment_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE7C_LABEL,
            "NATIVE_PASSPORT_PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_DTO"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE7C_LABEL);
        assert!(posture.delegated_enrollment_contract_dtos_added);

        for authority_added in [
            posture.delegated_enrollment_execution_added,
            posture.invitation_signing_added,
            posture.invitation_verification_added,
            posture.challenge_runtime_added,
            posture.proof_signing_or_verification_runtime_added,
            posture.capability_issuance_added,
            posture.device_key_generation_added,
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
            posture.storage_mutation_added,
            posture.wallet_or_ledger_mutation_added,
            posture.runtime_authority_changed,
            posture.native_secret_implementation_added,
        ] {
            assert!(!authority_added);
        }

        for forbidden in [
            "delegated_enrollment_execution",
            "invitation_signing",
            "invitation_verification",
            "challenge_runtime",
            "proof_signing",
            "proof_verification",
            "capability_issuance",
            "device_key_generation",
            "storage_mutation",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE7C_FORBIDDEN_DELEGATED_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase7c_surfaces_extend_phase7b_without_back_mutating_it() {
        assert_eq!(
            PHASE7C_ENABLED_SURFACES.len(),
            PHASE7B_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE7C_ENABLED_SURFACES[..PHASE7B_ENABLED_SURFACES.len()],
            PHASE7B_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE7C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::DelegatedEnrollmentContractDto)
        );
        assert_eq!(
            PHASE7B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::NewDeviceEnrollmentContractDto)
        );
    }

    #[test]
    fn phase7c_reviews_restore_and_new_device_contracts_into_delegated_contract() {
        let restore = restore_contract();
        let enrollment = enrollment_contract(&restore);

        let descriptor =
            review_native_delegated_enrollment_contract_draft(&restore, &enrollment, valid_draft())
                .expect("valid delegated enrollment contract should review");

        assert_eq!(
            descriptor.contract_domain,
            PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_DOMAIN
        );
        assert_eq!(
            descriptor.contract_version,
            PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_VERSION
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
            descriptor.enrollment_contract_domain,
            PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN
        );
        assert_eq!(descriptor.target_device_class, DeviceClass::DesktopReadOnly);
        assert_eq!(
            descriptor.delegated_sources,
            PHASE7C_REQUIRED_DELEGATED_SOURCES
        );
        assert_eq!(
            descriptor.approving_authorities,
            PHASE7C_REQUIRED_APPROVING_AUTHORITIES
        );
        assert_eq!(
            descriptor.challenge_placeholders,
            PHASE7C_REQUIRED_CHALLENGE_PLACEHOLDERS
        );
        assert_eq!(
            descriptor.proof_placeholders,
            PHASE7C_REQUIRED_PROOF_PLACEHOLDERS
        );
        assert_eq!(descriptor.policy_metadata, PHASE7C_REQUIRED_POLICY_METADATA);
        assert!(descriptor.contract_only);

        validate_native_delegated_enrollment_contract_descriptor(
            &restore,
            &enrollment,
            &descriptor,
        )
        .expect("reviewed delegated descriptor should validate");
    }

    #[test]
    fn phase7c_required_labels_and_target_classes_are_locked() {
        assert_eq!(
            PHASE7C_REQUIRED_DELEGATED_SOURCES,
            &[
                NativeDelegatedEnrollmentSource::RestoreContract,
                NativeDelegatedEnrollmentSource::NewDeviceEnrollmentContract,
                NativeDelegatedEnrollmentSource::DelegatedInvitationMetadata,
            ]
        );
        assert_eq!(
            PHASE7C_REQUIRED_APPROVING_AUTHORITIES,
            &[
                NativeDelegatedApprovingAuthority::RecoveryRootApproval,
                NativeDelegatedApprovingAuthority::ExistingAuthorizedDevice,
            ]
        );
        assert_eq!(
            PHASE7C_REQUIRED_CHALLENGE_PLACEHOLDERS,
            &[
                NativeDelegatedChallengePlaceholder::InvitationChallenge,
                NativeDelegatedChallengePlaceholder::EnrollmentChallenge,
            ]
        );
        assert_eq!(
            PHASE7C_REQUIRED_PROOF_PLACEHOLDERS,
            &[
                NativeDelegatedProofPlaceholder::InvitationProof,
                NativeDelegatedProofPlaceholder::DevicePossessionProof,
            ]
        );
        assert_eq!(
            PHASE7C_REQUIRED_POLICY_METADATA,
            &[
                NativeDelegatedPolicyMetadata::SingleUseInvitation,
                NativeDelegatedPolicyMetadata::ExpiresBeforeCapabilityIssuance,
                NativeDelegatedPolicyMetadata::ReadOnlyCapabilityCeiling,
            ]
        );
        assert_eq!(
            PHASE7C_ALLOWED_TARGET_DEVICE_CLASSES,
            &[
                DeviceClass::TvReadOnly,
                DeviceClass::DesktopReadOnly,
                DeviceClass::MobileReadOnly,
            ]
        );
    }

    #[test]
    fn phase7c_rejects_domain_version_platform_and_linkage_drift() {
        let restore = restore_contract();
        let enrollment = enrollment_contract(&restore);

        let mut bad_domain = valid_draft();
        bad_domain.contract_domain = "native-passport/delegated-enrollment-contract/v0";
        assert_eq!(
            review_native_delegated_enrollment_contract_draft(&restore, &enrollment, bad_domain,),
            Err(NativeDelegatedEnrollmentContractReviewError::ContractDomainMismatch)
        );

        let mut bad_version = valid_draft();
        bad_version.contract_version = 2;
        assert_eq!(
            review_native_delegated_enrollment_contract_draft(&restore, &enrollment, bad_version,),
            Err(NativeDelegatedEnrollmentContractReviewError::ContractVersionMismatch)
        );

        let mut bad_platform = valid_draft();
        bad_platform.platform_family = NativePlatformFamily::AndroidKeystore;
        assert_eq!(
            review_native_delegated_enrollment_contract_draft(&restore, &enrollment, bad_platform,),
            Err(NativeDelegatedEnrollmentContractReviewError::PlatformFamilyMismatch)
        );

        let mut bad_restore_domain = valid_draft();
        bad_restore_domain.restore_contract_domain = "native-passport/restore-contract/v0";
        assert_eq!(
            review_native_delegated_enrollment_contract_draft(
                &restore,
                &enrollment,
                bad_restore_domain,
            ),
            Err(NativeDelegatedEnrollmentContractReviewError::RestoreContractDomainMismatch)
        );

        let mut bad_enrollment_domain = valid_draft();
        bad_enrollment_domain.enrollment_contract_domain =
            "native-passport/new-device-enrollment-contract/v0";
        assert_eq!(
            review_native_delegated_enrollment_contract_draft(
                &restore,
                &enrollment,
                bad_enrollment_domain,
            ),
            Err(NativeDelegatedEnrollmentContractReviewError::EnrollmentContractDomainMismatch)
        );
    }

    #[test]
    fn phase7c_rejects_empty_duplicate_missing_required_and_non_contract_descriptor() {
        let restore = restore_contract();
        let enrollment = enrollment_contract(&restore);

        let mut missing_sources = valid_draft();
        missing_sources.requested_delegated_sources.clear();
        assert_eq!(
            review_native_delegated_enrollment_contract_draft(
                &restore,
                &enrollment,
                missing_sources,
            ),
            Err(NativeDelegatedEnrollmentContractReviewError::MissingDelegatedSources)
        );

        let mut duplicate_source = valid_draft();
        duplicate_source
            .requested_delegated_sources
            .push(NativeDelegatedEnrollmentSource::RestoreContract);
        assert_eq!(
            review_native_delegated_enrollment_contract_draft(
                &restore,
                &enrollment,
                duplicate_source,
            ),
            Err(NativeDelegatedEnrollmentContractReviewError::DuplicateDelegatedSource)
        );

        let mut missing_required_source = valid_draft();
        missing_required_source.requested_delegated_sources = vec![
            NativeDelegatedEnrollmentSource::RestoreContract,
            NativeDelegatedEnrollmentSource::NewDeviceEnrollmentContract,
        ];
        assert_eq!(
            review_native_delegated_enrollment_contract_draft(
                &restore,
                &enrollment,
                missing_required_source,
            ),
            Err(NativeDelegatedEnrollmentContractReviewError::MissingRequiredDelegatedSource)
        );

        let mut missing_authorities = valid_draft();
        missing_authorities.requested_approving_authorities.clear();
        assert_eq!(
            review_native_delegated_enrollment_contract_draft(
                &restore,
                &enrollment,
                missing_authorities,
            ),
            Err(NativeDelegatedEnrollmentContractReviewError::MissingApprovingAuthorities)
        );

        let mut missing_challenges = valid_draft();
        missing_challenges.requested_challenge_placeholders.clear();
        assert_eq!(
            review_native_delegated_enrollment_contract_draft(
                &restore,
                &enrollment,
                missing_challenges,
            ),
            Err(NativeDelegatedEnrollmentContractReviewError::MissingChallengePlaceholders)
        );

        let mut missing_proofs = valid_draft();
        missing_proofs.requested_proof_placeholders.clear();
        assert_eq!(
            review_native_delegated_enrollment_contract_draft(
                &restore,
                &enrollment,
                missing_proofs,
            ),
            Err(NativeDelegatedEnrollmentContractReviewError::MissingProofPlaceholders)
        );

        let mut missing_policy = valid_draft();
        missing_policy.requested_policy_metadata.clear();
        assert_eq!(
            review_native_delegated_enrollment_contract_draft(
                &restore,
                &enrollment,
                missing_policy,
            ),
            Err(NativeDelegatedEnrollmentContractReviewError::MissingPolicyMetadata)
        );

        let mut descriptor =
            review_native_delegated_enrollment_contract_draft(&restore, &enrollment, valid_draft())
                .expect("valid descriptor");
        descriptor.contract_only = false;

        assert_eq!(
            validate_native_delegated_enrollment_contract_descriptor(
                &restore,
                &enrollment,
                &descriptor,
            ),
            Err(
                NativeDelegatedEnrollmentContractReviewError::DelegatedEnrollmentContractNotContractOnly
            )
        );
    }

    #[test]
    fn phase7c_rejects_delegated_runtime_signing_proof_capability_key_storage_route_and_mutation_flags(
    ) {
        let restore = restore_contract();
        let enrollment = enrollment_contract(&restore);

        let mutators: &[fn(&mut NativeDelegatedEnrollmentContractDraftV1)] = &[
            |draft| draft.requests_delegated_enrollment_execution = true,
            |draft| draft.requests_invitation_signing = true,
            |draft| draft.requests_invitation_verification = true,
            |draft| draft.requests_challenge_runtime = true,
            |draft| draft.requests_proof_signing_or_verification = true,
            |draft| draft.requests_capability_issuance = true,
            |draft| draft.requests_device_key_generation = true,
            |draft| draft.requests_key_derivation_runtime = true,
            |draft| draft.requests_restore_execution = true,
            |draft| draft.requests_pin_validation_runtime = true,
            |draft| draft.requests_pin_derivation_runtime = true,
            |draft| draft.requests_vault_unlock = true,
            |draft| draft.includes_vault_runtime = true,
            |draft| draft.includes_platform_sealer_implementation = true,
            |draft| draft.stores_invitation_or_secret_material = true,
            |draft| draft.exports_secret_or_material = true,
            |draft| draft.requests_encryption_or_decryption = true,
            |draft| draft.requests_live_rpc = true,
            |draft| draft.requests_runtime_io = true,
            |draft| draft.adds_routes = true,
            |draft| draft.requests_storage_mutation = true,
            |draft| draft.requests_wallet_or_ledger_mutation = true,
        ];

        for mutate in mutators {
            let mut draft = valid_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_delegated_enrollment_contract_draft(&restore, &enrollment, draft,),
                Err(NativeDelegatedEnrollmentContractReviewError::UnsafeDelegatedAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase7c_source_remains_contract_only_without_runtime_or_secrets() {
        let source = fs::read_to_string(repo_file("src/native/delegated_enrollment.rs"))
            .expect("delegated source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeDelegatedEnrollmentContractDescriptorV1"));
        assert!(source.contains("NativeDelegatedEnrollmentContractDraftV1"));
        assert!(source.contains("review_native_delegated_enrollment_contract_draft"));
        assert!(native_mod.contains("DelegatedEnrollmentContractDto"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "execute_delegated_enrollment(",
            "delegate_enrollment(",
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
            "pin:",
            "pin_hash:",
            "root_private_key:",
            "device_private_key:",
            "vault_master_key:",
            "ciphertext:",
            "plaintext:",
            "sealed_bytes:",
            "capability_token:",
        ] {
            assert!(
                !source.contains(forbidden_runtime_pattern),
                "Phase 7C delegated enrollment source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
