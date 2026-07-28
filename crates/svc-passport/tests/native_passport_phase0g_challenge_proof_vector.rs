use std::{collections::BTreeSet, fs, path::PathBuf};

use serde_json::Value;

const LOCKED_PASSPORT_ID: &str =
    "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
const LOCKED_DEVICE_ID: &str =
    "device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d";
const LOCKED_DEVICE_PUBLIC_KEY_HEX: &str =
    "2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";
const LOCKED_DEVICE_AUTHORIZATION_HASH_HEX: &str =
    "2f49a3ca1163cca3e1f489d75adc51f04d4ad08783a73a157eaba9f2cebe2277";
const EXPECTED_CHALLENGE_ID_HASH_INPUT: &str = "rustyonions.native-passport.challenge-id.v1|rustyonions-devnet|local-dev|svc-passport|prove_session|passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df|device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d|101112131415161718191a1b1c1d1e1f|1720000100000|1720000400000";
const EXPECTED_CHALLENGE_ID_B3: &str =
    "58577b7fde04e28ccce564ea08ed46575bb00f60924a9c1cf8d7546497b1148e";
const EXPECTED_CHALLENGE_ID: &str =
    "challenge:v1:b3:58577b7fde04e28ccce564ea08ed46575bb00f60924a9c1cf8d7546497b1148e";
const EXPECTED_PROOF_TRANSCRIPT_INPUT: &str = "rustyonions.native-passport.challenge-proof.v1|challenge:v1:b3:58577b7fde04e28ccce564ea08ed46575bb00f60924a9c1cf8d7546497b1148e|101112131415161718191a1b1c1d1e1f|prove_session|svc-passport|local-dev|identity.read,catalog.read,content.read,entitlement.read,receipts.read,confirmed_roc.read,capability.revoke_self|1720000100000|1720000400000|passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df|device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d|2f49a3ca1163cca3e1f489d75adc51f04d4ad08783a73a157eaba9f2cebe2277|0000000000000000000000000000000000000000000000000000000000000000";
const EXPECTED_PROOF_TRANSCRIPT_B3: &str =
    "1ddbe318883cfc8b49a0c685e66b95208111cc764b2f0615b35eda5d0bf82e9c";

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
fn challenge_proof_vector_schema_and_phase_are_locked() {
    let value = read_json("native_passport_challenge_proof_v1.json");

    assert_eq!(
        value["schema"].as_str(),
        Some("svc-passport.native-passport.challenge-proof-vector.v1")
    );
    assert_eq!(
        value["phase"].as_str(),
        Some("NATIVE_PASSPORT_PHASE0G_CHALLENGE_PROOF_VECTOR")
    );
    assert_eq!(
        value["status"].as_str(),
        Some("challenge_proof_transcript_b3_locked_no_runtime_signature")
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
    assert_eq!(value["replay_store_added"].as_bool(), Some(false));
    assert_eq!(value["never_use_for_real_passport"].as_bool(), Some(true));
}

#[test]
fn challenge_proof_uses_locked_device_authorization_context() {
    let device = read_json("native_passport_device_authorization_v1.json");
    let challenge = read_json("native_passport_challenge_proof_v1.json");

    assert_eq!(device["passport_id"].as_str(), Some(LOCKED_PASSPORT_ID));
    assert_eq!(
        device["device"]["device_id"].as_str(),
        Some(LOCKED_DEVICE_ID)
    );
    assert_eq!(
        device["device"]["device_public_key_hex"].as_str(),
        Some(LOCKED_DEVICE_PUBLIC_KEY_HEX)
    );
    assert_eq!(
        device["authorization"]["authorization_transcript_b3_hex"].as_str(),
        Some(LOCKED_DEVICE_AUTHORIZATION_HASH_HEX)
    );

    assert_eq!(challenge["passport_id"].as_str(), Some(LOCKED_PASSPORT_ID));
    assert_eq!(challenge["device_id"].as_str(), Some(LOCKED_DEVICE_ID));
    assert_eq!(
        challenge["device_public_key_hex"].as_str(),
        Some(LOCKED_DEVICE_PUBLIC_KEY_HEX)
    );
    assert_eq!(
        challenge["device_authorization_hash_hex"].as_str(),
        Some(LOCKED_DEVICE_AUTHORIZATION_HASH_HEX)
    );
}

#[test]
fn phase0g_locks_challenge_id_b3_digest() {
    let value = read_json("native_passport_challenge_proof_v1.json");
    let challenge = &value["challenge"];

    assert_eq!(
        challenge["challenge_id_status"].as_str(),
        Some("b3_locked_phase0g_no_challenge_runtime")
    );
    assert_eq!(
        challenge["challenge_id_format"].as_str(),
        Some("challenge:v1:b3:{b3_digest_hex}")
    );
    assert_eq!(
        challenge["challenge_id_hash_input_utf8"].as_str(),
        Some(EXPECTED_CHALLENGE_ID_HASH_INPUT)
    );

    let computed = blake3::hash(EXPECTED_CHALLENGE_ID_HASH_INPUT.as_bytes())
        .to_hex()
        .to_string();
    assert_eq!(computed, EXPECTED_CHALLENGE_ID_B3);
    assert_eq!(
        challenge["challenge_id_b3_digest_hex"].as_str(),
        Some(EXPECTED_CHALLENGE_ID_B3)
    );
    assert_eq!(
        challenge["challenge_id"].as_str(),
        Some(EXPECTED_CHALLENGE_ID)
    );
}

#[test]
fn phase0g_locks_proof_transcript_b3_without_device_signature_runtime() {
    let value = read_json("native_passport_challenge_proof_v1.json");
    let proof = &value["proof"];

    assert_eq!(
        proof["proof_transcript_status"].as_str(),
        Some("b3_locked_phase0g_no_signature")
    );
    assert_eq!(
        proof["proof_transcript_encoding"].as_str(),
        Some("pipe-delimited-canonical-v1")
    );
    assert_eq!(
        proof["proof_transcript_input_utf8"].as_str(),
        Some(EXPECTED_PROOF_TRANSCRIPT_INPUT)
    );

    let computed = blake3::hash(EXPECTED_PROOF_TRANSCRIPT_INPUT.as_bytes())
        .to_hex()
        .to_string();
    assert_eq!(computed, EXPECTED_PROOF_TRANSCRIPT_B3);
    assert_eq!(
        proof["proof_transcript_b3_hex"].as_str(),
        Some(EXPECTED_PROOF_TRANSCRIPT_B3)
    );

    assert_eq!(
        proof["device_signature_status"].as_str(),
        Some("pending_phase1_or_later_ed25519_device_signature_lock")
    );
    assert_eq!(
        proof["device_signature_hex"].as_str(),
        Some("PHASE1_PENDING_ED25519_DEVICE_SIGNATURE")
    );
}

#[test]
fn challenge_proof_read_only_scopes_are_safe() {
    let value = read_json("native_passport_challenge_proof_v1.json");
    let scopes = string_set(&value["challenge"]["requested_scopes"]);

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
            scopes.contains(required),
            "missing requested scope {required}"
        );
    }

    for forbidden in [
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
            !scopes.contains(forbidden),
            "forbidden scope {forbidden} must not appear in challenge proof vector"
        );
    }
}

