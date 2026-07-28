use std::{collections::BTreeSet, fs, path::PathBuf};

use serde_json::Value;

fn inventory_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("vectors")
        .join("native_passport_phase0b_inventory.json")
}

fn inventory() -> Value {
    let path = inventory_path();
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
fn vector_inventory_schema_and_phase_are_locked() {
    let value = inventory();

    assert_eq!(
        value["schema"].as_str(),
        Some("svc-passport.native-passport.phase0b-vector-inventory.v1")
    );
    assert_eq!(
        value["phase"].as_str(),
        Some("NATIVE_PASSPORT_PHASE0B_VECTOR_INVENTORY")
    );
    assert_eq!(
        value["status"].as_str(),
        Some("spec_inventory_locked_no_crypto_runtime")
    );
    assert_eq!(
        value["canonical_passport_package_owner"].as_str(),
        Some("svc-passport")
    );
    assert_eq!(value["new_passport_crate_allowed"].as_bool(), Some(false));
}

#[test]
fn phase0b_inventory_does_not_claim_runtime_authority() {
    let value = inventory();

    assert_eq!(value["runtime_authority_changed"].as_bool(), Some(false));
    assert_eq!(
        value["native_secret_implementation_added"].as_bool(),
        Some(false)
    );

    let forbidden = string_set_at(&value, "phase0b_forbidden_runtime_changes");

    for item in [
        "mnemonic generation",
        "seed derivation",
        "private key creation",
        "signature generation",
        "vault encryption",
        "challenge route implementation",
        "capability issuance",
        "ledger mutation",
        "wallet mutation",
        "new Passport crate",
    ] {
        assert!(
            forbidden.contains(item),
            "Phase 0B inventory must keep {item} forbidden"
        );
    }
}

#[test]
fn vector_profiles_cover_root_device_proof_request_and_vault() {
    let value = inventory();
    let profiles = value["required_vector_profiles"]
        .as_array()
        .expect("required_vector_profiles must be an array");

    let names: BTreeSet<&str> = profiles
        .iter()
        .map(|profile| {
            profile["name"]
                .as_str()
                .expect("profile name must be string")
        })
        .collect();

    for required in [
        "root_identity_derivation_v1",
        "device_authorization_v1",
        "challenge_proof_transcript_v1",
        "request_proof_v1",
        "vault_header_v1",
    ] {
        assert!(
            names.contains(required),
            "missing required vector profile {required}"
        );
    }
}

#[test]
fn vector_files_are_named_before_crypto_implementation() {
    let value = inventory();
    let profiles = value["required_vector_profiles"]
        .as_array()
        .expect("required_vector_profiles must be an array");

    let required_files: BTreeSet<&str> = profiles
        .iter()
        .map(|profile| {
            profile["required_file"]
                .as_str()
                .expect("required_file must be string")
        })
        .collect();

    for required in [
        "native_passport_root_identity_v1.json",
        "native_passport_device_authorization_v1.json",
        "native_passport_challenge_proof_v1.json",
        "native_passport_request_proof_v1.json",
        "native_passport_vault_header_v1.json",
    ] {
        assert!(
            required_files.contains(required),
            "missing required vector file name {required}"
        );
    }
}

#[test]
fn username_terms_do_not_treat_real_human_names_as_identifiers() {
    let value = inventory();
    let terms = &value["network_name_terms"];

    assert_eq!(terms["username_handle_term"].as_str(), Some("@username"));
    assert_eq!(
        terms["real_or_legal_human_name_identifier_allowed"].as_bool(),
        Some(false)
    );

    let rule = terms["rule"].as_str().expect("rule must be present");
    assert!(
        rule.contains("@username and site name are network identifiers"),
        "rule must identify @username and site names as the network identifiers"
    );
    assert!(
        rule.contains("real or legal human names are optional profile text only"),
        "rule must keep real/legal human names out of identifier authority"
    );

    let root_profile = value["required_vector_profiles"]
        .as_array()
        .expect("required_vector_profiles must be array")
        .iter()
        .find(|profile| profile["name"].as_str() == Some("root_identity_derivation_v1"))
        .expect("root identity profile must exist");

    let must_not_include: BTreeSet<&str> = root_profile["must_not_include"]
        .as_array()
        .expect("must_not_include must be array")
        .iter()
        .map(|item| item.as_str().expect("must_not_include item must be string"))
        .collect();

    for forbidden in [
        "username",
        "@username",
        "display_name",
        "real_human_name",
        "legal_name",
    ] {
        assert!(
            must_not_include.contains(forbidden),
            "root identity vectors must not include {forbidden}"
        );
    }
}

#[test]
fn initial_scope_sets_are_safe_and_non_overlapping() {
    let value = inventory();
    let read_only = string_set_at(&value, "initial_read_only_scopes");
    let forbidden = string_set_at(&value, "forbidden_initial_scopes");

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
            read_only.contains(required),
            "missing initial read-only scope {required}"
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
            "missing forbidden initial scope {denied}"
        );
        assert!(
            !read_only.contains(denied),
            "forbidden scope {denied} must not be read-only"
        );
    }
}
