use std::{fs, path::PathBuf};

const PHASE1F_LABEL: &str = "NATIVE_PASSPORT_PHASE1F_FEATURE_MATRIX_ACCEPTANCE";

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn cargo_toml_text() -> String {
    fs::read_to_string(repo_file("Cargo.toml")).expect("svc-passport Cargo.toml should be readable")
}

fn features_section(text: &str) -> &str {
    let Some(start) = text.find("[features]") else {
        panic!("Cargo.toml must contain [features] section");
    };
    let rest = &text[start..];
    if let Some(next_table_offset) =
        rest.lines()
            .enumerate()
            .skip(1)
            .find_map(|(line_index, line)| {
                let trimmed = line.trim();
                (trimmed.starts_with('[') && trimmed.ends_with(']')).then_some(line_index)
            })
    {
        let byte_offset = rest
            .lines()
            .take(next_table_offset)
            .map(|line| line.len() + 1)
            .sum::<usize>();
        &rest[..byte_offset]
    } else {
        rest
    }
}

#[test]
fn phase1f_acceptance_label_is_locked() {
    assert_eq!(
        PHASE1F_LABEL,
        "NATIVE_PASSPORT_PHASE1F_FEATURE_MATRIX_ACCEPTANCE"
    );
}

#[test]
fn cargo_feature_matrix_declares_native_passport_without_default_enablement() {
    let cargo_toml = cargo_toml_text();
    let features = features_section(&cargo_toml);

    assert!(
        features
            .lines()
            .any(|line| line.trim_start().starts_with("native-passport = [")),
        "svc-passport must declare a native-passport feature"
    );

    if let Some(default_line) = features
        .lines()
        .find(|line| line.trim_start().starts_with("default"))
    {
        assert!(
            !default_line.contains("native-passport"),
            "default feature set must not implicitly enable native-passport"
        );
    }
}

#[cfg(not(feature = "native-passport"))]
#[test]
fn default_build_keeps_native_passport_surface_feature_gated() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport test build must not enable native-passport"
    );

    let lib_source =
        fs::read_to_string(repo_file("src/lib.rs")).expect("lib.rs should be readable");
    assert!(
        lib_source.contains("#[cfg(feature = \"native-passport\")]"),
        "native module must remain feature-gated in lib.rs"
    );
    assert!(
        lib_source.contains("pub mod native;"),
        "native module declaration should exist behind cfg gate"
    );
}

#[cfg(feature = "native-passport")]
mod feature_matrix {
    use svc_passport::{
        kms::{
            service_kms_injection_posture, ServiceKmsConstructionMode,
            NATIVE_PASSPORT_PHASE1D_LABEL,
        },
        native::{
            native_passport_dto_posture, native_passport_feature_posture,
            native_passport_redacted_status, redacted_value_for, B3DigestHex,
            DeviceAuthorizationDraftV1, DeviceClass, DeviceIdV1, Ed25519PublicKeyHex,
            NativePassportRedactedStatusReadiness, NativePassportScope, NativePassportSurface,
            PassportIdV1, NATIVE_PASSPORT_FEATURE_NAME, NATIVE_PASSPORT_PHASE1A_LABEL,
            NATIVE_PASSPORT_PHASE1B_LABEL, NATIVE_PASSPORT_PHASE1C_LABEL,
            NATIVE_PASSPORT_PHASE1E_LABEL, NATIVE_PASSPORT_READ_ONLY_SCOPE_CEILING,
            NATIVE_PASSPORT_REDACTED_STATUS_SCHEMA_V1, NATIVE_PASSPORT_REDACTED_VALUE,
            NATIVE_PASSPORT_UNSAFE_SCOPE_STRINGS, PHASE1B_FORBIDDEN_DTO_FIELDS,
            PHASE1C_ENABLED_SURFACES, PHASE1E_ENABLED_SURFACES, PHASE1E_REDACTED_FIELD_NAMES,
        },
    };

    const PASSPORT_ID: &str = "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
    const DEVICE_ID: &str =
        "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";
    const ROOT_PUBLIC_KEY: &str =
        "3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909";
    const DEVICE_PUBLIC_KEY: &str =
        "2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";
    const NONCE_HEX: &str = "000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f";

