#[cfg(not(feature = "native-passport"))]
#[test]
fn phase2d_proto_naming_acceptance_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native Passport proto/naming acceptance"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use ron_naming::{
        HandleV1 as RonNamingHandleV1, UsernameV1 as RonNamingUsernameV1,
        NATIVE_PASSPORT_PHASE2B_LABEL as RON_NAMING_PHASE2B_LABEL,
    };
    use ron_proto::{
        ChallengeIdV1 as RonProtoChallengeIdV1, DeviceIdV1 as RonProtoDeviceIdV1,
        PassportIdV1 as RonProtoPassportIdV1,
    };
    use svc_passport::{
        native::{
            native_passport_username_reuse_posture, parse_optional_username_handle,
            ChallengeIdV1 as SvcChallengeIdV1, DeviceIdV1 as SvcDeviceIdV1, NativePassportSurface,
            PassportIdV1 as SvcPassportIdV1, NATIVE_PASSPORT_USERNAME_PHASE2B_LABEL,
            PHASE2B_ENABLED_SURFACES,
        },
        profile::{normalize_handle, normalize_username},
    };

    const PHASE2D_LABEL: &str = "NATIVE_PASSPORT_PHASE2D_PROTO_NAMING_ACCEPTANCE";

    const LOCKED_PASSPORT_ID: &str = "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
    const LOCKED_DEVICE_ID: &str =
        "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";
    const LOCKED_CHALLENGE_ID: &str =
        "challenge:v1:b3:58577b7fde04e28ccce564ea08ed46575bb00f60924a9c1cf8d7546497b1148e";

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    #[test]
    fn phase2d_acceptance_label_is_locked() {
        assert_eq!(
            PHASE2D_LABEL,
            "NATIVE_PASSPORT_PHASE2D_PROTO_NAMING_ACCEPTANCE"
        );
    }

    #[test]
    fn phase2d_accepts_phase2a_proto_id_reuse_contract() {
        let ron_passport = RonProtoPassportIdV1::parse(LOCKED_PASSPORT_ID).unwrap();
        let svc_passport = SvcPassportIdV1::parse(LOCKED_PASSPORT_ID).unwrap();

        let ron_device = RonProtoDeviceIdV1::parse(LOCKED_DEVICE_ID).unwrap();
        let svc_device = SvcDeviceIdV1::parse(LOCKED_DEVICE_ID).unwrap();

        let ron_challenge = RonProtoChallengeIdV1::parse(LOCKED_CHALLENGE_ID).unwrap();
        let svc_challenge = SvcChallengeIdV1::parse(LOCKED_CHALLENGE_ID).unwrap();

        assert_eq!(svc_passport.as_str(), ron_passport.as_str());
        assert_eq!(svc_device.as_str(), ron_device.as_str());
        assert_eq!(svc_challenge.as_str(), ron_challenge.as_str());

        for bad in [
            "passport:v1:main:ed25519:b3:ACC2761E583FAFC93CBB880BEF1BD7285F43B3BBF326B9E185B226C5533CB7DF",
            "passport:v1:main:ed25519:sha256:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df",
            "device:v1:ed25519:b3:short",
            "challenge:v1:b3:short",
        ] {
            assert!(RonProtoPassportIdV1::parse(bad).is_err());
            assert!(SvcPassportIdV1::parse(bad).is_err());
        }
    }

    #[test]
    fn phase2d_accepts_phase2b_ron_naming_reuse_contract() {
        assert_eq!(
            NATIVE_PASSPORT_USERNAME_PHASE2B_LABEL,
            RON_NAMING_PHASE2B_LABEL
        );

        let ron_username = RonNamingUsernameV1::parse("Creator_Seven").unwrap();
        let svc_handle = parse_optional_username_handle(Some("Creator_Seven"))
            .unwrap()
            .expect("optional handle should be present");

        assert_eq!(ron_username.as_str(), "creator_seven");
        assert_eq!(svc_handle.as_str(), "@creator_seven");

        let ron_handle = RonNamingHandleV1::parse("@Creator_Seven").unwrap();
        assert_eq!(svc_handle.as_str(), ron_handle.as_str());

        for rejected in [
            "admin",
            "@site",
            "ledger",
            "@wallet",
            "creator__one",
            "John Smith",
        ] {
            assert!(RonNamingUsernameV1::parse(rejected).is_err());
            assert!(RonNamingHandleV1::parse(rejected).is_err());
            assert!(parse_optional_username_handle(Some(rejected)).is_err());
        }
    }

    #[test]
    fn phase2d_accepts_phase2c_profile_reconciliation_contract() {
        for accepted in ["SkinnyCrabby", "@SkinnyCrabby", "creator_007"] {
            let profile_username = normalize_username(accepted).unwrap();
            let naming_username = RonNamingUsernameV1::parse(accepted).unwrap();

            assert_eq!(profile_username, naming_username.as_str());

            let profile_handle = normalize_handle(accepted).unwrap();
            let naming_handle = RonNamingHandleV1::parse(accepted).unwrap();

            assert_eq!(profile_handle, naming_handle.as_str());
        }

        for rejected in [
            "creator__one",
            "ledger",
            "@ledger",
            "site",
            "@site",
            "wallet",
        ] {
            assert!(
                normalize_username(rejected).is_err(),
                "profile username must reject {rejected}"
            );
            assert!(
                normalize_handle(rejected).is_err(),
                "profile handle must reject {rejected}"
            );
        }

        assert_eq!(
            normalize_username("crab-link").expect("legacy profile route allows internal hyphen"),
            "crab-link"
        );
        assert!(
            RonNamingUsernameV1::parse("crab-link").is_err(),
            "native ron-naming username remains stricter than legacy profile routes"
        );
        assert!(
            parse_optional_username_handle(Some("crab-link")).is_err(),
            "native optional handle parser keeps ron-naming behavior"
        );
    }

    #[test]
    fn phase2d_feature_surfaces_are_phase2_complete_without_route_or_runtime_authority() {
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

        let posture = native_passport_username_reuse_posture();

        assert_eq!(posture.canonical_username_owner, "ron-naming");
        assert!(posture.ron_naming_reuse_enabled);
        assert!(!posture.runtime_authority_changed);
        assert!(!posture.native_secret_implementation_added);
        assert!(!posture.routes_added);
        assert!(!posture.signing_or_verification_runtime_added);
        assert!(!posture.vault_runtime_added);
        assert!(!posture.capability_issuance_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
    }

    #[test]
    fn phase2d_cargo_feature_matrix_keeps_canonical_proto_default_safe() {
        let cargo_toml =
            fs::read_to_string(repo_file("Cargo.toml")).expect("Cargo.toml should be readable");

        assert!(cargo_toml.contains("ron-proto"));
        assert!(cargo_toml.contains("ron-naming"));
        assert!(cargo_toml.contains("native-passport = ["));

        assert!(
            cargo_toml.contains(r#"ron-proto = { path = "../ron-proto" }"#),
            "ron-proto must remain available to always-compiled native_plan canonical constants",
        );

        assert!(
            !cargo_toml.contains(r#"ron-proto = { path = "../ron-proto", optional = true }"#),
            "ron-proto can no longer be optional because native_plan is compiled without native-passport",
        );

        assert!(
            !cargo_toml.contains(r#""dep:ron-proto""#),
            "native-passport must not redundantly feature-enable a non-optional ron-proto dependency",
        );

        assert!(
            cargo_toml.contains(r#""dep:ron-naming""#),
            "ron-naming remains native-passport feature gated",
        );

        assert!(
            cargo_toml.contains(r#""dep:ron-auth""#),
            "ron-auth signing/transcript integration remains native-passport feature gated",
        );

        let default_feature_line = cargo_toml
            .lines()
            .find(|line| line.trim_start().starts_with("default"))
            .unwrap_or("");

        assert!(
            !default_feature_line.contains("native-passport"),
            "default feature set must not enable native-passport",
        );
    }

    #[test]
    fn phase2d_sources_keep_proto_and_naming_rules_out_of_routes_and_runtime_authority() {
        let dto_source = fs::read_to_string(repo_file("src/native/dto.rs")).unwrap();
        let username_source = fs::read_to_string(repo_file("src/native/username.rs")).unwrap();
        let profile_source = fs::read_to_string(repo_file("src/profile.rs")).unwrap();

        assert!(dto_source.contains("ron_proto::PassportIdV1::parse"));
        assert!(dto_source.contains("ron_proto::DeviceIdV1::parse"));
        assert!(dto_source.contains("ron_proto::ChallengeIdV1::parse"));

        assert!(username_source.contains("ron_naming::UsernameV1"));
        assert!(username_source.contains("ron_naming::HandleV1"));
        assert!(profile_source.contains("\"ledger\""));
        assert!(profile_source.contains("contains(\"__\")"));

        for forbidden_runtime_pattern in [
            "fn issue_capability",
            "issue_capability(",
            "capability_issuer:",
            "pub capability_issuer",
            "vault_master_key:",
            "pub vault_master_key",
            "let vault_master_key",
            "derived_vault_key:",
            "pub derived_vault_key",
            "let derived_vault_key",
            "root_private_key:",
            "pub root_private_key",
            "let root_private_key",
            "device_private_key:",
            "pub device_private_key",
            "let device_private_key",
            "wallet.spend(",
            "ledger.write(",
            "mint_roc(",
            "burn_roc(",
        ] {
            assert!(
                !dto_source.contains(forbidden_runtime_pattern),
                "native DTO source must not gain runtime authority pattern {forbidden_runtime_pattern}"
            );
            assert!(
                !username_source.contains(forbidden_runtime_pattern),
                "native username source must not gain runtime authority pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
