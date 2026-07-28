#[cfg(not(feature = "native-passport"))]
#[test]
fn native_passport_phase1e_status_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not enable native-passport redacted status implicitly"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use svc_passport::native::{
        native_passport_redacted_status, redacted_value_for, NativePassportRedactedStatusReadiness,
        NativePassportSurface, NATIVE_PASSPORT_PHASE1E_LABEL,
        NATIVE_PASSPORT_REDACTED_STATUS_SCHEMA_V1, NATIVE_PASSPORT_REDACTED_VALUE,
        PHASE1E_REDACTED_FIELD_NAMES,
    };

    const PASSPORT_ID: &str = "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
    const DEVICE_ID: &str =
        "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";
    const CHALLENGE_ID: &str =
        "challenge:v1:b3:58577b7fde04e28ccce564ea08ed46575bb00f60924a9c1cf8d7546497b1148e";
    const DEVICE_PUBLIC_KEY: &str =
        "2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";
    const SIGNATURE_PLACEHOLDER: &str = "PHASE1_PENDING_ED25519_DEVICE_SIGNATURE";

    #[test]
    fn phase1e_redacted_status_schema_and_label_are_locked() {
        let status = native_passport_redacted_status();

        assert_eq!(
            NATIVE_PASSPORT_PHASE1E_LABEL,
            "NATIVE_PASSPORT_PHASE1E_NATIVE_STATUS_REDACTION"
        );
        assert_eq!(
            NATIVE_PASSPORT_REDACTED_STATUS_SCHEMA_V1,
            "svc-passport.native-passport.redacted-status.v1"
        );
        assert_eq!(status.schema, NATIVE_PASSPORT_REDACTED_STATUS_SCHEMA_V1);
        assert_eq!(status.phase_label, NATIVE_PASSPORT_PHASE1E_LABEL);
        assert_eq!(status.owner, "svc-passport");
        assert_eq!(status.feature_name, "native-passport");
        assert_eq!(
            status.readiness,
            NativePassportRedactedStatusReadiness::ContractsReadyNoRuntimeAuthority
        );
    }

    #[test]
    fn phase1e_redacted_status_reports_surfaces_without_runtime_authority() {
        let status = native_passport_redacted_status();

        assert!(status
            .enabled_surfaces
            .contains(&NativePassportSurface::FeaturePosture));
        assert!(status
            .enabled_surfaces
            .contains(&NativePassportSurface::Phase0Contracts));
        assert!(status
            .enabled_surfaces
            .contains(&NativePassportSurface::NativeDtoTypes));
        assert!(status
            .enabled_surfaces
            .contains(&NativePassportSurface::NativeStatusRedaction));

        assert_eq!(status.dto_type_count, 8);
        assert_eq!(status.read_only_scope_count, 7);
        assert_eq!(status.unsafe_scope_count, 8);
        assert_eq!(status.service_kms_mode, "development_in_process");

        assert!(!status.runtime_authority_changed);
        assert!(!status.native_secret_implementation_added);
        assert!(!status.routes_added);
        assert!(!status.signing_or_verification_runtime_added);
        assert!(!status.vault_runtime_added);
        assert!(!status.capability_issuance_added);
        assert!(!status.wallet_or_ledger_mutation_added);
    }

    #[test]
    fn phase1e_redaction_field_list_covers_ids_keys_signatures_vaults_and_authority() {
        let status = native_passport_redacted_status();

        assert_eq!(status.redacted_field_names, PHASE1E_REDACTED_FIELD_NAMES);

        for field in [
            "passport_id",
            "device_id",
            "challenge_id",
            "root_public_key_hex",
            "device_public_key_hex",
            "authorization_transcript_b3_hex",
            "challenge_proof_transcript_b3_hex",
            "request_transcript_b3_hex",
            "capability_hash_hex",
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
            "root_signature_hex",
            "device_signature_hex",
            "device_request_signature_hex",
        ] {
            assert!(
                status.redacted_field_names.contains(&field),
                "redacted status field list must include {field}"
            );
            assert_eq!(
                redacted_value_for(field),
                Some(NATIVE_PASSPORT_REDACTED_VALUE)
            );
        }

        assert_eq!(redacted_value_for("safe_phase_label"), None);
    }

    #[test]
    fn phase1e_status_debug_output_does_not_leak_locked_fixture_values() {
        let status = native_passport_redacted_status();
        let debug = format!("{status:?}");

        for forbidden_value in [
            PASSPORT_ID,
            DEVICE_ID,
            CHALLENGE_ID,
            DEVICE_PUBLIC_KEY,
            SIGNATURE_PLACEHOLDER,
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
        assert!(debug.contains("development_in_process"));
    }

    #[test]
    fn phase1e_status_is_inspection_only_and_adds_no_route_or_runtime_surface() {
        let status = native_passport_redacted_status();

        assert!(!status.routes_added);
        assert!(!status.signing_or_verification_runtime_added);
        assert!(!status.vault_runtime_added);
        assert!(!status.capability_issuance_added);
        assert!(!status.wallet_or_ledger_mutation_added);
        assert!(!status.runtime_authority_changed);
        assert!(!status.native_secret_implementation_added);
    }
}