#[test]
fn challenge_proof_excludes_human_names_pins_wallets_and_secret_material() {
    let value = read_json("native_passport_challenge_proof_v1.json");

    assert_eq!(
        value["must_be_signed_by"].as_str(),
        Some("authorized device private key")
    );
    assert_eq!(
        value["must_be_verified_with"].as_str(),
        Some("device_public_key_hex plus current DeviceAuthorizationV1")
    );

    let not_signers = string_set(&value["must_not_be_signed_by"]);
    for forbidden in [
        "Passport root private key",
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
            "challenge proof must not be signed by {forbidden}"
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
        "raw long lived capability",
    ] {
        assert!(
            not_included.contains(forbidden),
            "challenge proof fixture must not include {forbidden}"
        );
    }
}

#[test]
fn phase0g_does_not_authorize_runtime_changes() {
    let value = read_json("native_passport_challenge_proof_v1.json");
    let forbidden = string_set(&value["phase0g_forbidden_runtime_changes"]);

    for item in [
        "challenge route implementation",
        "challenge persistence",
        "replay store",
        "device signature generation",
        "device signature verification",
        "capability issuance",
        "vault encryption",
        "username finalization",
        "ledger mutation",
        "wallet mutation",
        "new Passport crate",
    ] {
        assert!(
            forbidden.contains(item),
            "Phase 0G must keep {item} forbidden"
        );
    }
}

#[test]
fn phase0b_inventory_named_challenge_proof_file_now_exists() {
    let inventory = read_json("native_passport_phase0b_inventory.json");
    let profiles = inventory["required_vector_profiles"]
        .as_array()
        .expect("required_vector_profiles must be array");

    let profile = profiles
        .iter()
        .find(|profile| profile["name"].as_str() == Some("challenge_proof_transcript_v1"))
        .expect("challenge_proof_transcript_v1 profile must exist in Phase 0B inventory");

    assert_eq!(
        profile["required_file"].as_str(),
        Some("native_passport_challenge_proof_v1.json")
    );
    assert!(vector_path("native_passport_challenge_proof_v1.json").is_file());
}