    #[test]
    fn feature_matrix_phase_labels_are_locked() {
        assert_eq!(
            NATIVE_PASSPORT_PHASE1A_LABEL,
            "NATIVE_PASSPORT_PHASE1A_FEATURE_ISOLATION_NATIVE_MODULE_GATES"
        );
        assert_eq!(
            NATIVE_PASSPORT_PHASE1B_LABEL,
            "NATIVE_PASSPORT_PHASE1B_NATIVE_DTO_TYPES"
        );
        assert_eq!(
            NATIVE_PASSPORT_PHASE1C_LABEL,
            "NATIVE_PASSPORT_PHASE1C_NATIVE_MODULE_DTO_POSTURE"
        );
        assert_eq!(
            NATIVE_PASSPORT_PHASE1D_LABEL,
            "NATIVE_PASSPORT_PHASE1D_SERVER_KMS_DEV_ISOLATION"
        );
        assert_eq!(
            NATIVE_PASSPORT_PHASE1E_LABEL,
            "NATIVE_PASSPORT_PHASE1E_NATIVE_STATUS_REDACTION"
        );
    }

    #[test]
    fn feature_matrix_surfaces_are_layered_without_back_mutating_phase1c() {
        assert_eq!(
            PHASE1C_ENABLED_SURFACES,
            &[
                NativePassportSurface::FeaturePosture,
                NativePassportSurface::Phase0Contracts,
                NativePassportSurface::NativeDtoTypes,
            ]
        );

        assert_eq!(
            PHASE1E_ENABLED_SURFACES,
            &[
                NativePassportSurface::FeaturePosture,
                NativePassportSurface::Phase0Contracts,
                NativePassportSurface::NativeDtoTypes,
                NativePassportSurface::NativeStatusRedaction,
            ]
        );

        let feature = native_passport_feature_posture();
        let dto = native_passport_dto_posture();
        let status = native_passport_redacted_status();

        assert_eq!(feature.feature_name, NATIVE_PASSPORT_FEATURE_NAME);
        assert_eq!(dto.enabled_surfaces, PHASE1C_ENABLED_SURFACES);
        assert_eq!(status.enabled_surfaces, PHASE1E_ENABLED_SURFACES);
    }

    #[test]
    fn feature_matrix_dto_types_validate_locked_phase0_values() {
        let passport_id = PassportIdV1::parse(PASSPORT_ID).expect("passport id parses");
        let device_id = DeviceIdV1::parse(DEVICE_ID).expect("device id parses");
        let root_public_key = Ed25519PublicKeyHex::parse(ROOT_PUBLIC_KEY).unwrap();
        let device_public_key = Ed25519PublicKeyHex::parse(DEVICE_PUBLIC_KEY).unwrap();
        let nonce = B3DigestHex::parse("authorization_nonce_hex", NONCE_HEX).unwrap();

        let dto = DeviceAuthorizationDraftV1::new(
            passport_id,
            device_id,
            root_public_key,
            0,
            device_public_key,
            DeviceClass::TvReadOnly,
            NATIVE_PASSPORT_READ_ONLY_SCOPE_CEILING.to_vec(),
            nonce,
        )
        .expect("bounded DTO should validate");

        assert_eq!(dto.passport_id().as_str(), PASSPORT_ID);
        assert_eq!(dto.device_id().as_str(), DEVICE_ID);
        assert_eq!(dto.root_key_epoch(), 0);
        assert_eq!(dto.device_class(), DeviceClass::TvReadOnly);
        assert_eq!(dto.authorized_scope_ceiling().len(), 7);
        assert_eq!(
            dto.scope_ceiling_csv(),
            "identity.read,catalog.read,content.read,entitlement.read,receipts.read,confirmed_roc.read,capability.revoke_self"
        );
    }

    #[test]
    fn feature_matrix_scope_and_secret_boundaries_are_locked() {
        assert_eq!(NATIVE_PASSPORT_READ_ONLY_SCOPE_CEILING.len(), 7);
        assert_eq!(NATIVE_PASSPORT_UNSAFE_SCOPE_STRINGS.len(), 8);

        for read_scope in NATIVE_PASSPORT_READ_ONLY_SCOPE_CEILING {
            assert_eq!(
                NativePassportScope::parse_read_only(read_scope.as_str()).unwrap(),
                *read_scope
            );
        }

        for unsafe_scope in NATIVE_PASSPORT_UNSAFE_SCOPE_STRINGS {
            assert!(
                NativePassportScope::parse_read_only(unsafe_scope).is_err(),
                "unsafe scope must be rejected: {unsafe_scope}"
            );
        }

        for forbidden_field in [
            "mnemonic_words",
            "bip39_seed",
            "root_private_key",
            "root_signing_seed",
            "device_private_key",
            "pin",
            "vault_master_key",
            "derived_vault_key",
            "platform_device_secret",
            "wallet_spend_authority",
            "ledger_mutation_authority",
            "raw_long_lived_capability",
        ] {
            assert!(
                PHASE1B_FORBIDDEN_DTO_FIELDS.contains(&forbidden_field),
                "Phase 1 forbidden DTO fields must contain {forbidden_field}"
            );
        }
    }

