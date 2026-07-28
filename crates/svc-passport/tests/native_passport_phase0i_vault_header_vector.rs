use std::{collections::BTreeSet, fs, path::PathBuf};

use serde_json::Value;

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

fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn vault_header_vector_schema_and_phase_are_locked() {
    let value = read_json("native_passport_vault_header_v1.json");

    assert_eq!(
        value["schema"].as_str(),
        Some("svc-passport.native-passport.vault-header-vector.v1")
    );
    assert_eq!(
        value["phase"].as_str(),
        Some("NATIVE_PASSPORT_PHASE0I_VAULT_HEADER_VECTOR")
    );
    assert_eq!(
        value["status"].as_str(),
        Some("vault_header_fixture_contract_locked_no_encryption_runtime")
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
    assert_eq!(value["vault_encryption_added"].as_bool(), Some(false));
    assert_eq!(value["pin_handling_added"].as_bool(), Some(false));
    assert_eq!(value["platform_secret_access_added"].as_bool(), Some(false));
    assert_eq!(
        value["wallet_or_ledger_mutation_added"].as_bool(),
        Some(false)
    );
    assert_eq!(value["never_use_for_real_passport"].as_bool(), Some(true));
}

#[test]
fn vault_header_has_root_and_device_compartments() {
    let value = read_json("native_passport_vault_header_v1.json");
    let compartments = value["compartments"]
        .as_array()
        .expect("compartments must be array");

    assert_eq!(compartments.len(), 2);

    let kinds: BTreeSet<&str> = compartments
        .iter()
        .map(|item| {
            item["compartment_kind"]
                .as_str()
                .expect("compartment_kind must be string")
        })
        .collect();

    assert!(kinds.contains("passport_root_compartment"));
    assert!(kinds.contains("device_compartment"));
}

#[test]
fn vault_header_required_fields_are_present_for_each_compartment() {
    let value = read_json("native_passport_vault_header_v1.json");
    let compartments = value["compartments"]
        .as_array()
        .expect("compartments must be array");

    for compartment in compartments {
        assert_eq!(
            compartment["vault_schema"].as_str(),
            Some("RustyOnionsNativePassportVaultHeaderV1")
        );
        assert_eq!(compartment["vault_format_version"].as_u64(), Some(1));
        assert_eq!(compartment["kdf"].as_str(), Some("Argon2id"));
        assert_eq!(compartment["kdf_version"].as_str(), Some("v1"));
        assert_eq!(compartment["aead"].as_str(), Some("XChaCha20Poly1305"));

        let salt_hex = compartment["salt_hex"]
            .as_str()
            .expect("salt_hex must exist");
        let nonce_hex = compartment["nonce_hex"]
            .as_str()
            .expect("nonce_hex must exist");

        assert_eq!(salt_hex.len(), 64, "salt must be 32 bytes of hex");
        assert_eq!(
            nonce_hex.len(),
            48,
            "XChaCha20Poly1305 nonce must be 24 bytes of hex"
        );
        assert!(salt_hex.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(nonce_hex.bytes().all(|b| b.is_ascii_hexdigit()));

        assert_eq!(compartment["ciphertext_included"].as_bool(), Some(false));
        assert_eq!(
            compartment["secret_material_included"].as_bool(),
            Some(false)
        );
    }
}

#[test]
fn kdf_parameters_are_bounded_and_compartment_specific() {
    let value = read_json("native_passport_vault_header_v1.json");
    let compartments = value["compartments"]
        .as_array()
        .expect("compartments must be array");

    let root = compartments
        .iter()
        .find(|item| item["compartment_kind"].as_str() == Some("passport_root_compartment"))
        .expect("root compartment must exist");
    let device = compartments
        .iter()
        .find(|item| item["compartment_kind"].as_str() == Some("device_compartment"))
        .expect("device compartment must exist");

    assert_eq!(root["kdf_params"]["memory_mib"].as_u64(), Some(64));
    assert_eq!(root["kdf_params"]["time_cost"].as_u64(), Some(3));
    assert_eq!(root["kdf_params"]["parallelism"].as_u64(), Some(1));
    assert_eq!(root["kdf_params"]["output_len_bytes"].as_u64(), Some(32));

    assert_eq!(device["kdf_params"]["memory_mib"].as_u64(), Some(32));
    assert_eq!(device["kdf_params"]["time_cost"].as_u64(), Some(3));
    assert_eq!(device["kdf_params"]["parallelism"].as_u64(), Some(1));
    assert_eq!(device["kdf_params"]["output_len_bytes"].as_u64(), Some(32));

    assert_ne!(root["salt_hex"], device["salt_hex"]);
    assert_ne!(root["nonce_hex"], device["nonce_hex"]);
}

#[test]
fn authenticated_header_hex_matches_canonical_header_input() {
    let value = read_json("native_passport_vault_header_v1.json");
    let compartments = value["compartments"]
        .as_array()
        .expect("compartments must be array");

    for compartment in compartments {
        assert_eq!(
            compartment["authenticated_header_encoding"].as_str(),
            Some("pipe-delimited-canonical-v1")
        );
        assert_eq!(
            compartment["authenticated_header_input_format"].as_str(),
            Some("utf8:domain|version|compartment_kind|kdf|kdf_version|kdf_params|salt_hex|aead|nonce_hex|compartment_purpose|domain_tag")
        );

        let input = compartment["authenticated_header_input_utf8"]
            .as_str()
            .expect("authenticated_header_input_utf8 must be string");
        let expected_hex = lower_hex(input.as_bytes());

        assert_eq!(
            compartment["authenticated_header_hex"].as_str(),
            Some(expected_hex.as_str())
        );

        assert!(input.contains("rustyonions.native-passport.vault-header.v1"));
        assert!(input.contains(compartment["compartment_kind"].as_str().unwrap()));
        assert!(input.contains(compartment["salt_hex"].as_str().unwrap()));
        assert!(input.contains(compartment["nonce_hex"].as_str().unwrap()));
    }
}

#[test]
fn vault_domain_tags_are_separate_for_root_and_device_material() {
    let value = read_json("native_passport_vault_header_v1.json");
    let compartments = value["compartments"]
        .as_array()
        .expect("compartments must be array");

    let tags: BTreeSet<&str> = compartments
        .iter()
        .map(|item| {
            item["domain_tag_utf8"]
                .as_str()
                .expect("domain tag must exist")
        })
        .collect();

    assert!(tags.contains("rustyonions.native-passport.vault-root-compartment.v1"));
    assert!(tags.contains("rustyonions.native-passport.vault-device-compartment.v1"));
}

#[test]
fn vault_header_excludes_secret_material_pins_wallets_and_human_names() {
    let value = read_json("native_passport_vault_header_v1.json");
    let forbidden = string_set(&value["must_not_include"]);

    for item in [
        "mnemonic words",
        "bip39 seed",
        "root private key",
        "root signing seed",
        "device private key",
        "PIN",
        "vault master key",
        "derived vault key",
        "platform device secret",
        "wallet spend authority",
        "ledger mutation authority",
        "raw long lived capability",
        "real human name",
        "legal name",
    ] {
        assert!(
            forbidden.contains(item),
            "vault header fixture must not include {item}"
        );
    }
}

#[test]
fn phase0i_does_not_authorize_runtime_vault_behavior() {
    let value = read_json("native_passport_vault_header_v1.json");
    let forbidden = string_set(&value["phase0i_forbidden_runtime_changes"]);

    for item in [
        "vault encryption",
        "vault decryption",
        "PIN capture",
        "PIN verification",
        "platform keystore access",
        "mnemonic storage",
        "root private key storage",
        "device private key storage",
        "capability issuance",
        "ledger mutation",
        "wallet mutation",
        "new Passport crate",
    ] {
        assert!(
            forbidden.contains(item),
            "Phase 0I must keep {item} forbidden"
        );
    }
}

#[test]
fn phase0b_inventory_named_vault_header_file_now_exists() {
    let inventory = read_json("native_passport_phase0b_inventory.json");
    let profiles = inventory["required_vector_profiles"]
        .as_array()
        .expect("required_vector_profiles must be array");

    let profile = profiles
        .iter()
        .find(|profile| profile["name"].as_str() == Some("vault_header_v1"))
        .expect("vault_header_v1 profile must exist in Phase 0B inventory");

    assert_eq!(
        profile["required_file"].as_str(),
        Some("native_passport_vault_header_v1.json")
    );
    assert!(vector_path("native_passport_vault_header_v1.json").is_file());
}
