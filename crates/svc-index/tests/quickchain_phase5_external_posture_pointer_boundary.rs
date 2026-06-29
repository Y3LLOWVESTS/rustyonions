#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 5 Round 3 selected external-posture pointer boundary tests for svc-index.
//! RO:WHY — Index may point to selected external posture evidence/report/status artifacts,
//! but pointers remain lookup truth only, never proof, payment, finality, settlement,
//! bridge, public-market, exchange, ROX/Solana, or outside-program authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, src/router.rs, src/http/routes, Cargo manifest.
//! RO:INVARIANTS — pointers are references; b3 proves bytes only; names/crab navigation are not authority.
//! RO:TEST — cargo test -p svc-index --test quickchain_phase5_external_posture_pointer_boundary.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const POINTER_SCHEMA: &str = "svc-index.quickchain-external-posture-pointer.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExternalPosturePointer {
    schema: String,
    pointer_id: String,
    chain_id: String,
    epoch_id: String,
    selected_posture: String,
    posture_label: String,
    anchor_commitment_cid: String,
    evidence_cid: String,
    report_cid: String,
    source_ref: String,
    lookup_only: bool,
    read_only: bool,
    display_only: bool,
    b3_reference_only: bool,
    evidence_reference_only: bool,
    proof_truth: bool,
    wallet_side_effect: bool,
    ledger_side_effect: bool,
    paid_unlock_authority: bool,
    balance_truth: bool,
    receipt_truth: bool,
    reward_truth: bool,
    finality_truth: bool,
    settlement_truth: bool,
    root_authority: bool,
    pruning_authority: bool,
    public_bridge_authority: bool,
    public_market_authority: bool,
    exchange_facing_authority: bool,
    outside_program_authority: bool,
    rox_runtime_authority: bool,
    solana_runtime_authority: bool,
}

