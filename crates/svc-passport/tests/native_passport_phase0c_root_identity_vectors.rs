use std::{collections::BTreeSet, fs, path::PathBuf};

use serde_json::Value;

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

fn string_set_at<'a>(value: &'a Value, key: &str) -> BTreeSet<&'a str> {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} must be an array"))
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("{key} items must be strings"))
        })
        .collect()
}

#[test]
fn root_identity_vector_schema_and_phase_are_locked() {
    let value = root_vector();

    assert_eq!(
        value["schema"].as_str(),
        Some("svc-passport.native-passport.root-identity-vector.v1")
    );
    assert_eq!(
        value["phase"].as_str(),
        Some("NATIVE_PASSPORT_PHASE0D_PASSPORT_ID_B3_LOCK")
    );
    assert_eq!(
        value["status"].as_str(),
        Some("passport_id_b3_locked_no_runtime_crypto")
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
    assert_eq!(value["never_use_for_real_passport"].as_bool(), Some(true));
}

#[test]
fn public_bip39_fixture_material_is_locked() {
    let value = root_vector();

    assert_eq!(value["mnemonic_language"].as_str(), Some("english"));
    assert_eq!(value["mnemonic_strength_bits"].as_u64(), Some(128));
    assert_eq!(
        value["mnemonic_entropy_hex"].as_str(),
        Some("00000000000000000000000000000000")
    );

    let words = value["mnemonic_words"]
        .as_array()
        .expect("mnemonic_words must be array");

    assert_eq!(words.len(), 12);
    assert_eq!(words[0].as_str(), Some("abandon"));
    assert_eq!(words[10].as_str(), Some("abandon"));
    assert_eq!(words[11].as_str(), Some("about"));

    assert_eq!(
        value["bip39_seed_hex"].as_str(),
        Some("5eb00bbddcf069084889a8ab9155568165f5c453ccb85e70811aaed6f6da5fc19a5ac40b389cd370d086206dec8aa6c43daea6690f20ad3d8d48b2d2ce9e38e4")
    );
}

#[test]
fn pin_and_username_are_not_bip39_passphrase_inputs() {
    let value = root_vector();
    let policy = &value["bip39_passphrase_policy"];

    assert_eq!(policy["profile"].as_str(), Some("empty_string_v1"));
    assert_eq!(policy["passphrase_used_for_this_vector"].as_str(), Some(""));
    assert_eq!(
        policy["numeric_pin_used_as_bip39_passphrase"].as_bool(),
        Some(false)
    );
    assert_eq!(
        policy["username_used_as_bip39_passphrase"].as_bool(),
        Some(false)
    );
}

#[test]
fn root_derivation_reference_bytes_are_present_but_not_runtime_code() {
    let value = root_vector();
    let derivation = &value["root_derivation"];

    assert_eq!(
        derivation["status"].as_str(),
        Some("phase0c_reference_bytes_locked_for_later_implementation_tests")
    );
    assert_eq!(derivation["hkdf"].as_str(), Some("HKDF-SHA256"));
    assert_eq!(
        derivation["hkdf_salt_utf8"].as_str(),
        Some("rustyonions.native-passport.root-seed.v1")
    );
    assert_eq!(
        derivation["hkdf_info_utf8"].as_str(),
        Some("rustyonions.native-passport.root-signing-key.v1")
    );
    assert_eq!(derivation["signature_algorithm"].as_str(), Some("ed25519"));

    assert_eq!(
        derivation["root_signing_seed_hex"].as_str(),
        Some("6494f88bac1634fb4db0aaaecc6841f4dc21e4e744e2cb58378fcda7cf3d2dca")
    );
    assert_eq!(
        derivation["root_public_key_hex"].as_str(),
        Some("3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909")
    );
}

#[test]
fn passport_id_digest_is_locked_by_phase0d() {
    let value = root_vector();
    let passport_id = &value["passport_id_derivation"];

    assert_eq!(
        passport_id["status"].as_str(),
        Some("digest_locked_phase0d_b3")
    );
    assert_eq!(passport_id["algorithm"].as_str(), Some("BLAKE3-256"));
    assert_eq!(passport_id["kind"].as_str(), Some("main"));
    assert_eq!(passport_id["key_algorithm"].as_str(), Some("ed25519"));
    assert_eq!(
        passport_id["domain_tag_utf8"].as_str(),
        Some("rustyonions.native-passport.passport-id.v1")
    );
    assert_eq!(
        passport_id["hash_input_format"].as_str(),
        Some("utf8:domain|kind|key_algorithm|root_public_key_hex")
    );
    assert_eq!(
        passport_id["hash_input_utf8"].as_str(),
        Some("rustyonions.native-passport.passport-id.v1|main|ed25519|3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909")
    );
    assert_eq!(
        passport_id["passport_id"].as_str(),
        Some("passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df")
    );
}

#[test]
fn root_identity_derivation_excludes_usernames_human_names_wallets_and_vaults() {
    let value = root_vector();
    let excluded = string_set_at(&value, "must_not_participate_in_root_identity_derivation");

    for forbidden in [
        "PIN",
        "device secret",
        "device key",
        "vault key",
        "vault ciphertext",
        "username",
        "@username",
        "site name",
        "display name",
        "real human name",
        "legal name",
        "wallet account",
        "ledger account",
        "ROC balance",
    ] {
        assert!(
            excluded.contains(forbidden),
            "root identity derivation must exclude {forbidden}"
        );
    }
}

#[test]
fn phase0c_fixture_does_not_authorize_runtime_changes() {
    let value = root_vector();
    let forbidden = string_set_at(&value, "fixture_forbidden_runtime_changes");

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
            forbidden.contains(item),
            "Phase 0C fixture must keep {item} forbidden"
        );
    }
}

#[test]
fn required_root_identity_domain_tags_are_locked() {
    let value = root_vector();
    let tags = string_set_at(&value, "domain_tags");

    for required in [
        "rustyonions.native-passport.root-seed.v1",
        "rustyonions.native-passport.root-signing-key.v1",
        "rustyonions.native-passport.passport-id.v1",
    ] {
        assert!(tags.contains(required), "missing domain tag {required}");
    }
}
