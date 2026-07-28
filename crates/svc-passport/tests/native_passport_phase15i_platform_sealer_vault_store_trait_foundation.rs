#[cfg(not(feature = "native-passport"))]
#[test]
fn phase15i_platform_storage_traits_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native platform-storage traits"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf, sync::Mutex};

    use svc_passport::native::{
        load_native_encrypted_vault, native_platform_storage_trait_posture,
        recover_native_interrupted_vault_write, remove_native_encrypted_vault, seal_native_secret,
        unseal_native_secret, write_native_encrypted_vault_atomic, NativeEncryptedVaultV1,
        NativePlatformFamily, NativePlatformSealer, NativePlatformStorageError,
        NativePlatformStorageOperation, NativeSealedMaterialV1, NativeSecretBytes,
        NativeSecureCompartment, NativeVaultRecoveryOutcome, NativeVaultRemovalOutcome,
        NativeVaultStore, NATIVE_PASSPORT_PHASE15I_LABEL, PHASE15I_ATOMIC_VAULT_WRITE_STEPS,
        PHASE15I_FORBIDDEN_PLATFORM_STORAGE_AUTHORITY_FLAGS, PHASE15I_MAX_ENCRYPTED_VAULT_BYTES,
        PHASE15I_MAX_SEALED_MATERIAL_BYTES, PHASE15I_MAX_SECRET_MATERIAL_BYTES,
        PHASE15I_PLATFORM_STORAGE_DOMAIN, PHASE15I_PLATFORM_STORAGE_VERSION,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    #[derive(Debug)]
    struct TestSealer {
        platform_family: NativePlatformFamily,
    }

    impl NativePlatformSealer for TestSealer {
        fn platform_family(&self) -> NativePlatformFamily {
            self.platform_family
        }

        fn seal(
            &self,
            compartment: NativeSecureCompartment,
            secret: &NativeSecretBytes,
        ) -> Result<NativeSealedMaterialV1, NativePlatformStorageError> {
            let compartment_tag = match compartment {
                NativeSecureCompartment::RecoveryRoot => 1,
                NativeSecureCompartment::DeviceKey => 2,
            };

            let mut sealed = Vec::with_capacity(secret.len() + 1);
            sealed.push(compartment_tag);
            sealed.extend(secret.as_slice().iter().map(|byte| byte ^ 0xA5));

            NativeSealedMaterialV1::new(self.platform_family, compartment, sealed)
        }

        fn unseal(
            &self,
            sealed: &NativeSealedMaterialV1,
        ) -> Result<NativeSecretBytes, NativePlatformStorageError> {
            let expected_tag = match sealed.compartment {
                NativeSecureCompartment::RecoveryRoot => 1,
                NativeSecureCompartment::DeviceKey => 2,
            };

            if sealed.as_slice().first() != Some(&expected_tag) {
                return Err(NativePlatformStorageError::BackendFailure {
                    operation: NativePlatformStorageOperation::Unseal,
                });
            }

            NativeSecretBytes::new(
                sealed.as_slice()[1..]
                    .iter()
                    .map(|byte| byte ^ 0xA5)
                    .collect(),
            )
        }
    }

    #[derive(Debug, Default)]
    struct MemoryVaultStore {
        vault: Mutex<Option<NativeEncryptedVaultV1>>,
    }

    impl NativeVaultStore for MemoryVaultStore {
        fn load_encrypted_vault(
            &self,
        ) -> Result<Option<NativeEncryptedVaultV1>, NativePlatformStorageError> {
            Ok(self.vault.lock().expect("memory vault lock").clone())
        }

        fn write_encrypted_vault_atomic(
            &self,
            vault: &NativeEncryptedVaultV1,
        ) -> Result<(), NativePlatformStorageError> {
            *self.vault.lock().expect("memory vault lock") = Some(vault.clone());

            Ok(())
        }

        fn recover_interrupted_write(
            &self,
        ) -> Result<NativeVaultRecoveryOutcome, NativePlatformStorageError> {
            Ok(NativeVaultRecoveryOutcome::NoRecoveryNeeded)
        }

        fn remove_encrypted_vault(
            &self,
        ) -> Result<NativeVaultRemovalOutcome, NativePlatformStorageError> {
            let removed = self
                .vault
                .lock()
                .expect("memory vault lock")
                .take()
                .is_some();

            Ok(if removed {
                NativeVaultRemovalOutcome::Removed
            } else {
                NativeVaultRemovalOutcome::NotFound
            })
        }
    }

    struct FailingSealer;

    impl NativePlatformSealer for FailingSealer {
        fn platform_family(&self) -> NativePlatformFamily {
            NativePlatformFamily::MacosKeychain
        }

        fn seal(
            &self,
            _compartment: NativeSecureCompartment,
            _secret: &NativeSecretBytes,
        ) -> Result<NativeSealedMaterialV1, NativePlatformStorageError> {
            Err(NativePlatformStorageError::BackendFailure {
                operation: NativePlatformStorageOperation::Seal,
            })
        }

        fn unseal(
            &self,
            _sealed: &NativeSealedMaterialV1,
        ) -> Result<NativeSecretBytes, NativePlatformStorageError> {
            Err(NativePlatformStorageError::BackendFailure {
                operation: NativePlatformStorageOperation::Unseal,
            })
        }
    }

    #[test]
    fn phase15i_posture_limits_and_atomic_contract_are_locked() {
        let posture = native_platform_storage_trait_posture();

        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE15I_LABEL);
        assert!(posture.secret_buffer_added);
        assert!(posture.secret_buffer_zeroizes_on_drop);
        assert!(posture.sealed_material_envelope_added);
        assert!(posture.encrypted_vault_envelope_added);
        assert!(posture.platform_sealer_trait_added);
        assert!(posture.vault_store_trait_added);
        assert!(posture.atomic_write_contract_added);
        assert!(posture.interrupted_write_recovery_contract_added);
        assert!(posture.bounded_validation_helpers_added);

        assert!(!posture.os_platform_adapter_added);
        assert!(!posture.filesystem_adapter_added);
        assert!(!posture.encryption_runtime_added);
        assert!(!posture.decryption_runtime_added);
        assert!(!posture.vault_unlock_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.frontend_secret_custody_added);
        assert!(!posture.capability_issuance_added);
        assert!(!posture.wallet_or_ledger_mutation_added);

        assert_eq!(
            PHASE15I_PLATFORM_STORAGE_DOMAIN,
            "native-passport/platform-storage/v1"
        );
        assert_eq!(PHASE15I_PLATFORM_STORAGE_VERSION, 1);
        assert_eq!(PHASE15I_ATOMIC_VAULT_WRITE_STEPS.len(), 6);

        for forbidden in [
            "os_platform_adapter",
            "filesystem_adapter",
            "plaintext_temporary_file",
            "frontend_secret_custody",
            "raw_secret_export",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(PHASE15I_FORBIDDEN_PLATFORM_STORAGE_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase15i_sealer_helpers_round_trip_bounded_secret_material() {
        let sealer = TestSealer {
            platform_family: NativePlatformFamily::MacosKeychain,
        };

        let secret =
            NativeSecretBytes::new(b"phase15i-device-secret".to_vec()).expect("bounded secret");

        let sealed = seal_native_secret(
            &sealer,
            NativePlatformFamily::MacosKeychain,
            NativeSecureCompartment::DeviceKey,
            &secret,
        )
        .expect("sealed material");

        assert_eq!(sealed.contract_domain, PHASE15I_PLATFORM_STORAGE_DOMAIN);
        assert_eq!(sealed.contract_version, PHASE15I_PLATFORM_STORAGE_VERSION);
        assert_eq!(sealed.compartment, NativeSecureCompartment::DeviceKey);
        assert!(!sealed.is_empty());

        let unsealed = unseal_native_secret(
            &sealer,
            NativePlatformFamily::MacosKeychain,
            NativeSecureCompartment::DeviceKey,
            &sealed,
        )
        .expect("unsealed secret");

        assert_eq!(unsealed.as_slice(), b"phase15i-device-secret");

        let debug = format!("{unsealed:?}");
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("phase15i-device-secret"));
    }

    #[test]
    fn phase15i_vault_store_helpers_write_load_recover_and_remove() {
        let store = MemoryVaultStore::default();

        assert_eq!(
            load_native_encrypted_vault(&store).expect("empty vault load"),
            None
        );

        let vault = NativeEncryptedVaultV1::new(b"encrypted-vault-envelope".to_vec())
            .expect("encrypted vault");

        write_native_encrypted_vault_atomic(&store, &vault).expect("atomic memory write");

        let loaded = load_native_encrypted_vault(&store)
            .expect("vault load")
            .expect("stored vault");

        assert_eq!(loaded.as_slice(), b"encrypted-vault-envelope");

        assert_eq!(
            recover_native_interrupted_vault_write(&store).expect("recovery"),
            NativeVaultRecoveryOutcome::NoRecoveryNeeded
        );

        assert_eq!(
            remove_native_encrypted_vault(&store).expect("remove"),
            NativeVaultRemovalOutcome::Removed
        );

        assert_eq!(
            remove_native_encrypted_vault(&store).expect("second remove"),
            NativeVaultRemovalOutcome::NotFound
        );
    }

    #[test]
    fn phase15i_rejects_empty_oversized_and_mismatched_material() {
        assert_eq!(
            NativeSecretBytes::new(Vec::new()),
            Err(NativePlatformStorageError::EmptySecretMaterial)
        );

        assert_eq!(
            NativeSecretBytes::new(vec![7; PHASE15I_MAX_SECRET_MATERIAL_BYTES + 1]),
            Err(NativePlatformStorageError::SecretMaterialTooLarge {
                actual: PHASE15I_MAX_SECRET_MATERIAL_BYTES + 1,
                maximum: PHASE15I_MAX_SECRET_MATERIAL_BYTES,
            })
        );

        assert_eq!(
            NativeSealedMaterialV1::new(
                NativePlatformFamily::MacosKeychain,
                NativeSecureCompartment::DeviceKey,
                Vec::new(),
            ),
            Err(NativePlatformStorageError::EmptySealedMaterial)
        );

        assert_eq!(
            NativeSealedMaterialV1::new(
                NativePlatformFamily::MacosKeychain,
                NativeSecureCompartment::DeviceKey,
                vec![1; PHASE15I_MAX_SEALED_MATERIAL_BYTES + 1],
            ),
            Err(NativePlatformStorageError::SealedMaterialTooLarge {
                actual: PHASE15I_MAX_SEALED_MATERIAL_BYTES + 1,
                maximum: PHASE15I_MAX_SEALED_MATERIAL_BYTES,
            })
        );

        assert_eq!(
            NativeEncryptedVaultV1::new(vec![9; PHASE15I_MAX_ENCRYPTED_VAULT_BYTES + 1]),
            Err(NativePlatformStorageError::EncryptedVaultTooLarge {
                actual: PHASE15I_MAX_ENCRYPTED_VAULT_BYTES + 1,
                maximum: PHASE15I_MAX_ENCRYPTED_VAULT_BYTES,
            })
        );

        let sealer = TestSealer {
            platform_family: NativePlatformFamily::MacosKeychain,
        };

        let secret = NativeSecretBytes::new(vec![1, 2, 3]).expect("secret");

        assert_eq!(
            seal_native_secret(
                &sealer,
                NativePlatformFamily::WindowsDpapi,
                NativeSecureCompartment::DeviceKey,
                &secret,
            ),
            Err(NativePlatformStorageError::PlatformFamilyMismatch {
                expected: NativePlatformFamily::WindowsDpapi,
                actual: NativePlatformFamily::MacosKeychain,
            })
        );
    }

    #[test]
    fn phase15i_propagates_backend_failures_without_secret_details() {
        let sealer = FailingSealer;
        let secret = NativeSecretBytes::new(vec![1, 2, 3]).expect("secret");

        assert_eq!(
            seal_native_secret(
                &sealer,
                NativePlatformFamily::MacosKeychain,
                NativeSecureCompartment::RecoveryRoot,
                &secret,
            ),
            Err(NativePlatformStorageError::BackendFailure {
                operation: NativePlatformStorageOperation::Seal,
            })
        );
    }

    #[test]
    fn phase15i_source_remains_platform_neutral_without_runtime_adapters() {
        let source = fs::read_to_string(repo_file("src/native/platform_storage.rs"))
            .expect("Phase 15I source");

        assert!(source.contains("pub trait NativePlatformSealer"));
        assert!(source.contains("pub trait NativeVaultStore"));
        assert!(source.contains("self.bytes.fill(0)"));
        assert!(source.contains("write_encrypted_vault_atomic"));
        assert!(source.contains("recover_interrupted_write"));

        for forbidden_runtime_pattern in [
            "#[tauri::command]",
            "tauri::",
            "keyring::",
            "security_framework::",
            "secret_service::",
            "windows::Security",
            "std::fs::",
            "tokio::fs",
            "OpenOptions::",
            "File::create",
            "rename(",
            "sync_all(",
            "platform_unseal(",
            "unlock_vault(",
            "issue_capability(",
            "wallet.spend(",
            "ledger.write(",
        ] {
            assert!(
                !source.contains(forbidden_runtime_pattern),
                "Phase 15I trait foundation must not add runtime adapter pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
