use std::{collections::BTreeSet, fs, path::PathBuf};

use serde_json::Value;

const LOCKED_PASSPORT_ID: &str =
    "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
const LOCKED_ROOT_PUBLIC_KEY_HEX: &str =
    "3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909";

fn vector_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("vectors")
        .join(file_name)
}

fn read_json(file_name: &str) -> Value {
    let path = vector_path(file_name);
    let data = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    serde_json::from_str(&data)
        .unwrap_or_else(|err| panic!("failed to parse {} as JSON: {err}", path.display()))
}

fn string_set(value: &Value) -> BTreeSet<&str> {
    value
        .as_array()
        .expect("value must be array")
        .iter()
        .map(|item| item.as_str().expect("array items must be strings"))
        .collect()
}

#[test]
fn device_authorization_vector_schema_and_phase_are_locked() {
    let value = read_json("native_passport_device_authorization_v1.json");

    assert_eq!(
        value["schema"].as_str(),
        Some("svc-passport.native-passport.device-authorization-vector.v1")
    );
    assert_eq!(
        value["phase"].as_str(),
        Some("NATIVE_PASSPORT_PHASE0F_DEVICE_ID_AND_AUTH_TRANSCRIPT_B3_LOCK")
    );
    assert_eq!(
        value["status"].as_str(),
        Some("device_id_and_authorization_transcript_b3_locked_no_runtime_signing")
    );
    assert_eq!(
        value["canonical_passport_package_owner"].as_str(),
        Some("svc-passport")
    );
    assert_eq!(value["runtime_authority_changed"].as_bool(), Some(false));
    assert_eq!(
        value["native_secret_implementation_added"].as_bool(),
        Some(false)
    );
    assert_eq!(value["signature_generation_added"].as_bool(), Some(false));
    assert_eq!(value["challenge_route_added"].as_bool(), Some(false));
    assert_eq!(value["capability_issuance_added"].as_bool(), Some(false));
    assert_eq!(value["never_use_for_real_passport"].as_bool(), Some(true));
}

#[test]
fn device_authorization_uses_locked_passport_id_and_root_public_key() {
    let root = read_json("native_passport_root_identity_v1.json");
    let device = read_json("native_passport_device_authorization_v1.json");

    assert_eq!(
        root["passport_id_derivation"]["passport_id"].as_str(),
        Some(LOCKED_PASSPORT_ID)
    );
    assert_eq!(
        root["root_derivation"]["root_public_key_hex"].as_str(),
        Some(LOCKED_ROOT_PUBLIC_KEY_HEX)
    );

    assert_eq!(device["passport_id"].as_str(), Some(LOCKED_PASSPORT_ID));
    assert_eq!(
        device["root_public_key_hex"].as_str(),
        Some(LOCKED_ROOT_PUBLIC_KEY_HEX)
    );
    assert_eq!(device["root_key_epoch"].as_u64(), Some(0));
}

#[test]
fn device_identity_fields_are_present_and_device_id_digest_is_locked_by_phase0f() {
    let value = read_json("native_passport_device_authorization_v1.json");
    let device = &value["device"];

    assert_eq!(
        device["device_id_status"].as_str(),
        Some("digest_locked_phase0f_b3")
    );
    assert_eq!(
        device["device_id_format"].as_str(),
        Some("device:v1:ed25519:b3:{b3_digest_hex}")
    );
    assert_eq!(
        device["device_id"].as_str(),
        Some(
            "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d"
        )
    );
    assert_eq!(
        device["device_public_key_hex"].as_str(),
        Some("2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17")
    );
    assert_eq!(device["device_class"].as_str(), Some("tv_read_only"));
    assert_eq!(device["device_private_key_included"].as_bool(), Some(false));
    assert_eq!(
        device["device_private_key_generated"].as_bool(),
        Some(false)
    );
}

