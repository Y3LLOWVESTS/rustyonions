#[cfg(not(feature = "native-passport"))]
#[test]
fn native_passport_phase1c_dto_posture_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not enable native-passport DTO posture implicitly"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use svc_passport::native::{
        native_passport_dto_posture, B3DigestHex, ChallengeIdV1, DeviceClass, DeviceIdV1,
        Ed25519PublicKeyHex, NativePassportScope, NativePassportSurface, PassportIdV1,
        NATIVE_PASSPORT_PHASE1C_LABEL, NATIVE_PASSPORT_READ_ONLY_SCOPE_CEILING,
        NATIVE_PASSPORT_UNSAFE_SCOPE_STRINGS, PHASE1B_FORBIDDEN_DTO_FIELDS, PHASE1C_DTO_TYPE_NAMES,
        PHASE1C_ENABLED_SURFACES,
    };

    const PASSPORT_ID: &str = "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
    const DEVICE_ID: &str =
        "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";
    const CHALLENGE_ID: &str =
        "challenge:v1:b3:58577b7fde04e28ccce564ea08ed46575bb00f60924a9c1cf8d7546497b1148e";
    const DEVICE_PUBLIC_KEY: &str =
        "2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";
    const DIGEST_HEX: &str = "66eb9fbd0da6e2425978405a9e32094bbf8bbd078a2c62021fc301d622afdd3e";

    #[test]
    fn phase1c_label_and_surface_posture_are_locked() {
        let posture = native_passport_dto_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE1C_LABEL,
            "NATIVE_PASSPORT_PHASE1C_NATIVE_MODULE_DTO_POSTURE"
        );
        assert_eq!(posture.owner, "svc-passport");
        assert_eq!(posture.feature_name, "native-passport");
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE1C_LABEL);
        assert_eq!(posture.enabled_surfaces, PHASE1C_ENABLED_SURFACES);
        assert_eq!(
            posture.enabled_surfaces,
            &[
                NativePassportSurface::FeaturePosture,
                NativePassportSurface::Phase0Contracts,
                NativePassportSurface::NativeDtoTypes,
            ]
        );
    }

    #[test]
    fn phase1c_dto_type_names_are_complete_and_stable() {
        let posture = native_passport_dto_posture();

        assert_eq!(posture.dto_type_names, PHASE1C_DTO_TYPE_NAMES);

        for expected in [
            "PassportIdV1",
            "DeviceIdV1",
            "ChallengeIdV1",
            "B3DigestHex",
            "Ed25519PublicKeyHex",
            "NativePassportScope",
            "DeviceClass",
            "DeviceAuthorizationDraftV1",
        ] {
            assert!(
                posture.dto_type_names.contains(&expected),
                "Phase 1C DTO posture must expose {expected}"
            );
        }
    }

    #[test]
    fn phase1c_dto_posture_keeps_runtime_authority_disabled() {
        let posture = native_passport_dto_posture();

        assert!(!posture.runtime_authority_changed);
        assert!(!posture.native_secret_implementation_added);
        assert!(!posture.routes_added);
        assert!(!posture.signing_or_verification_runtime_added);
        assert!(!posture.vault_runtime_added);
        assert!(!posture.capability_issuance_added);
    }

    #[test]
    fn phase1c_dto_posture_reports_scope_boundaries() {
        let posture = native_passport_dto_posture();

        assert_eq!(
            posture.read_only_scope_count,
            NATIVE_PASSPORT_READ_ONLY_SCOPE_CEILING.len()
        );
        assert_eq!(
            posture.unsafe_scope_count,
            NATIVE_PASSPORT_UNSAFE_SCOPE_STRINGS.len()
        );
        assert_eq!(posture.read_only_scope_count, 7);
        assert_eq!(posture.unsafe_scope_count, 8);

        for read_scope in NATIVE_PASSPORT_READ_ONLY_SCOPE_CEILING {
            assert_eq!(
                NativePassportScope::parse_read_only(read_scope.as_str()).unwrap(),
                *read_scope
            );
        }

        for unsafe_scope in NATIVE_PASSPORT_UNSAFE_SCOPE_STRINGS {
            assert!(NativePassportScope::parse_read_only(unsafe_scope).is_err());
        }
    }

    #[test]
    fn phase1c_dto_posture_reports_forbidden_secret_fields() {
        let posture = native_passport_dto_posture();

        assert_eq!(posture.forbidden_dto_fields, PHASE1B_FORBIDDEN_DTO_FIELDS);

        for forbidden in [
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
                posture.forbidden_dto_fields.contains(&forbidden),
                "Phase 1C posture must keep {forbidden} forbidden"
            );
        }
    }

    #[test]
    fn phase1c_root_module_reexports_native_dto_types() {
        let passport_id = PassportIdV1::parse(PASSPORT_ID).expect("passport id parses");
        let device_id = DeviceIdV1::parse(DEVICE_ID).expect("device id parses");
        let challenge_id = ChallengeIdV1::parse(CHALLENGE_ID).expect("challenge id parses");
        let device_public_key =
            Ed25519PublicKeyHex::parse(DEVICE_PUBLIC_KEY).expect("public key parses");
        let digest =
            B3DigestHex::parse("proof_transcript_b3_hex", DIGEST_HEX).expect("digest parses");
        let device_class = DeviceClass::parse("tv_read_only").expect("device class parses");

        assert_eq!(passport_id.as_str(), PASSPORT_ID);
        assert_eq!(device_id.as_str(), DEVICE_ID);
        assert_eq!(challenge_id.as_str(), CHALLENGE_ID);
        assert_eq!(device_public_key.as_str(), DEVICE_PUBLIC_KEY);
        assert_eq!(digest.as_str(), DIGEST_HEX);
        assert_eq!(device_class, DeviceClass::TvReadOnly);
    }
}
