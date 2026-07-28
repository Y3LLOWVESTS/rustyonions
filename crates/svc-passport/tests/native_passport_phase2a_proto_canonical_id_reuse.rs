#[cfg(not(feature = "native-passport"))]
#[test]
fn phase2a_proto_reuse_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native Passport proto reuse"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use ron_proto::{
        ChallengeIdV1 as ProtoChallengeIdV1, DeviceIdV1 as ProtoDeviceIdV1,
        PassportIdV1 as ProtoPassportIdV1,
    };
    use svc_passport::native::{
        ChallengeIdV1 as SvcChallengeIdV1, DeviceIdV1 as SvcDeviceIdV1,
        PassportIdV1 as SvcPassportIdV1,
    };

    const PASSPORT_ID: &str = "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
    const DEVICE_ID: &str =
        "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";
    const CHALLENGE_ID: &str =
        "challenge:v1:b3:58577b7fde04e28ccce564ea08ed46575bb00f60924a9c1cf8d7546497b1148e";

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    #[test]
    fn phase2a_svc_passport_native_ids_match_ron_proto_canonical_ids() {
        let proto_passport = ProtoPassportIdV1::parse(PASSPORT_ID).unwrap();
        let svc_passport = SvcPassportIdV1::parse(PASSPORT_ID).unwrap();

        let proto_device = ProtoDeviceIdV1::parse(DEVICE_ID).unwrap();
        let svc_device = SvcDeviceIdV1::parse(DEVICE_ID).unwrap();

        let proto_challenge = ProtoChallengeIdV1::parse(CHALLENGE_ID).unwrap();
        let svc_challenge = SvcChallengeIdV1::parse(CHALLENGE_ID).unwrap();

        assert_eq!(proto_passport.as_str(), svc_passport.as_str());
        assert_eq!(proto_device.as_str(), svc_device.as_str());
        assert_eq!(proto_challenge.as_str(), svc_challenge.as_str());
    }

    #[test]
    fn phase2a_svc_passport_rejects_the_same_bad_ids_as_ron_proto() {
        for bad_passport in [
            "passport:legacy:dev",
            "passport:v1:main:ed25519:b3:ACC2761E583FAFC93CBB880BEF1BD7285F43B3BBF326B9E185B226C5533CB7DF",
            "passport:v1:main:ed25519:b3:acc2",
        ] {
            assert!(ProtoPassportIdV1::parse(bad_passport).is_err());
            assert!(SvcPassportIdV1::parse(bad_passport).is_err());
        }

        for bad_device in [
            "device:legacy:dev",
            "device:v1:ed25519:b3:4C18B950FEB56BCAD2579821D89AEE76B259C28F77D459D5FEC33BEDD3F41F2D",
            "device:v1:ed25519:b3:4c18",
        ] {
            assert!(ProtoDeviceIdV1::parse(bad_device).is_err());
            assert!(SvcDeviceIdV1::parse(bad_device).is_err());
        }
    }

    #[test]
    fn phase2a_svc_passport_dto_source_uses_ron_proto_id_parsers() {
        let dto_source =
            fs::read_to_string(repo_file("src/native/dto.rs")).expect("dto source reads");

        assert!(dto_source.contains("ron_proto::PassportIdV1::parse"));
        assert!(dto_source.contains("ron_proto::DeviceIdV1::parse"));
        assert!(dto_source.contains("ron_proto::ChallengeIdV1::parse"));

        assert!(
            !dto_source.contains("is_passport_id_v1_main_ed25519_b3(&value)"),
            "svc-passport native DTO should not call its old local Passport ID guard"
        );
        assert!(
            !dto_source.contains("is_device_id_v1_ed25519_b3(&value)"),
            "svc-passport native DTO should not call its old local Device ID guard"
        );
    }

    #[test]
    fn phase2a_native_passport_feature_owns_optional_ron_proto_reuse() {
        let cargo_toml =
            fs::read_to_string(repo_file("Cargo.toml")).expect("Cargo.toml should read");

        assert!(
            cargo_toml.contains("ron-proto"),
            "svc-passport must depend on ron-proto for canonical Native Passport IDs"
        );
        assert!(
            cargo_toml.contains("native-passport = ["),
            "native-passport feature line must remain explicit"
        );
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
