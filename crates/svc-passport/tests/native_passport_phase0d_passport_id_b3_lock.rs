use std::{fs, path::PathBuf};

use serde_json::Value;
use svc_passport::native_plan::{
    is_passport_id_v1_main_ed25519_b3, PASSPORT_ID_V1_B3_DIGEST_HEX_LEN,
    PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX,
};

const EXPECTED_B3_DIGEST: &str = "acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
const EXPECTED_PASSPORT_ID: &str =
    "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df";
const EXPECTED_HASH_INPUT: &str = "rustyonions.native-passport.passport-id.v1|main|ed25519|3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909";

fn root_vector_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("vectors")
        .join("native_passport_root_identity_v1.json")
}

fn root_vector() -> Value {
    let path = root_vector_path();
    let data = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    serde_json::from_str(&data)
        .unwrap_or_else(|err| panic!("failed to parse {} as JSON: {err}", path.display()))
}

#[test]
fn phase0d_locks_passport_id_b3_digest_from_hash_input() {
    let value = root_vector();
    let passport_id = &value["passport_id_derivation"];

    assert_eq!(
        value["phase"].as_str(),
        Some("NATIVE_PASSPORT_PHASE0D_PASSPORT_ID_B3_LOCK")
    );
    assert_eq!(
        value["status"].as_str(),
        Some("passport_id_b3_locked_no_runtime_crypto")
    );
    assert_eq!(
        passport_id["status"].as_str(),
        Some("digest_locked_phase0d_b3")
    );

    let hash_input = passport_id["hash_input_utf8"]
        .as_str()
        .expect("hash_input_utf8 must be string");
    assert_eq!(hash_input, EXPECTED_HASH_INPUT);

    let computed = blake3::hash(hash_input.as_bytes()).to_hex().to_string();
    assert_eq!(computed, EXPECTED_B3_DIGEST);
    assert_eq!(
        passport_id["b3_digest_hex"].as_str(),
        Some(EXPECTED_B3_DIGEST)
    );
    assert_eq!(
        passport_id["passport_id"].as_str(),
        Some(EXPECTED_PASSPORT_ID)
    );
}

#[test]
fn phase0d_locks_canonical_passport_id_v1_format() {
    assert_eq!(
        PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX,
        "passport:v1:main:ed25519:b3:"
    );
    assert_eq!(PASSPORT_ID_V1_B3_DIGEST_HEX_LEN, 64);

    assert!(is_passport_id_v1_main_ed25519_b3(EXPECTED_PASSPORT_ID));

    assert!(!is_passport_id_v1_main_ed25519_b3("passport:main:dev"));
    assert!(!is_passport_id_v1_main_ed25519_b3(
        "passport:main:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df"
    ));
    assert!(!is_passport_id_v1_main_ed25519_b3(
        "passport:v1:main:ed25519:b3:ACC2761E583FAFC93CBB880BEF1BD7285F43B3BBF326B9E185B226C5533CB7DF"
    ));
    assert!(!is_passport_id_v1_main_ed25519_b3(
        "passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7d"
    ));
}

#[test]
fn phase0d_does_not_add_runtime_authority_or_secrets() {
    let value = root_vector();

    assert_eq!(value["runtime_authority_changed"].as_bool(), Some(false));
    assert_eq!(
        value["native_secret_implementation_added"].as_bool(),
        Some(false)
    );
    assert_eq!(value["never_use_for_real_passport"].as_bool(), Some(true));

    let forbidden = value["fixture_forbidden_runtime_changes"]
        .as_array()
        .expect("fixture_forbidden_runtime_changes must be array");

    for item in [
        "mnemonic generation",
        "private key storage",
        "signature generation",
        "vault encryption",
        "challenge route implementation",
        "capability issuance",
        "username finalization",
        "ledger mutation",
        "wallet mutation",
        "new Passport crate",
    ] {
        assert!(
            forbidden.iter().any(|entry| entry.as_str() == Some(item)),
            "Phase 0D must keep {item} forbidden"
        );
    }
}
