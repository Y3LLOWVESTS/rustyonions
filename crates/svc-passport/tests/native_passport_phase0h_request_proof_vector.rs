use std::{collections::BTreeSet, fs, path::PathBuf};

use serde_json::Value;

const EXPECTED_CAPABILITY_INPUT: &str = "rustyonions.native-passport.capability-hash.v1|passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df|device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d|challenge:v1:b3:58577b7fde04e28ccce564ea08ed46575bb00f60924a9c1cf8d7546497b1148e|1ddbe318883cfc8b49a0c685e66b95208111cc764b2f0615b35eda5d0bf82e9c|identity.read,catalog.read,content.read,entitlement.read,receipts.read,confirmed_roc.read,capability.revoke_self|1720000400000|1720086800000";
const EXPECTED_CAPABILITY_B3: &str =
    "d462638181da2b42d29e4bc0e1a27b79e87e590dd9ebd874a53d8b4f906c4e7b";
const EXPECTED_QUERY_B3: &str = "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262";
const EXPECTED_BODY_B3: &str = "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262";
const EXPECTED_CANONICAL_PATH: &str = "/v1/passport/status/passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
const EXPECTED_REQUEST_TRANSCRIPT_INPUT: &str = "rustyonions.native-passport.request-proof.v1|d462638181da2b42d29e4bc0e1a27b79e87e590dd9ebd874a53d8b4f906c4e7b|GET|/v1/passport/status/passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df|af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262|af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262|1720000500000|202122232425262728292a2b2c2d2e2f|passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df|device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d|2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17";
const EXPECTED_REQUEST_TRANSCRIPT_B3: &str =
    "e9b20526e44fecefe6fa89c4fe9c0e24a5c4f60d7ee5c073c4fd1d8e8dcf216c";

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
fn request_proof_vector_schema_and_phase_are_locked() {
    let value = read_json("native_passport_request_proof_v1.json");

    assert_eq!(
        value["schema"].as_str(),
        Some("svc-passport.native-passport.request-proof-vector.v1")
    );
    assert_eq!(
        value["phase"].as_str(),
        Some("NATIVE_PASSPORT_PHASE0H_REQUEST_PROOF_VECTOR")
    );
    assert_eq!(
        value["status"].as_str(),
        Some("request_proof_transcript_b3_locked_no_runtime_signature")
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
    assert_eq!(
        value["request_verification_route_added"].as_bool(),
        Some(false)
    );
    assert_eq!(value["capability_issuance_added"].as_bool(), Some(false));
    assert_eq!(
        value["wallet_or_ledger_mutation_added"].as_bool(),
        Some(false)
    );
    assert_eq!(value["never_use_for_real_passport"].as_bool(), Some(true));
}

#[test]
fn request_proof_uses_locked_challenge_proof_context() {
    let challenge = read_json("native_passport_challenge_proof_v1.json");
    let request = read_json("native_passport_request_proof_v1.json");

    assert_eq!(request["passport_id"], challenge["passport_id"]);
    assert_eq!(request["device_id"], challenge["device_id"]);
    assert_eq!(
        request["device_public_key_hex"],
        challenge["device_public_key_hex"]
    );
    assert_eq!(
        request["challenge_id"],
        challenge["challenge"]["challenge_id"]
    );
    assert_eq!(
        request["challenge_proof_transcript_b3_hex"],
        challenge["proof"]["proof_transcript_b3_hex"]
    );
}

#[test]
fn phase0h_locks_synthetic_capability_hash_without_capability_issuance() {
    let value = read_json("native_passport_request_proof_v1.json");
    let capability = &value["capability"];

    assert_eq!(
        capability["status"].as_str(),
        Some("synthetic_fixture_hash_locked_no_capability_issuance")
    );
    assert_eq!(
        capability["capability_hash_input_format"].as_str(),
        Some("utf8:domain|passport_id|device_id|challenge_id|challenge_proof_transcript_b3_hex|scope_csv|issued_at_ms|expires_at_ms")
    );
    assert_eq!(
        capability["capability_hash_input_utf8"].as_str(),
        Some(EXPECTED_CAPABILITY_INPUT)
    );

    let computed = blake3::hash(EXPECTED_CAPABILITY_INPUT.as_bytes())
        .to_hex()
        .to_string();
    assert_eq!(computed, EXPECTED_CAPABILITY_B3);
    assert_eq!(
        capability["capability_hash_hex"].as_str(),
        Some(EXPECTED_CAPABILITY_B3)
    );
    assert_eq!(
        capability["raw_long_lived_capability_included"].as_bool(),
        Some(false)
    );
}

#[test]
fn phase0h_locks_request_transcript_b3_without_device_signature_runtime() {
    let value = read_json("native_passport_request_proof_v1.json");
    let request = &value["request"];

    assert_eq!(request["method"].as_str(), Some("GET"));
    assert_eq!(
        request["canonical_path"].as_str(),
        Some(EXPECTED_CANONICAL_PATH)
    );
    assert_eq!(request["canonical_query"].as_str(), Some(""));
    assert_eq!(
        request["canonical_query_hash_hex"].as_str(),
        Some(EXPECTED_QUERY_B3)
    );
    assert_eq!(request["body_hash_hex"].as_str(), Some(EXPECTED_BODY_B3));
    assert_eq!(
        request["request_transcript_status"].as_str(),
        Some("b3_locked_phase0h_no_signature")
    );
    assert_eq!(
        request["request_transcript_encoding"].as_str(),
        Some("pipe-delimited-canonical-v1")
    );
    assert_eq!(
        request["request_transcript_input_utf8"].as_str(),
        Some(EXPECTED_REQUEST_TRANSCRIPT_INPUT)
    );

    let computed = blake3::hash(EXPECTED_REQUEST_TRANSCRIPT_INPUT.as_bytes())
        .to_hex()
        .to_string();
    assert_eq!(computed, EXPECTED_REQUEST_TRANSCRIPT_B3);
    assert_eq!(
        request["request_transcript_b3_hex"].as_str(),
        Some(EXPECTED_REQUEST_TRANSCRIPT_B3)
    );
    assert_eq!(
        request["request_transcript_hex"].as_str(),
        Some(EXPECTED_REQUEST_TRANSCRIPT_B3)
    );

    assert_eq!(
        request["device_signature_status"].as_str(),
        Some("pending_phase1_or_later_ed25519_device_request_signature_lock")
    );
    assert_eq!(
        request["device_signature_hex"].as_str(),
        Some("PHASE1_PENDING_ED25519_DEVICE_REQUEST_SIGNATURE")
    );
}

#[test]
fn request_proof_scope_ceiling_is_read_only() {
    let value = read_json("native_passport_request_proof_v1.json");
    let scopes = string_set(&value["capability"]["scope_ceiling"]);

    for required in [
        "identity.read",
        "catalog.read",
        "content.read",
        "entitlement.read",
        "receipts.read",
        "confirmed_roc.read",
        "capability.revoke_self",
    ] {
        assert!(scopes.contains(required), "missing scope {required}");
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
            "forbidden scope {forbidden} must not appear"
        );
    }
}

