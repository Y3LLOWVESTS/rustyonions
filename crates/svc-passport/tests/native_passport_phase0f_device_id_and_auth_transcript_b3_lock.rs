use std::{fs, path::PathBuf};

use serde_json::Value;
use svc_passport::native_plan::{
    is_device_id_v1_ed25519_b3, DEVICE_ID_V1_B3_DIGEST_HEX_LEN, DEVICE_ID_V1_ED25519_B3_PREFIX,
};

const DEVICE_PUBLIC_KEY_HEX: &str =
    "2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";
const EXPECTED_DEVICE_HASH_INPUT: &str =
    "rustyonions.native-passport.device-id.v1|ed25519|2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";
const EXPECTED_DEVICE_B3_DIGEST: &str =
    "4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";
const EXPECTED_DEVICE_ID: &str =
    "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";
const EXPECTED_AUTH_TRANSCRIPT_INPUT: &str = "rustyonions.native-passport.device-authorization.v1|1|rustyonions-devnet|local-dev|passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df|0|3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909|device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d|2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17|tv_read_only|identity.read,catalog.read,content.read,entitlement.read,receipts.read,confirmed_roc.read,capability.revoke_self|000102030405060708090a0b0c0d0e0f|1720000000000|1720086400000";
const EXPECTED_AUTH_TRANSCRIPT_B3: &str =
    "2f49a3ca1163cca3e1f489d75adc51f04d4ad08783a73a157eaba9f2cebe2277";

fn vector_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("vectors")
        .join("native_passport_device_authorization_v1.json")
}

fn device_vector() -> Value {
    let path = vector_path();
    let data = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    serde_json::from_str(&data)
        .unwrap_or_else(|err| panic!("failed to parse {} as JSON: {err}", path.display()))
}

#[test]
fn phase0f_locks_device_id_b3_digest_and_format() {
    let value = device_vector();
    let device = &value["device"];

    assert_eq!(
        value["phase"].as_str(),
        Some("NATIVE_PASSPORT_PHASE0F_DEVICE_ID_AND_AUTH_TRANSCRIPT_B3_LOCK")
    );
    assert_eq!(
        value["status"].as_str(),
        Some("device_id_and_authorization_transcript_b3_locked_no_runtime_signing")
    );
    assert_eq!(
        device["device_id_status"].as_str(),
        Some("digest_locked_phase0f_b3")
    );
    assert_eq!(
        device["device_id_hash_input_format"].as_str(),
        Some("utf8:domain|key_algorithm|device_public_key_hex")
    );
    assert_eq!(
        device["device_public_key_hex"].as_str(),
        Some(DEVICE_PUBLIC_KEY_HEX)
    );
    assert_eq!(
        device["device_id_hash_input_utf8"].as_str(),
        Some(EXPECTED_DEVICE_HASH_INPUT)
    );

    let computed = blake3::hash(EXPECTED_DEVICE_HASH_INPUT.as_bytes())
        .to_hex()
        .to_string();
    assert_eq!(computed, EXPECTED_DEVICE_B3_DIGEST);
    assert_eq!(
        device["device_id_b3_digest_hex"].as_str(),
        Some(EXPECTED_DEVICE_B3_DIGEST)
    );
    assert_eq!(device["device_id"].as_str(), Some(EXPECTED_DEVICE_ID));

    assert_eq!(DEVICE_ID_V1_ED25519_B3_PREFIX, "device:v1:ed25519:b3:");
    assert_eq!(DEVICE_ID_V1_B3_DIGEST_HEX_LEN, 64);
    assert!(is_device_id_v1_ed25519_b3(EXPECTED_DEVICE_ID));
    assert!(!is_device_id_v1_ed25519_b3("device:main:dev"));
    assert!(!is_device_id_v1_ed25519_b3(
        "device:v1:ed25519:b3:4C18B950FEB56BCAD2579821D89AEE76B259C28F77D459D5FEC33BEDD3F41F2D"
    ));
}

#[test]
fn phase0f_locks_authorization_transcript_b3_without_signature_runtime() {
    let value = device_vector();
    let authorization = &value["authorization"];

    assert_eq!(
        authorization["authorization_transcript_status"].as_str(),
        Some("b3_locked_phase0f_no_signature")
    );
    assert_eq!(
        authorization["authorization_transcript_encoding"].as_str(),
        Some("pipe-delimited-canonical-v1")
    );
    assert_eq!(
        authorization["authorization_transcript_input_utf8"].as_str(),
        Some(EXPECTED_AUTH_TRANSCRIPT_INPUT)
    );

    let computed = blake3::hash(EXPECTED_AUTH_TRANSCRIPT_INPUT.as_bytes())
        .to_hex()
        .to_string();
    assert_eq!(computed, EXPECTED_AUTH_TRANSCRIPT_B3);
    assert_eq!(
        authorization["authorization_transcript_b3_hex"].as_str(),
        Some(EXPECTED_AUTH_TRANSCRIPT_B3)
    );

    assert_eq!(
        authorization["root_signature_status"].as_str(),
        Some("pending_phase1_or_later_ed25519_signature_lock")
    );
    assert_eq!(
        authorization["root_signature_hex"].as_str(),
        Some("PHASE1_PENDING_ED25519_ROOT_SIGNATURE")
    );
}

#[test]
fn phase0f_still_forbids_runtime_authority_and_secret_material() {
    let value = device_vector();

    assert_eq!(value["runtime_authority_changed"].as_bool(), Some(false));
    assert_eq!(
        value["native_secret_implementation_added"].as_bool(),
        Some(false)
    );
    assert_eq!(value["signature_generation_added"].as_bool(), Some(false));
    assert_eq!(value["challenge_route_added"].as_bool(), Some(false));
    assert_eq!(value["capability_issuance_added"].as_bool(), Some(false));

    let must_not_include = value["must_not_include"]
        .as_array()
        .expect("must_not_include must be array");

    for item in [
        "mnemonic words",
        "bip39 seed",
        "root private key",
        "root signing seed",
        "device private key",
        "PIN",
        "vault key",
        "vault ciphertext",
        "wallet spend authority",
        "ledger mutation authority",
    ] {
        assert!(
            must_not_include
                .iter()
                .any(|entry| entry.as_str() == Some(item)),
            "Phase 0F vector must not include {item}"
        );
    }
}
