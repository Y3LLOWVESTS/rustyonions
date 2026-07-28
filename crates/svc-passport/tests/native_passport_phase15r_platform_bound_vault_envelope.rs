#[cfg(not(feature = "native-passport"))]
#[test]
fn phase15r_platform_bound_vault_is_feature_gated() {
    assert!(!cfg!(feature = "native-passport"));
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        decode_native_platform_bound_vault, encode_native_platform_bound_vault,
        encode_native_vault_authenticated_header, native_platform_bound_vault_posture,
        NativeEncryptedVaultV1, NativePinWrappedCompartmentVmkV1, NativePinWrappedVaultKeysV1,
        NativePlatformBoundVaultError, NativePlatformBoundVaultV1, NativePlatformFamily,
        NativeSealedMaterialV1, NativeSecureCompartment, NATIVE_PASSPORT_PHASE15R_CORE_LABEL,
        PHASE15Q_WRAPPED_VMK_BYTES,
    };

    const ROOT_SALT: [u8; 16] = [0x11; 16];

    const OPERATIONAL_SALT: [u8; 16] = [0x22; 16];

    const ROOT_NONCE: [u8; 24] = [0x33; 24];

    const OPERATIONAL_NONCE: [u8; 24] = [0x44; 24];

    fn wrapped_compartment(
        compartment: NativeSecureCompartment,
        salt: [u8; 16],
        nonce: [u8; 24],
        ciphertext_byte: u8,
    ) -> NativePinWrappedCompartmentVmkV1 {
        NativePinWrappedCompartmentVmkV1::new(
            compartment,
            salt,
            nonce,
            encode_native_vault_authenticated_header(compartment, &salt, &nonce),
            vec![ciphertext_byte; PHASE15Q_WRAPPED_VMK_BYTES],
        )
        .expect("structurally valid wrapped compartment")
    }

    fn wrapped_keys() -> NativePinWrappedVaultKeysV1 {
        NativePinWrappedVaultKeysV1::new(
            wrapped_compartment(
                NativeSecureCompartment::RecoveryRoot,
                ROOT_SALT,
                ROOT_NONCE,
                0x55,
            ),
            wrapped_compartment(
                NativeSecureCompartment::DeviceKey,
                OPERATIONAL_SALT,
                OPERATIONAL_NONCE,
                0x66,
            ),
        )
        .expect("two-compartment wrapped keys")
    }

    fn sealed_factor(
        platform: NativePlatformFamily,
        compartment: NativeSecureCompartment,
        reference: &[u8],
    ) -> NativeSealedMaterialV1 {
        NativeSealedMaterialV1::new(platform, compartment, reference.to_vec())
            .expect("bounded sealed material")
    }

    fn platform_bound_vault() -> NativePlatformBoundVaultV1 {
        NativePlatformBoundVaultV1::new(
            NativePlatformFamily::MacosKeychain,
            sealed_factor(
                NativePlatformFamily::MacosKeychain,
                NativeSecureCompartment::RecoveryRoot,
                b"memory://recovery-root",
            ),
            sealed_factor(
                NativePlatformFamily::MacosKeychain,
                NativeSecureCompartment::DeviceKey,
                b"memory://operational",
            ),
            wrapped_keys(),
        )
        .expect("platform-bound vault")
    }

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    #[test]
    fn phase15r_posture_preserves_owner_and_secret_boundaries() {
        let posture = native_platform_bound_vault_posture();

        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE15R_CORE_LABEL,);

        assert_eq!(posture.canonical_owner, "svc-passport",);

        assert!(posture.sealed_root_factor_persisted);

        assert!(posture.sealed_operational_factor_persisted);

        assert!(posture.phase15q_wrapped_keys_reused);

        assert!(posture.one_platform_family_bound);

        assert!(posture.bounded_binary_codec_added);

        assert!(!posture.plaintext_platform_factor_persisted);

        assert!(!posture.pin_persisted);
        assert!(!posture.vmk_plaintext_persisted);
        assert!(!posture.platform_backend_called);
        assert!(!posture.filesystem_io_added);
        assert!(!posture.tauri_command_added);

        assert!(!posture.ron_kms_key_lifecycle_touched);

        assert!(!posture.frontend_secret_custody_added);

        assert!(!posture.wallet_or_ledger_mutation_added);
    }

    #[test]
    fn phase15r_platform_bound_vault_round_trips() {
        let vault = platform_bound_vault();

        let encoded =
            encode_native_platform_bound_vault(&vault).expect("encode platform-bound vault");

        let decoded =
            decode_native_platform_bound_vault(&encoded).expect("decode platform-bound vault");

        assert_eq!(
            decoded.platform_family(),
            NativePlatformFamily::MacosKeychain,
        );

        assert_eq!(
            decoded.recovery_root_factor().compartment,
            NativeSecureCompartment::RecoveryRoot,
        );

        assert_eq!(
            decoded.operational_factor().compartment,
            NativeSecureCompartment::DeviceKey,
        );

        assert_eq!(
            decoded.recovery_root_factor().as_slice(),
            b"memory://recovery-root",
        );

        assert_eq!(
            decoded.operational_factor().as_slice(),
            b"memory://operational",
        );

        assert_eq!(
            decoded.wrapped_keys().operational().wrapped_vmk_len(),
            PHASE15Q_WRAPPED_VMK_BYTES,
        );
    }

    #[test]
    fn phase15r_rejects_platform_and_compartment_mismatch() {
        assert_eq!(
            NativePlatformBoundVaultV1::new(
                NativePlatformFamily::WindowsDpapi,
                sealed_factor(
                    NativePlatformFamily::MacosKeychain,
                    NativeSecureCompartment::RecoveryRoot,
                    b"root",
                ),
                sealed_factor(
                    NativePlatformFamily::WindowsDpapi,
                    NativeSecureCompartment::DeviceKey,
                    b"operational",
                ),
                wrapped_keys(),
            ),
            Err(NativePlatformBoundVaultError::PlatformFamilyMismatch {
                expected: NativePlatformFamily::WindowsDpapi,
                actual: NativePlatformFamily::MacosKeychain,
            },),
        );

        assert_eq!(
            NativePlatformBoundVaultV1::new(
                NativePlatformFamily::MacosKeychain,
                sealed_factor(
                    NativePlatformFamily::MacosKeychain,
                    NativeSecureCompartment::DeviceKey,
                    b"wrong-root-compartment",
                ),
                sealed_factor(
                    NativePlatformFamily::MacosKeychain,
                    NativeSecureCompartment::DeviceKey,
                    b"operational",
                ),
                wrapped_keys(),
            ),
            Err(NativePlatformBoundVaultError::CompartmentMismatch {
                expected: NativeSecureCompartment::RecoveryRoot,
                actual: NativeSecureCompartment::DeviceKey,
            },),
        );
    }

    #[test]
    fn phase15r_rejects_truncation_and_trailing_bytes() {
        let encoded = encode_native_platform_bound_vault(&platform_bound_vault())
            .expect("encode platform-bound vault");

        let truncated =
            NativeEncryptedVaultV1::new(encoded.as_slice()[..encoded.len() - 1].to_vec())
                .expect("bounded truncated vault");

        assert_eq!(
            decode_native_platform_bound_vault(&truncated,),
            Err(NativePlatformBoundVaultError::InvalidEncodedVault,),
        );

        let mut trailing = encoded.as_slice().to_vec();

        trailing.push(0);

        let trailing = NativeEncryptedVaultV1::new(trailing).expect("bounded trailing vault");

        assert_eq!(
            decode_native_platform_bound_vault(&trailing,),
            Err(NativePlatformBoundVaultError::InvalidEncodedVault,),
        );
    }

    #[test]
    fn phase15r_mobile_platform_codes_round_trip_and_unknown_local_rejects() {
        for platform in [
            NativePlatformFamily::IosKeychain,
            NativePlatformFamily::AndroidKeystore,
        ] {
            let vault = NativePlatformBoundVaultV1::new(
                platform,
                sealed_factor(
                    platform,
                    NativeSecureCompartment::RecoveryRoot,
                    b"mobile://recovery-root",
                ),
                sealed_factor(
                    platform,
                    NativeSecureCompartment::DeviceKey,
                    b"mobile://operational",
                ),
                wrapped_keys(),
            )
            .expect("mobile platform-bound vault");

            let encoded = encode_native_platform_bound_vault(&vault)
                .expect("encode mobile platform-bound vault");

            let decoded = decode_native_platform_bound_vault(&encoded)
                .expect("decode mobile platform-bound vault");

            assert_eq!(decoded.platform_family(), platform,);

            assert_eq!(decoded.recovery_root_factor().platform_family, platform,);

            assert_eq!(decoded.operational_factor().platform_family, platform,);
        }

        let unknown = NativePlatformBoundVaultV1::new(
            NativePlatformFamily::UnknownLocal,
            sealed_factor(
                NativePlatformFamily::UnknownLocal,
                NativeSecureCompartment::RecoveryRoot,
                b"unknown://recovery-root",
            ),
            sealed_factor(
                NativePlatformFamily::UnknownLocal,
                NativeSecureCompartment::DeviceKey,
                b"unknown://operational",
            ),
            wrapped_keys(),
        )
        .expect("structurally matched unknown-local vault");

        assert_eq!(
            encode_native_platform_bound_vault(&unknown,),
            Err(NativePlatformBoundVaultError::UnsupportedPlatformFamily {
                actual: NativePlatformFamily::UnknownLocal,
            },),
        );
    }

    #[test]
    fn phase15r_debug_and_source_boundaries_are_redacted() {
        let debug = format!("{:?}", platform_bound_vault(),);

        assert!(debug.contains("REDACTED_PLATFORM_SEALED_MATERIAL"));

        assert!(!debug.contains("memory://recovery-root"));

        let source = fs::read_to_string(repo_file("src/native/platform_bound_vault.rs"))
            .expect("Phase 15R core source");

        for required in [
            "NativeSealedMaterialV1",
            "NativePinWrappedVaultKeysV1",
            "encode_native_pin_wrapped_vault_keys",
            "decode_native_pin_wrapped_vault_keys",
            "PHASE15R_PLATFORM_BOUND_VAULT_MAGIC",
        ] {
            assert!(
                source.contains(required),
                "Phase 15R source missing {required}",
            );
        }

        for forbidden in [
            "ron_kms::",
            "ed25519_dalek",
            "SigningKey",
            "VerifyingKey",
            "#[tauri::command]",
            "tauri::",
            "std::fs::",
            "tokio::fs",
            "getrandom::",
            "seal_native_secret(",
            "unseal_native_secret(",
            "wallet.spend(",
            "ledger.write(",
            "println!",
            "eprintln!",
            "tracing::",
        ] {
            assert!(
                !source.contains(forbidden),
                "Phase 15R source contains forbidden {forbidden}",
            );
        }
    }
}