impl ExternalPosturePointer {
    fn sample() -> Self {
        Self {
            schema: POINTER_SCHEMA.to_owned(),
            pointer_id: "external-posture-pointer:phase5:r3:index".to_owned(),
            chain_id: "roc-dev".to_owned(),
            epoch_id: "epoch:phase5:r3:index".to_owned(),
            selected_posture: "anchor-only".to_owned(),
            posture_label: "external_posture_reference_only".to_owned(),
            anchor_commitment_cid: format!("b3:{}", "5".repeat(64)),
            evidence_cid: format!("b3:{}", "6".repeat(64)),
            report_cid: format!("b3:{}", "7".repeat(64)),
            source_ref: "backend-derived:index-pointer".to_owned(),
            lookup_only: true,
            read_only: true,
            display_only: true,
            b3_reference_only: true,
            evidence_reference_only: true,
            proof_truth: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            paid_unlock_authority: false,
            balance_truth: false,
            receipt_truth: false,
            reward_truth: false,
            finality_truth: false,
            settlement_truth: false,
            root_authority: false,
            pruning_authority: false,
            public_bridge_authority: false,
            public_market_authority: false,
            exchange_facing_authority: false,
            outside_program_authority: false,
            rox_runtime_authority: false,
            solana_runtime_authority: false,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != POINTER_SCHEMA {
            return Err("invalid external posture pointer schema".to_owned());
        }

        for (name, value) in [
            ("pointer_id", self.pointer_id.as_str()),
            ("chain_id", self.chain_id.as_str()),
            ("epoch_id", self.epoch_id.as_str()),
            ("selected_posture", self.selected_posture.as_str()),
            ("posture_label", self.posture_label.as_str()),
            ("source_ref", self.source_ref.as_str()),
        ] {
            validate_visible_token(name, value)?;
        }

        if self.selected_posture != "anchor-only" {
            return Err("index selected posture pointer must remain anchor-only".to_owned());
        }

        if self.posture_label != "external_posture_reference_only" {
            return Err("index external posture label must remain reference-only".to_owned());
        }

        validate_b3("anchor_commitment_cid", &self.anchor_commitment_cid)?;
        validate_b3("evidence_cid", &self.evidence_cid)?;
        validate_b3("report_cid", &self.report_cid)?;

        for (name, value) in [
            ("lookup_only", self.lookup_only),
            ("read_only", self.read_only),
            ("display_only", self.display_only),
            ("b3_reference_only", self.b3_reference_only),
            ("evidence_reference_only", self.evidence_reference_only),
        ] {
            if !value {
                return Err(format!(
                    "{name} must be true for external posture index pointers"
                ));
            }
        }

        for (name, value) in [
            ("proof_truth", self.proof_truth),
            ("wallet_side_effect", self.wallet_side_effect),
            ("ledger_side_effect", self.ledger_side_effect),
            ("paid_unlock_authority", self.paid_unlock_authority),
            ("balance_truth", self.balance_truth),
            ("receipt_truth", self.receipt_truth),
            ("reward_truth", self.reward_truth),
            ("finality_truth", self.finality_truth),
            ("settlement_truth", self.settlement_truth),
            ("root_authority", self.root_authority),
            ("pruning_authority", self.pruning_authority),
            ("public_bridge_authority", self.public_bridge_authority),
            ("public_market_authority", self.public_market_authority),
            ("exchange_facing_authority", self.exchange_facing_authority),
            ("outside_program_authority", self.outside_program_authority),
            ("rox_runtime_authority", self.rox_runtime_authority),
            ("solana_runtime_authority", self.solana_runtime_authority),
        ] {
            if value {
                return Err(format!(
                    "{name} must remain false for external posture index pointers"
                ));
            }
        }

        Ok(())
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(path: &str) -> String {
    let full = crate_root().join(path);
    fs::read_to_string(&full).unwrap_or_else(|err| panic!("read {}: {err}", full.display()))
}

fn normalized(text: &str) -> String {
    text.to_ascii_lowercase().replace('`', "")
}

fn strip_line_comments(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with("///"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn compact_without_comments(text: &str) -> String {
    strip_line_comments(text)
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    if root.is_file() {
        if root.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(root.to_path_buf());
        }
        return;
    }

    if !root.exists() {
        return;
    }

    let entries =
        fs::read_dir(root).unwrap_or_else(|err| panic!("read dir {}: {err}", root.display()));

    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("read dir entry in {}: {err}", root.display()))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn validate_visible_token(name: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 180 {
        return Err(format!("{name} must be 1..=180 bytes"));
    }

    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/' | '@'))
    {
        return Err(format!("{name} contains unsupported characters"));
    }

    Ok(())
}

fn validate_b3(name: &str, value: &str) -> Result<(), String> {
    let Some(hex) = value.strip_prefix("b3:") else {
        return Err(format!("{name} must be b3:<64 lowercase hex>"));
    };

    if hex.len() != 64 || !hex.chars().all(|ch| matches!(ch, '0'..='9' | 'a'..='f')) {
        return Err(format!("{name} must be b3:<64 lowercase hex>"));
    }

    Ok(())
}

fn assert_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        haystack.contains(needle),
        "{label} must contain required Phase 5 Round 3 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 5 Round 3 authority marker: {needle}"
    );
}

fn assert_no_key(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert_ne!(
                    key, forbidden,
                    "external posture index pointer must not expose forbidden authority key `{forbidden}`"
                );
                assert_no_key(nested, forbidden);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_key(nested, forbidden);
            }
        }
        _ => {}
    }
}

#[test]
fn docs_name_phase5_round3_index_external_posture_pointer_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 5 round 3 selected external posture pointer boundary",
        "svc-index may point to selected external posture artifacts only as b3/reference metadata",
        "selected external posture artifact cid proves bytes only",
        "selected external posture evidence cid proves bytes only",
        "selected external posture report cid proves bytes only",
        "svc-index does not prove external posture",
        "svc-index does not mutate wallet or ledger from selected external posture metadata",
        "svc-index cannot unlock paid content from selected external posture metadata",
        "quickchain_phase5_external_posture_pointer_boundary",
    ] {
        assert_contains(&doc, required, "svc-index quickchain-preflight.md");
    }
}

