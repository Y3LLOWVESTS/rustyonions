use ron_proto::{
    is_challenge_id_v1_b3, is_device_id_v1_ed25519_b3, is_passport_id_v1_main_ed25519_b3,
    B3DigestHex, ChallengeIdV1, DeviceClassV1, DeviceIdV1, Ed25519PublicKeyHex,
    LegacyPassportSubject, NativePassportDigestAlgorithm, NativePassportIdAlgorithm,
    NativePassportIdKind, NativePassportIdParseError, NativePassportIdVersion, PassportIdV1,
    B3_DIGEST_HEX_LEN, CHALLENGE_ID_V1_B3_PREFIX, DEVICE_ID_V1_ED25519_B3_PREFIX,
    ED25519_PUBLIC_KEY_HEX_LEN, PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX,
};

const PASSPORT_ID: &str =
    "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
const DEVICE_ID: &str =
    "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";
const CHALLENGE_ID: &str =
    "challenge:v1:b3:58577b7fde04e28ccce564ea08ed46575bb00f60924a9c1cf8d7546497b1148e";
const ROOT_PUBLIC_KEY: &str = "3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909";
const DIGEST_HEX: &str = "66eb9fbd0da6e2425978405a9e32094bbf8bbd078a2c62021fc301d622afdd3e";

#[test]
fn phase2a_prefixes_and_lengths_are_locked() {
    assert_eq!(
        PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX,
        "passport:v1:main:ed25519:b3:"
    );
    assert_eq!(DEVICE_ID_V1_ED25519_B3_PREFIX, "device:v1:ed25519:b3:");
    assert_eq!(CHALLENGE_ID_V1_B3_PREFIX, "challenge:v1:b3:");
    assert_eq!(B3_DIGEST_HEX_LEN, 64);
    assert_eq!(ED25519_PUBLIC_KEY_HEX_LEN, 64);
}

#[test]
fn phase2a_algorithm_version_and_kind_enums_are_explicit() {
    assert_eq!(NativePassportIdVersion::V1, NativePassportIdVersion::V1);
    assert_eq!(NativePassportIdKind::Main, NativePassportIdKind::Main);
    assert_eq!(
        NativePassportIdAlgorithm::Ed25519,
        NativePassportIdAlgorithm::Ed25519
    );
    assert_eq!(
        NativePassportDigestAlgorithm::Blake3,
        NativePassportDigestAlgorithm::Blake3
    );
    assert_eq!(DeviceClassV1::TvReadOnly, DeviceClassV1::TvReadOnly);
}

#[test]
fn phase2a_parses_locked_phase0_ids_and_public_values() {
    let passport_id = PassportIdV1::parse(PASSPORT_ID).expect("passport id parses");
    let device_id = DeviceIdV1::parse(DEVICE_ID).expect("device id parses");
    let challenge_id = ChallengeIdV1::parse(CHALLENGE_ID).expect("challenge id parses");
    let digest = B3DigestHex::parse("proof_transcript_b3_hex", DIGEST_HEX).unwrap();
    let public_key = Ed25519PublicKeyHex::parse(ROOT_PUBLIC_KEY).unwrap();

    assert_eq!(passport_id.as_str(), PASSPORT_ID);
    assert_eq!(passport_id.to_string(), PASSPORT_ID);
    assert_eq!(device_id.as_str(), DEVICE_ID);
    assert_eq!(device_id.to_string(), DEVICE_ID);
    assert_eq!(challenge_id.as_str(), CHALLENGE_ID);
    assert_eq!(challenge_id.to_string(), CHALLENGE_ID);
    assert_eq!(digest.as_str(), DIGEST_HEX);
    assert_eq!(public_key.as_str(), ROOT_PUBLIC_KEY);

    assert!(is_passport_id_v1_main_ed25519_b3(PASSPORT_ID));
    assert!(is_device_id_v1_ed25519_b3(DEVICE_ID));
    assert!(is_challenge_id_v1_b3(CHALLENGE_ID));
}

#[test]
fn phase2a_rejects_legacy_uppercase_short_and_wrong_prefix_ids() {
    assert_eq!(
        PassportIdV1::parse("passport:legacy:dev").unwrap_err(),
        NativePassportIdParseError::InvalidPrefix {
            field: "passport_id"
        }
    );

    assert!(matches!(
        PassportIdV1::parse("passport:v1:main:ed25519:b3:ACC2761E583FAFC93CBB880BEF1BD7285F43B3BBF326B9E185B226C5533CB7DF"),
        Err(NativePassportIdParseError::InvalidLowerHex { field: "passport_id" })
    ));

    assert!(matches!(
        DeviceIdV1::parse("device:v1:ed25519:b3:4c18"),
        Err(NativePassportIdParseError::InvalidHexLength {
            field: "device_id",
            expected_len: 64,
            actual_len: 4
        })
    ));

    assert_eq!(
        ChallengeIdV1::parse(
            "challenge:v1:sha256:58577b7fde04e28ccce564ea08ed46575bb00f60924a9c1cf8d7546497b1148e"
        )
        .unwrap_err(),
        NativePassportIdParseError::InvalidPrefix {
            field: "challenge_id"
        }
    );
}

#[test]
fn phase2a_legacy_subject_cannot_become_production_passport_id() {
    let legacy = LegacyPassportSubject::parse("legacy-dev-subject").unwrap();

    assert_eq!(legacy.as_str(), "legacy-dev-subject");
    assert!(PassportIdV1::parse(legacy.as_str()).is_err());
    assert!(!is_passport_id_v1_main_ed25519_b3(legacy.as_str()));
}

#[test]
fn phase2a_serde_round_trips_validate_canonical_id_strings() {
    let passport_id = PassportIdV1::parse(PASSPORT_ID).unwrap();
    let json = serde_json::to_string(&passport_id).unwrap();

    assert_eq!(json, format!("\"{PASSPORT_ID}\""));

    let parsed: PassportIdV1 = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, passport_id);

    let uppercase = format!(
        "\"{}\"",
        "passport:v1:main:ed25519:b3:ACC2761E583FAFC93CBB880BEF1BD7285F43B3BBF326B9E185B226C5533CB7DF"
    );
    assert!(serde_json::from_str::<PassportIdV1>(&uppercase).is_err());
}
