use std::{collections::BTreeSet, fs, path::PathBuf};

use serde_json::Value;
use svc_passport::native_plan::{
    is_device_id_v1_ed25519_b3, is_passport_id_v1_main_ed25519_b3, PassportRoutePosture,
    CURRENT_ROUTE_AUTHORITIES, PLANNED_NATIVE_V1_ROUTE_AUTHORITIES,
};

const VECTOR_FILES: &[&str] = &[
    "native_passport_root_identity_v1.json",
    "native_passport_device_authorization_v1.json",
    "native_passport_challenge_proof_v1.json",
    "native_passport_request_proof_v1.json",
    "native_passport_vault_header_v1.json",
];

fn vector_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("vectors")
        .join(file_name)
}

fn read_text(file_name: &str) -> String {
    let path = vector_path(file_name);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn read_json(file_name: &str) -> Value {
    let path = vector_path(file_name);
    let data = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    serde_json::from_str(&data)
        .unwrap_or_else(|err| panic!("failed to parse {} as JSON: {err}", path.display()))
}

fn b3(input: &str) -> String {
    blake3::hash(input.as_bytes()).to_hex().to_string()
}

fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn string_set(value: &Value) -> BTreeSet<&str> {
    value
        .as_array()
        .expect("value must be array")
        .iter()
        .map(|item| item.as_str().expect("array items must be strings"))
        .collect()
}

fn assert_false_if_present(value: &Value, key: &str, file_name: &str) {
    if let Some(flag) = value.get(key) {
        assert_eq!(
            flag.as_bool(),
            Some(false),
            "{file_name} must keep {key}=false in Phase 0"
        );
    }
}

#[test]
fn phase0_acceptance_all_inventory_named_vector_files_exist() {
    let inventory = read_json("native_passport_phase0b_inventory.json");
    assert_eq!(
        inventory["schema"].as_str(),
        Some("svc-passport.native-passport.phase0b-vector-inventory.v1")
    );
    assert_eq!(
        inventory["canonical_passport_package_owner"].as_str(),
        Some("svc-passport")
    );
    assert_eq!(
        inventory["new_passport_crate_allowed"].as_bool(),
        Some(false)
    );

    let profiles = inventory["required_vector_profiles"]
        .as_array()
        .expect("required_vector_profiles must be array");

    let required_files: BTreeSet<&str> = profiles
        .iter()
        .map(|profile| {
            profile["required_file"]
                .as_str()
                .expect("required_file must be string")
        })
        .collect();

    for file_name in VECTOR_FILES {
        assert!(
            required_files.contains(file_name),
            "Phase 0B inventory must name {file_name}"
        );
        assert!(
            vector_path(file_name).is_file(),
            "inventory-named vector file must exist: {file_name}"
        );
    }
}

#[test]
fn phase0_acceptance_route_inventory_keeps_current_and_planned_authority_separate() {
    assert!(
        !CURRENT_ROUTE_AUTHORITIES.is_empty(),
        "current route authority inventory must not be empty"
    );

    for route in CURRENT_ROUTE_AUTHORITIES {
        assert!(
            !route.native_v1_authority,
            "{} {} must not claim Native Passport V1 authority in Phase 0",
            route.method, route.path
        );
        assert_ne!(
            route.posture,
            PassportRoutePosture::PlannedNativeV1,
            "current route inventory must not contain planned Native V1 posture"
        );
    }

    assert_eq!(PLANNED_NATIVE_V1_ROUTE_AUTHORITIES.len(), 5);

    for route in PLANNED_NATIVE_V1_ROUTE_AUTHORITIES {
        assert_eq!(route.posture, PassportRoutePosture::PlannedNativeV1);
        assert!(
            route.native_v1_authority,
            "{} {} should remain planned Native V1 authority",
            route.method, route.path
        );
    }
}

#[test]
fn phase0_acceptance_vectors_keep_no_runtime_authority_posture() {
    for file_name in VECTOR_FILES {
        let value = read_json(file_name);

        assert_eq!(
            value["canonical_passport_package_owner"].as_str(),
            Some("svc-passport"),
            "{file_name} must keep svc-passport as canonical owner"
        );
        assert_eq!(
            value["runtime_authority_changed"].as_bool(),
            Some(false),
            "{file_name} must not claim runtime authority"
        );
        assert_eq!(
            value["native_secret_implementation_added"].as_bool(),
            Some(false),
            "{file_name} must not add native secret implementation"
        );
        assert_eq!(
            value["never_use_for_real_passport"].as_bool(),
            Some(true),
            "{file_name} must stay marked as fixture-only"
        );

        for flag in [
            "signature_generation_added",
            "challenge_route_added",
            "request_verification_route_added",
            "capability_issuance_added",
            "vault_encryption_added",
            "pin_handling_added",
            "platform_secret_access_added",
            "wallet_or_ledger_mutation_added",
        ] {
            assert_false_if_present(&value, flag, file_name);
        }
    }
}

#[test]
fn phase0_acceptance_identity_chain_links_across_vectors() {
    let root = read_json("native_passport_root_identity_v1.json");
    let device = read_json("native_passport_device_authorization_v1.json");
    let challenge = read_json("native_passport_challenge_proof_v1.json");
    let request = read_json("native_passport_request_proof_v1.json");

    let passport_id = root["passport_id_derivation"]["passport_id"]
        .as_str()
        .expect("root passport id must exist");
    let root_public_key = root["root_derivation"]["root_public_key_hex"]
        .as_str()
        .expect("root public key must exist");
    let device_id = device["device"]["device_id"]
        .as_str()
        .expect("device id must exist");
    let device_public_key = device["device"]["device_public_key_hex"]
        .as_str()
        .expect("device public key must exist");
    let device_auth_hash = device["authorization"]["authorization_transcript_b3_hex"]
        .as_str()
        .expect("device authorization transcript hash must exist");
    let challenge_id = challenge["challenge"]["challenge_id"]
        .as_str()
        .expect("challenge id must exist");
    let challenge_proof_hash = challenge["proof"]["proof_transcript_b3_hex"]
        .as_str()
        .expect("challenge proof transcript hash must exist");

    assert!(is_passport_id_v1_main_ed25519_b3(passport_id));
    assert!(is_device_id_v1_ed25519_b3(device_id));

    assert_eq!(device["passport_id"].as_str(), Some(passport_id));
    assert_eq!(
        device["root_public_key_hex"].as_str(),
        Some(root_public_key)
    );

    assert_eq!(challenge["passport_id"].as_str(), Some(passport_id));
    assert_eq!(challenge["device_id"].as_str(), Some(device_id));
    assert_eq!(
        challenge["device_public_key_hex"].as_str(),
        Some(device_public_key)
    );
    assert_eq!(
        challenge["device_authorization_hash_hex"].as_str(),
        Some(device_auth_hash)
    );

    assert_eq!(request["passport_id"].as_str(), Some(passport_id));
    assert_eq!(request["device_id"].as_str(), Some(device_id));
    assert_eq!(
        request["device_public_key_hex"].as_str(),
        Some(device_public_key)
    );
    assert_eq!(request["challenge_id"].as_str(), Some(challenge_id));
    assert_eq!(
        request["challenge_proof_transcript_b3_hex"].as_str(),
        Some(challenge_proof_hash)
    );
}

#[test]
fn phase0_acceptance_recomputes_all_locked_b3_values() {
    let root = read_json("native_passport_root_identity_v1.json");
    let device = read_json("native_passport_device_authorization_v1.json");
    let challenge = read_json("native_passport_challenge_proof_v1.json");
    let request = read_json("native_passport_request_proof_v1.json");

    let root_passport_id_input = root["passport_id_derivation"]["hash_input_utf8"]
        .as_str()
        .expect("root passport id input must exist");
    assert_eq!(
        b3(root_passport_id_input),
        root["passport_id_derivation"]["b3_digest_hex"]
            .as_str()
            .expect("root passport digest must exist")
    );

    let device_id_input = device["device"]["device_id_hash_input_utf8"]
        .as_str()
        .expect("device id input must exist");
    assert_eq!(
        b3(device_id_input),
        device["device"]["device_id_b3_digest_hex"]
            .as_str()
            .expect("device id digest must exist")
    );

    let device_auth_input = device["authorization"]["authorization_transcript_input_utf8"]
        .as_str()
        .expect("device authorization transcript input must exist");
    assert_eq!(
        b3(device_auth_input),
        device["authorization"]["authorization_transcript_b3_hex"]
            .as_str()
            .expect("device authorization digest must exist")
    );

    let challenge_id_input = challenge["challenge"]["challenge_id_hash_input_utf8"]
        .as_str()
        .expect("challenge id input must exist");
    assert_eq!(
        b3(challenge_id_input),
        challenge["challenge"]["challenge_id_b3_digest_hex"]
            .as_str()
            .expect("challenge id digest must exist")
    );

    let proof_input = challenge["proof"]["proof_transcript_input_utf8"]
        .as_str()
        .expect("challenge proof transcript input must exist");
    assert_eq!(
        b3(proof_input),
        challenge["proof"]["proof_transcript_b3_hex"]
            .as_str()
            .expect("challenge proof digest must exist")
    );

    let capability_input = request["capability"]["capability_hash_input_utf8"]
        .as_str()
        .expect("capability input must exist");
    assert_eq!(
        b3(capability_input),
        request["capability"]["capability_hash_hex"]
            .as_str()
            .expect("capability hash must exist")
    );

    assert_eq!(
        b3(request["request"]["canonical_query"].as_str().unwrap_or("")),
        request["request"]["canonical_query_hash_hex"]
            .as_str()
            .expect("query hash must exist")
    );
    assert_eq!(
        b3(""),
        request["request"]["body_hash_hex"]
            .as_str()
            .expect("body hash must exist")
    );

    let request_input = request["request"]["request_transcript_input_utf8"]
        .as_str()
        .expect("request transcript input must exist");
    assert_eq!(
        b3(request_input),
        request["request"]["request_transcript_b3_hex"]
            .as_str()
            .expect("request transcript hash must exist")
    );
}

#[test]
fn phase0_acceptance_vault_headers_are_canonical_without_encryption_runtime() {
    let vault = read_json("native_passport_vault_header_v1.json");

    assert_eq!(vault["vault_encryption_added"].as_bool(), Some(false));
    assert_eq!(vault["pin_handling_added"].as_bool(), Some(false));
    assert_eq!(vault["platform_secret_access_added"].as_bool(), Some(false));

    let compartments = vault["compartments"]
        .as_array()
        .expect("compartments must be array");
    assert_eq!(compartments.len(), 2);

    let kinds: BTreeSet<&str> = compartments
        .iter()
        .map(|item| item["compartment_kind"].as_str().unwrap())
        .collect();

    assert!(kinds.contains("passport_root_compartment"));
    assert!(kinds.contains("device_compartment"));

    for compartment in compartments {
        assert_eq!(compartment["ciphertext_included"].as_bool(), Some(false));
        assert_eq!(
            compartment["secret_material_included"].as_bool(),
            Some(false)
        );

        let input = compartment["authenticated_header_input_utf8"]
            .as_str()
            .expect("authenticated header input must exist");
        let expected_hex = lower_hex(input.as_bytes());

        assert_eq!(
            compartment["authenticated_header_hex"].as_str(),
            Some(expected_hex.as_str())
        );
    }
}

#[test]
fn phase0_acceptance_secret_and_ambiguous_placeholders_are_absent() {
    for file_name in VECTOR_FILES {
        let text = read_text(file_name);

        for forbidden in [
            "PENDING_COMPUTE_AFTER_INPUT_LOCK",
            "PHASE0D_PENDING_BLAKE3_DIGEST",
            "PHASE0F_PENDING_DEVICE_ID_B3_LOCK",
            "PHASE0F_PENDING_AUTHORIZATION_TRANSCRIPT_B3_LOCK",
            concat!("human names as signed ", "mutable pointers"),
        ] {
            assert!(
                !text.contains(forbidden),
                "{file_name} must not contain forbidden placeholder/wording {forbidden}"
            );
        }
    }
}

#[test]
fn phase0_acceptance_forbidden_runtime_changes_remain_declared() {
    let forbidden_keys = [
        (
            "native_passport_root_identity_v1.json",
            "fixture_forbidden_runtime_changes",
        ),
        (
            "native_passport_device_authorization_v1.json",
            "phase0e_forbidden_runtime_changes",
        ),
        (
            "native_passport_challenge_proof_v1.json",
            "phase0g_forbidden_runtime_changes",
        ),
        (
            "native_passport_request_proof_v1.json",
            "phase0h_forbidden_runtime_changes",
        ),
        (
            "native_passport_vault_header_v1.json",
            "phase0i_forbidden_runtime_changes",
        ),
    ];

    for (file_name, key) in forbidden_keys {
        let value = read_json(file_name);
        let forbidden = string_set(&value[key]);

        for required in [
            "capability issuance",
            "ledger mutation",
            "wallet mutation",
            "new Passport crate",
        ] {
            assert!(
                forbidden.contains(required),
                "{file_name} must keep {required} forbidden"
            );
        }
    }
}