#[test]
fn external_posture_pointer_is_reference_only_metadata() {
    let pointer = ExternalPosturePointer::sample();
    pointer
        .validate()
        .expect("external posture pointer should validate");

    assert!(pointer.lookup_only);
    assert!(pointer.read_only);
    assert!(pointer.display_only);
    assert!(pointer.b3_reference_only);
    assert!(pointer.evidence_reference_only);

    assert!(!pointer.proof_truth);
    assert!(!pointer.wallet_side_effect);
    assert!(!pointer.ledger_side_effect);
    assert!(!pointer.paid_unlock_authority);
    assert!(!pointer.balance_truth);
    assert!(!pointer.receipt_truth);
    assert!(!pointer.reward_truth);
    assert!(!pointer.finality_truth);
    assert!(!pointer.settlement_truth);
    assert!(!pointer.root_authority);
    assert!(!pointer.pruning_authority);
    assert!(!pointer.public_bridge_authority);
    assert!(!pointer.public_market_authority);
    assert!(!pointer.exchange_facing_authority);
    assert!(!pointer.outside_program_authority);
    assert!(!pointer.rox_runtime_authority);
    assert!(!pointer.solana_runtime_authority);

    let value = serde_json::to_value(&pointer).expect("pointer should serialize");

    for forbidden in [
        "wallet_receipt",
        "ledger_receipt",
        "balance_minor",
        "receipt_hash",
        "paid_unlock",
        "settlement_status",
        "finalized",
        "reward_payout",
        "bridge_settlement",
        "outside_program_execution",
        "public_market",
        "exchange_settlement",
        "solana_runtime",
        "rox_runtime",
    ] {
        assert_no_key(&value, forbidden);
    }
}

#[test]
fn external_posture_pointer_rejects_authority_flags_unknown_fields_and_bad_b3() {
    for field in [
        "proof_truth",
        "wallet_side_effect",
        "ledger_side_effect",
        "paid_unlock_authority",
        "balance_truth",
        "receipt_truth",
        "reward_truth",
        "finality_truth",
        "settlement_truth",
        "root_authority",
        "pruning_authority",
        "public_bridge_authority",
        "public_market_authority",
        "exchange_facing_authority",
        "outside_program_authority",
        "rox_runtime_authority",
        "solana_runtime_authority",
    ] {
        let mut value = serde_json::to_value(ExternalPosturePointer::sample())
            .expect("sample should serialize");
        value
            .as_object_mut()
            .expect("sample should be an object")
            .insert(field.to_owned(), json!(true));

        let parsed: ExternalPosturePointer =
            serde_json::from_value(value).expect("known field should deserialize");
        assert!(
            parsed.validate().is_err(),
            "authority flag `{field}` must reject when true"
        );
    }

    for poison in [
        "external_posture_truth",
        "external_posture_authority",
        "external_posture_paid_unlock",
        "external_posture_balance_truth",
        "external_posture_receipt_truth",
        "external_posture_reward_truth",
        "external_posture_finality",
        "external_posture_settlement",
        "external_posture_root_authority",
        "external_posture_pruning_authority",
        "outside_program_authority_claim",
        "outside_program_execution_truth",
        "public_chain_settlement_truth",
        "public_market_authority_claim",
        "exchange_facing_authority_claim",
        "rox_runtime_truth",
        "solana_runtime_truth",
    ] {
        let mut value = serde_json::to_value(ExternalPosturePointer::sample())
            .expect("sample should serialize");
        value
            .as_object_mut()
            .expect("sample should be an object")
            .insert(poison.to_owned(), json!(true));

        assert!(
            serde_json::from_value::<ExternalPosturePointer>(value).is_err(),
            "unknown authority field `{poison}` must be rejected"
        );
    }

    for field in ["anchor_commitment_cid", "evidence_cid", "report_cid"] {
        let mut pointer = ExternalPosturePointer::sample();
        match field {
            "anchor_commitment_cid" => pointer.anchor_commitment_cid = "b3:ABC".to_owned(),
            "evidence_cid" => pointer.evidence_cid = "not-b3".to_owned(),
            "report_cid" => pointer.report_cid = format!("b3:{}", "g".repeat(64)),
            _ => unreachable!("test field set is fixed"),
        }

        assert!(
            pointer.validate().is_err(),
            "bad canonical b3 value in `{field}` must reject"
        );
    }
}

#[test]
fn index_route_surface_does_not_expose_phase5_external_posture_runtime_routes() {
    let mut files = vec![crate_root().join("src/router.rs")];
    collect_rust_files(&crate_root().join("src/http/routes"), &mut files);

    let forbidden_route_fragments = [
        "\"/external-posture",
        "\"/external-settlement",
        "\"/outside-program",
        "\"/public-chain",
        "\"/public-market",
        "\"/exchange",
        "\"/bridge",
        "\"/rox",
        "\"/solana",
        "\"/staking",
        "\"/stake",
        "\"/liquidity",
        "\"/market",
        "\"/settlement",
        "\"/finality",
    ];

    for path in files {
        if !path.exists() {
            continue;
        }

        let source = normalized(&strip_line_comments(
            &fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("read route source {}: {err}", path.display())),
        ));

        for forbidden in forbidden_route_fragments {
            assert_not_contains(
                &source,
                forbidden,
                &format!("svc-index route source {}", path.display()),
            );
        }
    }
}

