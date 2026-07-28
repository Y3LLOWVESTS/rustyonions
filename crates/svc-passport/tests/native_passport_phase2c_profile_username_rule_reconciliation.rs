#[cfg(not(feature = "native-passport"))]
#[test]
fn phase2c_profile_username_reconciliation_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native Passport ron-naming reconciliation tests"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use ron_naming::{HandleV1 as RonNamingHandleV1, UsernameV1 as RonNamingUsernameV1};
    use svc_passport::{
        native::{
            native_passport_username_reuse_posture, parse_optional_username_handle,
            PHASE2B_FORBIDDEN_HANDLE_MEANINGS,
        },
        profile::{normalize_handle, normalize_username},
    };

    const PHASE2C_LABEL: &str = "NATIVE_PASSPORT_PHASE2C_PROFILE_USERNAME_RULE_RECONCILIATION";

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    #[test]
    fn phase2c_label_is_locked() {
        assert_eq!(
            PHASE2C_LABEL,
            "NATIVE_PASSPORT_PHASE2C_PROFILE_USERNAME_RULE_RECONCILIATION"
        );
    }

    #[test]
    fn profile_username_success_cases_match_ron_naming_canonical_output() {
        for input in [
            "SkinnyCrabby",
            "@SkinnyCrabby",
            "creator_007",
            "@creator_007",
            "Crab_User7",
        ] {
            let profile_username = normalize_username(input).expect("profile username normalizes");
            let naming_username =
                RonNamingUsernameV1::parse(input).expect("ron-naming username normalizes");

            assert_eq!(
                profile_username,
                naming_username.as_str(),
                "profile username normalization must match ron-naming for {input}"
            );

            let profile_handle = normalize_handle(input).expect("profile handle normalizes");
            let naming_handle = RonNamingHandleV1::parse(input).expect("ron-naming handle parses");

            assert_eq!(
                profile_handle,
                naming_handle.as_str(),
                "profile handle normalization must match ron-naming for {input}"
            );
        }
    }

    #[test]
    fn profile_username_rejection_cases_match_ron_naming_boundaries() {
        for input in [
            "",
            "ab",
            "@ab",
            "_creator",
            "creator_",
            "creator__one",
            "John Smith",
            "créator",
            "admin",
            "@admin",
            "root",
            "@root",
            "wallet",
            "@wallet",
            "ledger",
            "@ledger",
            "passport",
            "@passport",
            "site",
            "@site",
        ] {
            assert!(
                normalize_username(input).is_err(),
                "svc-passport profile username must reject {input:?}"
            );
            assert!(
                normalize_handle(input).is_err(),
                "svc-passport profile handle must reject {input:?}"
            );
            assert!(
                RonNamingUsernameV1::parse(input).is_err(),
                "ron-naming username must reject {input:?}"
            );
            assert!(
                RonNamingHandleV1::parse(input).is_err(),
                "ron-naming handle must reject {input:?}"
            );
        }
    }

    #[test]
    fn profile_legacy_hyphen_contract_is_documented_before_route_migration() {
        assert_eq!(
            normalize_username("crab-link")
                .expect("legacy profile route still accepts internal hyphen"),
            "crab-link"
        );
        assert_eq!(
            normalize_handle("crab-link")
                .expect("legacy profile route still accepts internal hyphen"),
            "@crab-link"
        );

        assert!(
            RonNamingUsernameV1::parse("crab-link").is_err(),
            "native ron-naming username remains stricter than legacy profile route"
        );
        assert!(
            RonNamingHandleV1::parse("crab-link").is_err(),
            "native ron-naming handle remains stricter than legacy profile route"
        );
        assert!(
            parse_optional_username_handle(Some("crab-link")).is_err(),
            "native optional handle parser keeps ron-naming behavior"
        );
    }

    #[test]
    fn optional_native_handle_parser_matches_profile_handle_normalization() {
        for input in ["SkinnyCrabby", "@SkinnyCrabby", "creator_007"] {
            let optional_handle = parse_optional_username_handle(Some(input))
                .expect("optional handle parses")
                .expect("optional handle is present");
            let profile_handle = normalize_handle(input).expect("profile handle normalizes");

            assert_eq!(optional_handle.as_str(), profile_handle);
        }

        assert_eq!(parse_optional_username_handle(None).unwrap(), None);
    }

    #[test]
    fn site_is_reserved_in_ron_naming_to_preserve_profile_route_contract() {
        let ron_naming_source = fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../ron-naming/src/passport_username.rs"),
        )
        .expect("ron-naming username source should be readable");

        assert!(
            ron_naming_source.contains("\"site\""),
            "ron-naming reserved username labels must include site"
        );

        assert!(normalize_username("@site").is_err());
        assert!(normalize_handle("@site").is_err());
        assert!(RonNamingUsernameV1::parse("@site").is_err());
        assert!(RonNamingHandleV1::parse("@site").is_err());
    }

    #[test]
    fn profile_source_remains_owner_of_claim_truth_not_wallet_or_ledger_authority() {
        let profile_source =
            fs::read_to_string(repo_file("src/profile.rs")).expect("profile source reads");
        let posture = native_passport_username_reuse_posture();

        assert_eq!(posture.canonical_username_owner, "ron-naming");
        assert!(posture.ron_naming_reuse_enabled);

        for forbidden in [
            "wallet spend authority",
            "ledger mutation authority",
            "capability issuance authority",
            "secret recovery material",
        ] {
            assert!(
                PHASE2B_FORBIDDEN_HANDLE_MEANINGS.contains(&forbidden),
                "forbidden handle meanings must include {forbidden}"
            );
        }

        for forbidden_runtime_token in [
            "wallet.spend",
            "ledger.write",
            "issue_capability",
            "root_private_key",
            "device_private_key",
            "vault_master_key",
            "derived_vault_key",
        ] {
            assert!(
                !profile_source.contains(forbidden_runtime_token),
                "profile source must not gain runtime authority token {forbidden_runtime_token}"
            );
        }
    }

    #[test]
    fn phase2c_does_not_change_runtime_authority_or_add_routes() {
        let posture = native_passport_username_reuse_posture();

        assert!(!posture.runtime_authority_changed);
        assert!(!posture.native_secret_implementation_added);
        assert!(!posture.routes_added);
        assert!(!posture.signing_or_verification_runtime_added);
        assert!(!posture.vault_runtime_added);
        assert!(!posture.capability_issuance_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
    }
}