#[test]
fn read_only_device_scope_ceiling_is_safe() {
    let value = read_json("native_passport_device_authorization_v1.json");
    let authorization = &value["authorization"];

    assert_eq!(
        authorization["schema_name"].as_str(),
        Some("DeviceAuthorizationV1")
    );
    assert_eq!(authorization["version"].as_u64(), Some(1));
    assert_eq!(
        authorization["network_id"].as_str(),
        Some("rustyonions-devnet")
    );
    assert_eq!(authorization["environment"].as_str(), Some("local-dev"));

    let allowed = string_set(&authorization["authorized_scope_ceiling"]);
    let forbidden = string_set(&authorization["forbidden_scope_ceiling"]);

    for required in [
        "identity.read",
        "catalog.read",
        "content.read",
        "entitlement.read",
        "receipts.read",
        "confirmed_roc.read",
        "capability.revoke_self",
    ] {
        assert!(
            allowed.contains(required),
            "missing allowed scope {required}"
        );
    }

    for denied in [
        "wallet.spend",
        "wallet.transfer",
        "ledger.write",
        "reward.issue",
        "node.control",
        "operator.admin",
        "content.publish",
        "capability.delegate_unbounded",
    ] {
        assert!(
            forbidden.contains(denied),
            "missing forbidden scope {denied}"
        );
        assert!(
            !allowed.contains(denied),
            "forbidden scope {denied} must not be in allowed ceiling"
        );
    }
}

#[test]
fn signature_and_transcript_are_explicitly_deferred() {
    let value = read_json("native_passport_device_authorization_v1.json");
    let authorization = &value["authorization"];

    assert_eq!(
        authorization["root_signature_status"].as_str(),
        Some("pending_phase1_or_later_ed25519_signature_lock")
    );
    assert_eq!(
        authorization["root_signature_hex"].as_str(),
        Some("PHASE1_PENDING_ED25519_ROOT_SIGNATURE")
    );
    assert_eq!(
        authorization["authorization_transcript_status"].as_str(),
        Some("b3_locked_phase0f_no_signature")
    );
    assert_eq!(
        authorization["authorization_transcript_encoding"].as_str(),
        Some("pipe-delimited-canonical-v1")
    );
    assert_eq!(
        authorization["authorization_transcript_b3_hex"].as_str(),
        Some("2f49a3ca1163cca3e1f489d75adc51f04d4ad08783a73a157eaba9f2cebe2277")
    );
}

#[test]
fn device_authorization_excludes_human_names_pins_wallets_and_secrets() {
    let value = read_json("native_passport_device_authorization_v1.json");

    assert_eq!(
        value["must_be_signed_by"].as_str(),
        Some("Passport root private key")
    );
    assert_eq!(
        value["must_be_verified_with"].as_str(),
        Some("root_public_key_hex")
    );

    let not_signers = string_set(&value["must_not_be_signed_by"]);
    for forbidden in [
        "device private key",
        "PIN",
        "username",
        "@username",
        "display name",
        "real human name",
        "legal name",
        "wallet account",
        "ledger account",
        "service KMS key",
    ] {
        assert!(
            not_signers.contains(forbidden),
            "device authorization must not be signed by {forbidden}"
        );
    }

    let not_included = string_set(&value["must_not_include"]);
    for forbidden in [
        "mnemonic words",
        "bip39 seed",
        "root private key",
        "root signing seed",
        "device private key",
        "PIN",
        "vault key",
        "vault ciphertext",
        "platform device secret",
        "wallet spend authority",
        "ledger mutation authority",
    ] {
        assert!(
            not_included.contains(forbidden),
            "device authorization fixture must not include {forbidden}"
        );
    }
}

#[test]
fn phase0e_does_not_authorize_runtime_changes() {
    let value = read_json("native_passport_device_authorization_v1.json");
    let forbidden = string_set(&value["phase0e_forbidden_runtime_changes"]);

    for item in [
        "device private key generation",
        "root signature generation",
        "signature verification route",
        "challenge route implementation",
        "capability issuance",
        "vault encryption",
        "username finalization",
        "ledger mutation",
        "wallet mutation",
        "new Passport crate",
    ] {
        assert!(
            forbidden.contains(item),
            "Phase 0E must keep {item} forbidden"
        );
    }
}

#[test]
fn phase0b_inventory_named_device_authorization_file_now_exists() {
    let inventory = read_json("native_passport_phase0b_inventory.json");
    let profiles = inventory["required_vector_profiles"]
        .as_array()
        .expect("required_vector_profiles must be array");

    let device_profile = profiles
        .iter()
        .find(|profile| profile["name"].as_str() == Some("device_authorization_v1"))
        .expect("device_authorization_v1 profile must exist in Phase 0B inventory");

    assert_eq!(
        device_profile["required_file"].as_str(),
        Some("native_passport_device_authorization_v1.json")
    );
    assert!(vector_path("native_passport_device_authorization_v1.json").is_file());
}
