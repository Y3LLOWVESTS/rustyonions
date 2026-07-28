#[cfg(not(feature = "native-passport"))]
#[test]
fn phase2b_ron_naming_username_reuse_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native Passport username reuse"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use ron_naming::{HandleV1 as RonNamingHandleV1, UsernameV1 as RonNamingUsernameV1};
    use svc_passport::native::{
        native_passport_username_reuse_posture, parse_optional_username_handle, HandleV1,
        NativePassportSurface, UsernameV1, NATIVE_PASSPORT_USERNAME_PHASE2B_LABEL,
        OPTIONAL_USERNAME_HANDLE_STATUS, PHASE2B_ENABLED_SURFACES,
        PHASE2B_FORBIDDEN_HANDLE_MEANINGS,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    #[test]
    fn phase2b_username_reuse_posture_is_locked() {
        let posture = native_passport_username_reuse_posture();

        assert_eq!(
            NATIVE_PASSPORT_USERNAME_PHASE2B_LABEL,
            "NATIVE_PASSPORT_PHASE2B_RON_NAMING_USERNAME_REUSE"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_USERNAME_PHASE2B_LABEL);
        assert_eq!(posture.canonical_username_owner, "ron-naming");
        assert_eq!(
            posture.optional_handle_status,
            OPTIONAL_USERNAME_HANDLE_STATUS
        );
        assert!(posture.ron_naming_reuse_enabled);
        assert_eq!(posture.username_min_len, 3);
        assert_eq!(posture.username_max_len, 32);
        assert!(posture.reserved_label_count >= 20);
    }

    #[test]
    fn phase2b_surface_layer_extends_phase1e_without_back_mutating_it() {
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
    }

    #[test]
    fn svc_passport_reexports_ron_naming_username_and_handle_types() {
        let ron_username = RonNamingUsernameV1::parse("Creator_Seven").unwrap();
        let svc_username = UsernameV1::parse("Creator_Seven").unwrap();

        let ron_handle = RonNamingHandleV1::parse("@Creator_Seven").unwrap();
        let svc_handle = HandleV1::parse("@Creator_Seven").unwrap();

        assert_eq!(ron_username.as_str(), svc_username.as_str());
        assert_eq!(ron_handle.as_str(), svc_handle.as_str());
        assert_eq!(svc_username.as_str(), "creator_seven");
        assert_eq!(svc_handle.as_str(), "@creator_seven");
    }

    #[test]
    fn optional_username_handle_parser_reuses_ron_naming_rules() {
        assert_eq!(parse_optional_username_handle(None).unwrap(), None);

        let parsed = parse_optional_username_handle(Some("Creator_Seven"))
            .unwrap()
            .expect("handle should be present");

        assert_eq!(parsed.as_str(), "@creator_seven");

        for rejected in ["admin", "@site", "@wallet", "John Smith", "creator__one"] {
            assert!(parse_optional_username_handle(Some(rejected)).is_err());
            assert!(RonNamingHandleV1::parse(rejected).is_err());
        }
    }

    #[test]
    fn optional_handles_never_gain_identity_wallet_ledger_or_secret_meaning() {
        let posture = native_passport_username_reuse_posture();

        assert!(!posture.runtime_authority_changed);
        assert!(!posture.native_secret_implementation_added);
        assert!(!posture.routes_added);
        assert!(!posture.signing_or_verification_runtime_added);
        assert!(!posture.vault_runtime_added);
        assert!(!posture.capability_issuance_added);
        assert!(!posture.wallet_or_ledger_mutation_added);

        for forbidden in [
            "legal name",
            "real human name",
            "Passport ID",
            "Device ID",
            "Challenge ID",
            "wallet spend authority",
            "ledger mutation authority",
            "capability issuance authority",
            "secret recovery material",
        ] {
            assert!(
                posture.forbidden_handle_meanings.contains(&forbidden),
                "optional handle posture must forbid {forbidden}"
            );
            assert!(
                PHASE2B_FORBIDDEN_HANDLE_MEANINGS.contains(&forbidden),
                "forbidden handle meanings constant must include {forbidden}"
            );
        }
    }

    #[test]
    fn phase2b_svc_native_username_source_uses_ron_naming_not_local_rules() {
        let source = fs::read_to_string(repo_file("src/native/username.rs"))
            .expect("native username source should be readable");
        let cargo_toml =
            fs::read_to_string(repo_file("Cargo.toml")).expect("Cargo.toml should be readable");

        assert!(source.contains("ron_naming::UsernameV1"));
        assert!(source.contains("ron_naming::HandleV1"));
        assert!(source.contains("ron_naming::RESERVED_USERNAME_LABELS"));

        assert!(cargo_toml.contains("ron-naming"));
        assert!(cargo_toml.contains("native-passport = ["));
        assert!(
            !cargo_toml
                .lines()
                .find(|line| line.trim_start().starts_with("default"))
                .unwrap_or("")
                .contains("native-passport"),
            "default feature set must not enable native-passport"
        );
    }
}
