#[cfg(not(feature = "native-passport"))]
#[test]
fn phase15q_vault_crypto_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile Native Passport vault crypto",
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        decode_native_pin_wrapped_vault_keys, encode_native_pin_wrapped_vault_keys,
        encode_native_vault_authenticated_header, native_vault_crypto_posture,
        native_vault_kek_info, unlock_native_operational_vmk, wrap_native_compartment_vmk,
        NativeEncryptedVaultV1, NativePinWrappedCompartmentVmkV1, NativePinWrappedVaultKeysV1,
        NativeSecretBytes, NativeSecureCompartment, NativeVaultCryptoError,
        NATIVE_PASSPORT_PHASE15Q_LABEL, PHASE15Q_AUTHENTICATED_HEADER_DOMAIN,
        PHASE15Q_AUTHENTICATED_HEADER_ENCODING, PHASE15Q_AUTHENTICATED_HEADER_INPUT_FORMAT,
        PHASE15Q_OPERATIONAL_KDF_PROFILE, PHASE15Q_PLATFORM_FACTOR_BYTES,
        PHASE15Q_ROOT_KDF_PROFILE, PHASE15Q_VAULT_MASTER_KEY_BYTES, PHASE15Q_WRAPPED_VMK_BYTES,
        PHASE6A_KDF_SALT_LEN,
    };

    const PIN: &[u8] = b"correct-horse-battery-staple";

    const ROOT_SALT: [u8; 16] = [
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f,
    ];

    const OPERATIONAL_SALT: [u8; 16] = [
        0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e,
        0x2f,
    ];

    const ROOT_NONCE: [u8; 24] = [
        0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e,
        0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47,
    ];

    const OPERATIONAL_NONCE: [u8; 24] = [
        0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e,
        0x5f, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67,
    ];

    fn secret(byte: u8, length: usize) -> NativeSecretBytes {
        NativeSecretBytes::new(vec![byte; length]).expect("bounded NativeSecretBytes")
    }

    fn fake_envelope(
        compartment: NativeSecureCompartment,
        salt: [u8; 16],
        nonce: [u8; 24],
    ) -> NativePinWrappedCompartmentVmkV1 {
        let header = encode_native_vault_authenticated_header(compartment, &salt, &nonce);

        NativePinWrappedCompartmentVmkV1::new(
            compartment,
            salt,
            nonce,
            header,
            vec![0x44; PHASE15Q_WRAPPED_VMK_BYTES],
        )
        .expect("structurally valid wrapped VMK")
    }

    fn fake_vault() -> NativePinWrappedVaultKeysV1 {
        NativePinWrappedVaultKeysV1::new(
            fake_envelope(NativeSecureCompartment::RecoveryRoot, ROOT_SALT, ROOT_NONCE),
            fake_envelope(
                NativeSecureCompartment::DeviceKey,
                OPERATIONAL_SALT,
                OPERATIONAL_NONCE,
            ),
        )
        .expect("structurally valid two-compartment vault")
    }

    fn real_vault() -> NativePinWrappedVaultKeysV1 {
        let root_factor = secret(0xa1, PHASE15Q_PLATFORM_FACTOR_BYTES);

        let operational_factor = secret(0xb2, PHASE15Q_PLATFORM_FACTOR_BYTES);

        let root_vmk = secret(0xc3, PHASE15Q_VAULT_MASTER_KEY_BYTES);

        let operational_vmk = secret(0xd4, PHASE15Q_VAULT_MASTER_KEY_BYTES);

        let root = wrap_native_compartment_vmk(
            NativeSecureCompartment::RecoveryRoot,
            PIN,
            &root_factor,
            &ROOT_SALT,
            &ROOT_NONCE,
            &root_vmk,
        )
        .expect("wrap recovery-root VMK");

        let operational = wrap_native_compartment_vmk(
            NativeSecureCompartment::DeviceKey,
            PIN,
            &operational_factor,
            &OPERATIONAL_SALT,
            &OPERATIONAL_NONCE,
            &operational_vmk,
        )
        .expect("wrap operational VMK");

        NativePinWrappedVaultKeysV1::new(root, operational)
            .expect("real two-compartment wrapped vault")
    }

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    #[test]
    fn phase15q_posture_preserves_protocol_and_kms_boundaries() {
        let posture = native_vault_crypto_posture();

        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE15Q_LABEL,);

        assert_eq!(posture.canonical_owner, "svc-passport",);

        assert!(posture.phase0i_header_encoding_preserved);

        assert!(posture.phase0i_fixture_salt_is_contract_only);

        assert!(posture.phase6_runtime_salt_length_preserved);

        assert!(posture.argon2id_pin_key_added);

        assert!(posture.hkdf_sha256_kek_added);

        assert!(posture.xchacha20poly1305_vmk_wrap_added);

        assert!(posture.same_pin_may_protect_both_compartments);

        assert!(posture.platform_factor_required_for_each_wrap);

        assert!(posture.ordinary_unlock_is_operational_only);

        assert!(!posture.public_root_unlock_api_added);

        assert!(!posture.ron_kms_key_lifecycle_touched);

        assert!(!posture.ed25519_signing_or_verification_added);

        assert!(!posture.pin_persisted);
        assert!(!posture.platform_sealer_called);
        assert!(!posture.filesystem_io_added);
        assert!(!posture.tauri_command_added);

        assert!(!posture.frontend_secret_dto_added);

        assert!(!posture.wallet_or_ledger_mutation_added);
    }

    #[test]
    fn phase15q_runtime_profile_reconciles_phase0i_and_phase6() {
        assert_eq!(
            PHASE15Q_AUTHENTICATED_HEADER_ENCODING,
            "pipe-delimited-canonical-v1",
        );

        assert_eq!(
            PHASE15Q_AUTHENTICATED_HEADER_INPUT_FORMAT,
            "utf8:domain|version|compartment_kind|kdf|kdf_version|kdf_params|salt_hex|aead|nonce_hex|compartment_purpose|domain_tag",
        );

        assert_eq!(
            PHASE15Q_AUTHENTICATED_HEADER_DOMAIN,
            "rustyonions.native-passport.vault-header.v1",
        );

        assert_eq!(PHASE6A_KDF_SALT_LEN, 16,);

        assert_eq!(PHASE15Q_ROOT_KDF_PROFILE.memory_mib, 64,);

        assert_eq!(PHASE15Q_OPERATIONAL_KDF_PROFILE.memory_mib, 32,);

        assert_eq!(PHASE15Q_ROOT_KDF_PROFILE.time_cost, 3,);

        assert_eq!(PHASE15Q_OPERATIONAL_KDF_PROFILE.time_cost, 3,);

        let root_header = String::from_utf8(encode_native_vault_authenticated_header(
            NativeSecureCompartment::RecoveryRoot,
            &ROOT_SALT,
            &ROOT_NONCE,
        ))
        .expect("root authenticated header UTF-8");

        assert!(
            root_header.starts_with(
                "rustyonions.native-passport.vault-header.v1|1|passport_root_compartment|Argon2id|v1|m=64,t=3,p=1,dk=32|",
            )
        );

        assert!(
            root_header.ends_with(
                "|future encrypted custody for recovery phrase and Passport root key material|rustyonions.native-passport.vault-root-compartment.v1",
            )
        );

        let operational_info =
            String::from_utf8(native_vault_kek_info(NativeSecureCompartment::DeviceKey))
                .expect("operational KEK info UTF-8");

        assert!(operational_info.contains("rustyonions.native-passport.vault-kek.v1",));

        assert!(operational_info.contains("device_compartment",));
    }

    #[test]
    fn phase15q_same_pin_wraps_separate_compartments_but_ordinary_unlock_is_operational_only() {
        let vault = real_vault();

        let encoded =
            encode_native_pin_wrapped_vault_keys(&vault).expect("encode wrapped vault keys");

        let decoded =
            decode_native_pin_wrapped_vault_keys(&encoded).expect("decode wrapped vault keys");

        let operational_factor = secret(0xb2, PHASE15Q_PLATFORM_FACTOR_BYTES);

        let operational_vmk =
            unlock_native_operational_vmk(decoded.operational(), PIN, &operational_factor)
                .expect("ordinary operational unlock");

        assert_eq!(
            operational_vmk.as_slice(),
            &[0xd4; PHASE15Q_VAULT_MASTER_KEY_BYTES],
        );

        assert_eq!(
            unlock_native_operational_vmk(decoded.recovery_root(), PIN, &operational_factor,),
            Err(NativeVaultCryptoError::OperationalCompartmentRequired,),
        );

        let wrong_factor = secret(0xee, PHASE15Q_PLATFORM_FACTOR_BYTES);

        assert_eq!(
            unlock_native_operational_vmk(decoded.operational(), PIN, &wrong_factor,),
            Err(NativeVaultCryptoError::AuthenticationFailed,),
        );

        assert_eq!(
            unlock_native_operational_vmk(
                decoded.operational(),
                b"incorrect-local-pin",
                &operational_factor,
            ),
            Err(NativeVaultCryptoError::AuthenticationFailed,),
        );

        let mut tampered_bytes = encoded.as_slice().to_vec();

        let last = tampered_bytes.last_mut().expect("encoded vault byte");

        *last ^= 0x01;

        let tampered =
            NativeEncryptedVaultV1::new(tampered_bytes).expect("bounded tampered encrypted vault");

        let tampered = decode_native_pin_wrapped_vault_keys(&tampered)
            .expect("ciphertext tamper preserves structural codec");

        assert_eq!(
            unlock_native_operational_vmk(tampered.operational(), PIN, &operational_factor,),
            Err(NativeVaultCryptoError::AuthenticationFailed,),
        );
    }

    #[test]
    fn phase15q_codec_rejects_header_tamper_truncation_and_trailing_bytes() {
        let vault = fake_vault();

        let encoded =
            encode_native_pin_wrapped_vault_keys(&vault).expect("encode fake wrapped vault keys");

        let header = vault.operational().authenticated_header();

        let header_offset = encoded
            .as_slice()
            .windows(header.len())
            .position(|window| window == header)
            .expect("operational authenticated header in encoded vault");

        let mut header_tampered = encoded.as_slice().to_vec();

        header_tampered[header_offset] ^= 0x01;

        let header_tampered =
            NativeEncryptedVaultV1::new(header_tampered).expect("bounded header-tampered vault");

        assert_eq!(
            decode_native_pin_wrapped_vault_keys(&header_tampered,),
            Err(NativeVaultCryptoError::InvalidAuthenticatedHeader,),
        );

        let truncated =
            NativeEncryptedVaultV1::new(encoded.as_slice()[..encoded.len() - 1].to_vec())
                .expect("bounded truncated vault");

        assert_eq!(
            decode_native_pin_wrapped_vault_keys(&truncated,),
            Err(NativeVaultCryptoError::InvalidEncodedVault,),
        );

        let mut trailing = encoded.as_slice().to_vec();

        trailing.push(0);

        let trailing =
            NativeEncryptedVaultV1::new(trailing).expect("bounded vault with trailing byte");

        assert_eq!(
            decode_native_pin_wrapped_vault_keys(&trailing,),
            Err(NativeVaultCryptoError::InvalidEncodedVault,),
        );
    }

    #[test]
    fn phase15q_rejects_unsafe_lengths_and_compartment_material_reuse_before_crypto() {
        let factor = secret(0xa1, PHASE15Q_PLATFORM_FACTOR_BYTES);

        let vmk = secret(0xc3, PHASE15Q_VAULT_MASTER_KEY_BYTES);

        assert_eq!(
            wrap_native_compartment_vmk(
                NativeSecureCompartment::RecoveryRoot,
                b"12345",
                &factor,
                &ROOT_SALT,
                &ROOT_NONCE,
                &vmk,
            ),
            Err(NativeVaultCryptoError::InvalidPinLength {
                actual: 5,
                minimum: 6,
                maximum: 64,
            },),
        );

        let short_factor = secret(0xa1, 31);

        assert_eq!(
            wrap_native_compartment_vmk(
                NativeSecureCompartment::RecoveryRoot,
                PIN,
                &short_factor,
                &ROOT_SALT,
                &ROOT_NONCE,
                &vmk,
            ),
            Err(NativeVaultCryptoError::InvalidPlatformFactorLength {
                actual: 31,
                expected: 32,
            },),
        );

        let short_vmk = secret(0xc3, 31);

        assert_eq!(
            wrap_native_compartment_vmk(
                NativeSecureCompartment::RecoveryRoot,
                PIN,
                &factor,
                &ROOT_SALT,
                &ROOT_NONCE,
                &short_vmk,
            ),
            Err(NativeVaultCryptoError::InvalidVaultMasterKeyLength {
                actual: 31,
                expected: 32,
            },),
        );

        assert_eq!(
            NativePinWrappedVaultKeysV1::new(
                fake_envelope(NativeSecureCompartment::RecoveryRoot, ROOT_SALT, ROOT_NONCE,),
                fake_envelope(
                    NativeSecureCompartment::DeviceKey,
                    ROOT_SALT,
                    OPERATIONAL_NONCE,
                ),
            ),
            Err(NativeVaultCryptoError::CompartmentMaterialReuse,),
        );
    }

    #[test]
    fn phase15q_debug_and_source_boundaries_are_redacted_and_kms_safe() {
        let vault = fake_vault();
        let debug = format!("{vault:?}");

        assert!(debug.contains("REDACTED_AUTHENTICATED_CIPHERTEXT"));

        assert!(!debug.contains("correct-horse-battery-staple"));

        assert!(!debug.contains("4444444444444444"));

        let source =
            fs::read_to_string(repo_file("src/native/vault_crypto.rs")).expect("Phase 15Q source");

        let cargo = fs::read_to_string(repo_file("Cargo.toml")).expect("svc-passport Cargo.toml");

        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        for required in [
            "Algorithm::Argon2id",
            "Hkdf::<Sha256>",
            "XChaCha20Poly1305",
            "pipe-delimited-canonical-v1",
            "unlock_native_operational_vmk",
            "OperationalCompartmentRequired",
        ] {
            assert!(
                source.contains(required),
                "Phase 15Q source missing {required}",
            );
        }

        for forbidden in [
            "ron_kms::",
            "ed25519_dalek",
            "SigningKey",
            "VerifyingKey",
            "create_ed25519",
            "verify_batch",
            "#[tauri::command]",
            "tauri::",
            "std::fs::",
            "tokio::fs",
            "File::create",
            "OpenOptions::",
            "localStorage",
            "sessionStorage",
            "indexedDB",
            "capability_token",
            "wallet.spend(",
            "ledger.write(",
            "println!",
            "eprintln!",
            "tracing::",
        ] {
            assert!(
                !source.contains(forbidden),
                "Phase 15Q source contains forbidden surface {forbidden}",
            );
        }

        assert!(!cargo.contains("ron-kms"));

        assert!(cargo.contains("\"dep:argon2\""));

        assert!(cargo.contains("\"dep:chacha20poly1305\""));

        assert!(cargo.contains("\"dep:hkdf\""));

        assert!(cargo.contains("\"dep:sha2\""));

        assert!(native_mod.contains("pub mod vault_crypto;"));

        assert!(native_mod.contains("pub use vault_crypto::{"));
    }
}
