#[cfg(not(feature = "native-passport"))]
#[test]
fn native_passport_phase1b_dtos_are_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not enable native-passport DTOs implicitly"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use svc_passport::native::{
        B3DigestHex, ChallengeIdV1, DeviceAuthorizationDraftV1, DeviceClass, DeviceIdV1,
        Ed25519PublicKeyHex, NativePassportDtoError, NativePassportScope, PassportIdV1,
        NATIVE_PASSPORT_PHASE1B_LABEL, NATIVE_PASSPORT_READ_ONLY_SCOPE_CEILING,
        NATIVE_PASSPORT_UNSAFE_SCOPE_STRINGS, PHASE1B_FORBIDDEN_DTO_FIELDS,
    };

    const PASSPORT_ID: &str = "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
    const DEVICE_ID: &str =
        "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";
    const CHALLENGE_ID: &str =
        "challenge:v1:b3:58577b7fde04e28ccce564ea08ed46575bb00f60924a9c1cf8d7546497b1148e";
    const ROOT_PUBLIC_KEY: &str =
        "3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909";
    const DEVICE_PUBLIC_KEY: &str =
        "2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";
    const NONCE_HEX: &str = "000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f";

    #[test]
    fn phase1b_label_is_locked() {
        assert_eq!(
            NATIVE_PASSPORT_PHASE1B_LABEL,
            "NATIVE_PASSPORT_PHASE1B_NATIVE_DTO_TYPES"
        );
    }

    #[test]
    fn passport_device_and_challenge_ids_validate_locked_formats() {
        let passport_id = PassportIdV1::parse(PASSPORT_ID).expect("passport id parses");
        let device_id = DeviceIdV1::parse(DEVICE_ID).expect("device id parses");
        let challenge_id = ChallengeIdV1::parse(CHALLENGE_ID).expect("challenge id parses");

        assert_eq!(passport_id.as_str(), PASSPORT_ID);
        assert_eq!(passport_id.to_string(), PASSPORT_ID);
        assert_eq!(device_id.as_str(), DEVICE_ID);
        assert_eq!(device_id.to_string(), DEVICE_ID);
        assert_eq!(challenge_id.as_str(), CHALLENGE_ID);
        assert_eq!(challenge_id.to_string(), CHALLENGE_ID);
    }

    #[test]
    fn id_wrappers_reject_legacy_uppercase_short_and_wrong_prefix_values() {
        assert_eq!(
            PassportIdV1::parse("passport:main:dev").unwrap_err(),
            NativePassportDtoError::InvalidPassportId
        );
        assert_eq!(
            PassportIdV1::parse("passport:v1:main:ed25519:b3:ACC2761E583FAFC93CBB880BEF1BD7285F43B3BBF326B9E185B226C5533CB7DF")
                .unwrap_err(),
            NativePassportDtoError::InvalidPassportId
        );
        assert_eq!(
            DeviceIdV1::parse("device:v1:ed25519:b3:4c18").unwrap_err(),
            NativePassportDtoError::InvalidDeviceId
        );
        assert_eq!(
            ChallengeIdV1::parse("challenge:v1:sha256:58577b7fde04e28ccce564ea08ed46575bb00f60924a9c1cf8d7546497b1148e")
                .unwrap_err(),
            NativePassportDtoError::InvalidChallengeId
        );
    }

    #[test]
    fn digest_and_public_key_hex_wrappers_validate_lowercase_hex_lengths() {
        let digest = B3DigestHex::parse("digest", NONCE_HEX).expect("digest parses");
        let public_key = Ed25519PublicKeyHex::parse(ROOT_PUBLIC_KEY).expect("public key parses");

        assert_eq!(digest.as_str(), NONCE_HEX);
        assert_eq!(public_key.as_str(), ROOT_PUBLIC_KEY);

        assert!(matches!(
            B3DigestHex::parse("digest", "ABC"),
            Err(NativePassportDtoError::InvalidLowerHex {
                field: "digest",
                expected_len: 64,
                actual_len: 3
            })
        ));

        assert!(matches!(
            Ed25519PublicKeyHex::parse("abc"),
            Err(NativePassportDtoError::InvalidLowerHex {
                field: "ed25519_public_key_hex",
                expected_len: 64,
                actual_len: 3
            })
        ));
    }

    #[test]
    fn scope_parser_accepts_read_only_scopes_and_rejects_unsafe_scopes() {
        for scope in NATIVE_PASSPORT_READ_ONLY_SCOPE_CEILING {
            let parsed =
                NativePassportScope::parse_read_only(scope.as_str()).expect("read scope parses");
            assert_eq!(parsed, *scope);
            assert_eq!(parsed.to_string(), scope.as_str());
        }

        for unsafe_scope in NATIVE_PASSPORT_UNSAFE_SCOPE_STRINGS {
            assert_eq!(
                NativePassportScope::parse_read_only(unsafe_scope).unwrap_err(),
                NativePassportDtoError::UnsafeScope((*unsafe_scope).to_owned())
            );
        }

        assert_eq!(
            NativePassportScope::parse_read_only("unknown.scope").unwrap_err(),
            NativePassportDtoError::UnsupportedScope("unknown.scope".to_owned())
        );
    }

    #[test]
    fn device_class_parser_accepts_supported_read_only_classes() {
        assert_eq!(
            DeviceClass::parse("tv_read_only").unwrap(),
            DeviceClass::TvReadOnly
        );
        assert_eq!(
            DeviceClass::parse("desktop_read_only").unwrap(),
            DeviceClass::DesktopReadOnly
        );
        assert_eq!(
            DeviceClass::parse("mobile_read_only").unwrap(),
            DeviceClass::MobileReadOnly
        );
        assert_eq!(DeviceClass::TvReadOnly.to_string(), "tv_read_only");

        assert_eq!(
            DeviceClass::parse("admin_wallet_device").unwrap_err(),
            NativePassportDtoError::UnsupportedDeviceClass("admin_wallet_device".to_owned())
        );
    }

    #[test]
    fn device_authorization_draft_accepts_bounded_read_only_contract() {
        let dto = DeviceAuthorizationDraftV1::new(
            PassportIdV1::parse(PASSPORT_ID).unwrap(),
            DeviceIdV1::parse(DEVICE_ID).unwrap(),
            Ed25519PublicKeyHex::parse(ROOT_PUBLIC_KEY).unwrap(),
            0,
            Ed25519PublicKeyHex::parse(DEVICE_PUBLIC_KEY).unwrap(),
            DeviceClass::TvReadOnly,
            NATIVE_PASSPORT_READ_ONLY_SCOPE_CEILING.to_vec(),
            B3DigestHex::parse("authorization_nonce_hex", NONCE_HEX).unwrap(),
        )
        .expect("device authorization DTO draft should validate");

        assert_eq!(dto.passport_id().as_str(), PASSPORT_ID);
        assert_eq!(dto.device_id().as_str(), DEVICE_ID);
        assert_eq!(dto.root_public_key_hex().as_str(), ROOT_PUBLIC_KEY);
        assert_eq!(dto.root_key_epoch(), 0);
        assert_eq!(dto.device_public_key_hex().as_str(), DEVICE_PUBLIC_KEY);
        assert_eq!(dto.device_class(), DeviceClass::TvReadOnly);
        assert_eq!(
            dto.scope_ceiling_csv(),
            "identity.read,catalog.read,content.read,entitlement.read,receipts.read,confirmed_roc.read,capability.revoke_self"
        );
        assert_eq!(dto.authorization_nonce_hex().as_str(), NONCE_HEX);
    }

    #[test]
    fn device_authorization_draft_rejects_empty_and_duplicate_scope_sets() {
        let empty = DeviceAuthorizationDraftV1::new(
            PassportIdV1::parse(PASSPORT_ID).unwrap(),
            DeviceIdV1::parse(DEVICE_ID).unwrap(),
            Ed25519PublicKeyHex::parse(ROOT_PUBLIC_KEY).unwrap(),
            0,
            Ed25519PublicKeyHex::parse(DEVICE_PUBLIC_KEY).unwrap(),
            DeviceClass::TvReadOnly,
            vec![],
            B3DigestHex::parse("authorization_nonce_hex", NONCE_HEX).unwrap(),
        );

        assert_eq!(empty.unwrap_err(), NativePassportDtoError::EmptyScopeSet);

        let duplicate = DeviceAuthorizationDraftV1::new(
            PassportIdV1::parse(PASSPORT_ID).unwrap(),
            DeviceIdV1::parse(DEVICE_ID).unwrap(),
            Ed25519PublicKeyHex::parse(ROOT_PUBLIC_KEY).unwrap(),
            0,
            Ed25519PublicKeyHex::parse(DEVICE_PUBLIC_KEY).unwrap(),
            DeviceClass::TvReadOnly,
            vec![
                NativePassportScope::IdentityRead,
                NativePassportScope::IdentityRead,
            ],
            B3DigestHex::parse("authorization_nonce_hex", NONCE_HEX).unwrap(),
        );

        assert_eq!(
            duplicate.unwrap_err(),
            NativePassportDtoError::DuplicateScope("identity.read")
        );
    }

    #[test]
    fn dto_layer_keeps_secret_material_field_names_forbidden() {
        for field in [
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
                PHASE1B_FORBIDDEN_DTO_FIELDS.contains(&field),
                "Phase 1B forbidden DTO fields must include {field}"
            );
        }
    }
}