#[test]
fn index_source_does_not_construct_phase5_external_posture_runtime_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/http/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/pipeline"), &mut files);
    collect_rust_files(&crate_root().join("src/store"), &mut files);
    collect_rust_files(&crate_root().join("src/router.rs"), &mut files);

    let forbidden_compact_markers = [
        "external_posture_truth:true",
        "\"external_posture_truth\":true",
        "external_posture_authority:true",
        "\"external_posture_authority\":true",
        "external_posture_paid_unlock:true",
        "\"external_posture_paid_unlock\":true",
        "external_posture_balance_truth:true",
        "\"external_posture_balance_truth\":true",
        "external_posture_receipt_truth:true",
        "\"external_posture_receipt_truth\":true",
        "external_posture_reward_truth:true",
        "\"external_posture_reward_truth\":true",
        "external_posture_finality:true",
        "\"external_posture_finality\":true",
        "external_posture_settlement:true",
        "\"external_posture_settlement\":true",
        "external_posture_root_authority:true",
        "\"external_posture_root_authority\":true",
        "external_posture_pruning_authority:true",
        "\"external_posture_pruning_authority\":true",
        "outside_program_authority:true",
        "\"outside_program_authority\":true",
        "public_market_authority:true",
        "\"public_market_authority\":true",
        "exchange_facing_authority:true",
        "\"exchange_facing_authority\":true",
        "rox_runtime_authority:true",
        "\"rox_runtime_authority\":true",
        "solana_runtime_authority:true",
        "\"solana_runtime_authority\":true",
        "index_proves_external_posture(",
        "index_grants_external_posture_authority(",
        "index_grants_outside_program_authority(",
        "index_grants_public_market_authority(",
        "index_grants_exchange_authority(",
        "index_grants_rox_runtime_authority(",
        "index_grants_solana_runtime_authority(",
        "unlock_from_external_posture(",
        "paid_from_external_posture(",
        "receipt_from_external_posture(",
        "balance_from_external_posture(",
        "reward_from_external_posture(",
        "finality_from_external_posture(",
        "settle_from_external_posture(",
        "execute_outside_program(",
        "bridge_settlement(",
        "external_settlement(",
        "mint_rox(",
        "rox_settlement(",
        "solana_settlement(",
    ];

    for path in files {
        if !path.exists() {
            continue;
        }

        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        let compact = compact_without_comments(&source);

        for forbidden in forbidden_compact_markers {
            assert_not_contains(
                &compact,
                forbidden,
                &format!("svc-index source {}", path.display()),
            );
        }
    }
}

#[test]
fn index_manifest_does_not_add_phase5_external_posture_runtime_dependencies() {
    let cargo = normalized(&read_rel("Cargo.toml"));

    for forbidden in [
        "ron-ledger",
        "svc-wallet",
        "ron-accounting",
        "svc-rewarder",
        "ron-policy",
        "anchor-lang",
        "spl-token",
        "ethers",
        "web3",
        "alloy",
        "solana-sdk",
        "solana-client",
    ] {
        assert_not_contains(&cargo, forbidden, "svc-index Cargo.toml");
    }
}

#[test]
fn index_allows_external_posture_reference_language_without_promoting_it_to_authority() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for allowed_reference_phrase in [
        "svc-index may point to selected external posture artifacts only as b3/reference metadata",
        "selected external posture artifact cid proves bytes only",
        "selected external posture evidence cid proves bytes only",
        "selected external posture report cid proves bytes only",
    ] {
        assert_contains(
            &doc,
            allowed_reference_phrase,
            "svc-index docs should allow Round 3 reference-only external posture language",
        );
    }

    for forbidden_authority_phrase in [
        "index pointer proves external posture",
        "index pointer proves outside program",
        "index pointer grants public market",
        "index pointer grants exchange",
        "index pointer grants rox runtime",
        "index pointer grants solana runtime",
        "index pointer unlocks paid content from external posture",
        "index pointer finalizes external posture settlement",
    ] {
        assert_not_contains(
            &doc,
            forbidden_authority_phrase,
            "svc-index docs must not promote external posture pointer authority",
        );
    }
}

#[test]
fn preflight_runner_dynamically_discovers_phase5_external_posture_pointer_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "svc-index dev-quickchain-preflight.sh");
    }
}