#[test]
fn request_proof_excludes_human_names_pins_wallets_and_secret_material() {
    let value = read_json("native_passport_request_proof_v1.json");

    assert_eq!(
        value["must_be_signed_by"].as_str(),
        Some("authorized device private key")
    );
    assert_eq!(
        value["must_be_verified_with"].as_str(),
        Some("device_public_key_hex plus unexpired device-bound capability")
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
            "request proof must not be signed by {forbidden}"
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
            "request proof fixture must not include {forbidden}"
        );
    }
}

#[test]
fn phase0h_does_not_authorize_runtime_changes() {
    let value = read_json("native_passport_request_proof_v1.json");
    let forbidden = string_set(&value["phase0h_forbidden_runtime_changes"]);

    for item in [
        "request verification route",
        "request replay store",
        "device request signature generation",
        "device request signature verification",
        "capability issuance",
        "vault encryption",
        "username finalization",
        "ledger mutation",
        "wallet mutation",
        "new Passport crate",
    ] {
        assert!(
            forbidden.contains(item),
            "Phase 0H must keep {item} forbidden"
        );
    }
}

#[test]
fn phase0b_inventory_named_request_proof_file_now_exists() {
    let inventory = read_json("native_passport_phase0b_inventory.json");
    let profiles = inventory["required_vector_profiles"]
        .as_array()
        .expect("required_vector_profiles must be array");

    let profile = profiles
        .iter()
        .find(|profile| profile["name"].as_str() == Some("request_proof_v1"))
        .expect("request_proof_v1 profile must exist in Phase 0B inventory");

    assert_eq!(
        profile["required_file"].as_str(),
        Some("native_passport_request_proof_v1.json")
    );
    assert!(vector_path("native_passport_request_proof_v1.json").is_file());
}