    #[test]
    fn feature_matrix_kms_seam_supports_injection_without_native_secret_runtime() {
        let kms = service_kms_injection_posture();

        assert_eq!(kms.phase_label, NATIVE_PASSPORT_PHASE1D_LABEL);
        assert_eq!(
            kms.default_mode,
            ServiceKmsConstructionMode::DevelopmentInProcess
        );
        assert!(kms.explicit_injection_supported);
        assert!(kms.dev_default_constructor_isolated);

        assert!(!kms.runtime_authority_changed);
        assert!(!kms.native_secret_implementation_added);
        assert!(!kms.native_passport_signing_runtime_added);
        assert!(!kms.vault_runtime_added);
        assert!(!kms.capability_issuance_added);
    }

    #[test]
    fn feature_matrix_redacted_status_exposes_only_safe_status_facts() {
        let status = native_passport_redacted_status();

        assert_eq!(status.schema, NATIVE_PASSPORT_REDACTED_STATUS_SCHEMA_V1);
        assert_eq!(status.phase_label, NATIVE_PASSPORT_PHASE1E_LABEL);
        assert_eq!(status.owner, "svc-passport");
        assert_eq!(status.feature_name, "native-passport");
        assert_eq!(
            status.readiness,
            NativePassportRedactedStatusReadiness::ContractsReadyNoRuntimeAuthority
        );
        assert_eq!(status.dto_type_count, 8);
        assert_eq!(status.read_only_scope_count, 7);
        assert_eq!(status.unsafe_scope_count, 8);
        assert_eq!(status.service_kms_mode, "development_in_process");

        assert_eq!(status.redacted_field_names, PHASE1E_REDACTED_FIELD_NAMES);

        for field in [
            "passport_id",
            "device_id",
            "challenge_id",
            "root_public_key_hex",
            "device_public_key_hex",
            "root_signature_hex",
            "device_signature_hex",
            "device_request_signature_hex",
            "wallet_spend_authority",
            "ledger_mutation_authority",
            "raw_long_lived_capability",
        ] {
            assert_eq!(
                redacted_value_for(field),
                Some(NATIVE_PASSPORT_REDACTED_VALUE)
            );
        }

        assert_eq!(redacted_value_for("phase_label"), None);
    }

    #[test]
    fn feature_matrix_all_phase1_postures_keep_runtime_authority_off() {
        let feature = native_passport_feature_posture();
        let dto = native_passport_dto_posture();
        let kms = service_kms_injection_posture();
        let status = native_passport_redacted_status();

        assert!(!feature.runtime_authority_changed);
        assert!(!feature.native_secret_implementation_added);
        assert!(!feature.routes_added);
        assert!(!feature.signing_or_verification_runtime_added);
        assert!(!feature.vault_runtime_added);
        assert!(!feature.capability_issuance_added);

        assert!(!dto.runtime_authority_changed);
        assert!(!dto.native_secret_implementation_added);
        assert!(!dto.routes_added);
        assert!(!dto.signing_or_verification_runtime_added);
        assert!(!dto.vault_runtime_added);
        assert!(!dto.capability_issuance_added);

        assert!(!kms.runtime_authority_changed);
        assert!(!kms.native_secret_implementation_added);
        assert!(!kms.native_passport_signing_runtime_added);
        assert!(!kms.vault_runtime_added);
        assert!(!kms.capability_issuance_added);

        assert!(!status.runtime_authority_changed);
        assert!(!status.native_secret_implementation_added);
        assert!(!status.routes_added);
        assert!(!status.signing_or_verification_runtime_added);
        assert!(!status.vault_runtime_added);
        assert!(!status.capability_issuance_added);
        assert!(!status.wallet_or_ledger_mutation_added);
    }

    #[test]
    fn feature_matrix_redacted_status_debug_does_not_leak_fixture_values() {
        let status = native_passport_redacted_status();
        let debug = format!("{status:?}");

        for forbidden_value in [
            PASSPORT_ID,
            DEVICE_ID,
            ROOT_PUBLIC_KEY,
            DEVICE_PUBLIC_KEY,
            NONCE_HEX,
            "PHASE1_PENDING_ED25519_DEVICE_SIGNATURE",
            "mnemonic words",
            "bip39 seed",
            "root private key",
            "device private key",
            "wallet spend authority",
            "ledger mutation authority",
        ] {
            assert!(
                !debug.contains(forbidden_value),
                "redacted status Debug output must not leak {forbidden_value}"
            );
        }

        assert!(debug.contains("ContractsReadyNoRuntimeAuthority"));
    }
}
