#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 5 Round 2 DA/archive/challenge fallback pointer boundary tests for svc-index.
//! RO:WHY — Index may point to DA/archive/challenge artifacts by b3/reference only,
//! but pointers must not become DA truth, archive truth, paid unlock, pruning, settlement,
//! finality, wallet, ledger, outside-DA, or outside-chain authority.
//! RO:INVARIANTS — index entries are lookup metadata only; svc-wallet/ron-ledger remain truth.
//! RO:TEST — cargo test -p svc-index --test quickchain_phase5_da_fallback_pointer_boundary.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const POINTER_SCHEMA: &str = "svc-index.quickchain-da-fallback-pointer.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexDaFallbackPointer {
    schema: String,
    pointer_id: String,
    source: String,
    pointer_kind: String,
    da_bundle_cid: String,
    archive_cid: String,
    restore_artifact_cid: String,
    challenge_evidence_cid: String,
    retention_window_epochs: u64,
    lookup_only: bool,
    read_only: bool,
    display_only: bool,
    b3_reference_only: bool,
    evidence_reference_only: bool,
    pruning_blocked: bool,
    da_truth: bool,
    archive_restore_truth: bool,
    missing_data_challenge_truth: bool,
    pruning_authority: bool,
    paid_unlock_authority: bool,
    payment_truth: bool,
    reward_truth: bool,
    settlement_truth: bool,
    finality_truth: bool,
    wallet_side_effect: bool,
    ledger_side_effect: bool,
    balance_truth: bool,
    receipt_truth: bool,
    outside_da_truth: bool,
    outside_chain_truth: bool,
}

impl IndexDaFallbackPointer {
    fn sample() -> Self {
        Self {
            schema: POINTER_SCHEMA.to_string(),
            pointer_id: "da-fallback-pointer:phase5:r2:svc-index".to_string(),
            source: "backend-derived:svc-index".to_string(),
            pointer_kind: "da_archive_challenge_reference_pointer".to_string(),
            da_bundle_cid: format!("b3:{}", "2".repeat(64)),
            archive_cid: format!("b3:{}", "3".repeat(64)),
            restore_artifact_cid: format!("b3:{}", "4".repeat(64)),
            challenge_evidence_cid: format!("b3:{}", "5".repeat(64)),
            retention_window_epochs: 32,
            lookup_only: true,
            read_only: true,
            display_only: true,
            b3_reference_only: true,
            evidence_reference_only: true,
            pruning_blocked: true,
            da_truth: false,
            archive_restore_truth: false,
            missing_data_challenge_truth: false,
            pruning_authority: false,
            paid_unlock_authority: false,
            payment_truth: false,
            reward_truth: false,
            settlement_truth: false,
            finality_truth: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            balance_truth: false,
            receipt_truth: false,
            outside_da_truth: false,
            outside_chain_truth: false,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != POINTER_SCHEMA {
            return Err("invalid svc-index DA fallback pointer schema".to_string());
        }

        for (name, value) in [
            ("pointer_id", self.pointer_id.as_str()),
            ("source", self.source.as_str()),
            ("pointer_kind", self.pointer_kind.as_str()),
        ] {
            validate_visible_token(name, value)?;
        }

        validate_b3("da_bundle_cid", &self.da_bundle_cid)?;
        validate_b3("archive_cid", &self.archive_cid)?;
        validate_b3("restore_artifact_cid", &self.restore_artifact_cid)?;
        validate_b3("challenge_evidence_cid", &self.challenge_evidence_cid)?;

        if self.retention_window_epochs == 0 {
            return Err("retention_window_epochs must be nonzero".to_string());
        }

        if !self.lookup_only
            || !self.read_only
            || !self.display_only
            || !self.b3_reference_only
            || !self.evidence_reference_only
            || !self.pruning_blocked
        {
            return Err("svc-index DA fallback pointer must be read-only reference metadata with pruning blocked".to_string());
        }

        if self.da_truth
            || self.archive_restore_truth
            || self.missing_data_challenge_truth
            || self.pruning_authority
            || self.paid_unlock_authority
            || self.payment_truth
            || self.reward_truth
            || self.settlement_truth
            || self.finality_truth
            || self.wallet_side_effect
            || self.ledger_side_effect
            || self.balance_truth
            || self.receipt_truth
            || self.outside_da_truth
            || self.outside_chain_truth
        {
            return Err("svc-index DA fallback pointer must not carry authority flags".to_string());
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

fn collect_rust_files(path: &Path, out: &mut Vec<PathBuf>) {
    if !path.exists() {
        return;
    }

    if path.is_file() {
        if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(path.to_path_buf());
        }
        return;
    }

    let entries =
        fs::read_dir(path).unwrap_or_else(|err| panic!("read dir {}: {err}", path.display()));

    for entry in entries {
        let child = entry
            .unwrap_or_else(|err| panic!("read dir entry in {}: {err}", path.display()))
            .path();

        collect_rust_files(&child, out);
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
        "{label} must contain required Phase 5 Round 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 5 Round 2 authority marker: {needle}"
    );
}

fn assert_no_key(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert_ne!(
                    key, forbidden,
                    "svc-index DA fallback pointer must not expose forbidden authority key `{forbidden}`"
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
fn docs_name_phase5_round2_index_da_fallback_pointer_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 5 round 2 da/archive/challenge fallback pointer boundary",
        "svc-index may point to da/archive/challenge fallback artifacts by b3/reference only",
        "svc-index remains lookup and pointer metadata only",
        "da fallback artifact cid proves bytes only",
        "archive restore artifact cid proves bytes only",
        "missing-data challenge artifact cid proves bytes only",
        "retention-window metadata is display/reference metadata only",
        "svc-index does not mutate wallet or ledger from da/archive/challenge evidence",
        "svc-index does not unlock paid content from da/archive/challenge evidence",
        "svc-index is not da truth, archive restore truth, missing-data challenge truth, pruning authority, balance truth, receipt truth, finality truth, settlement truth, outside-da truth, or outside-chain truth",
        "no pruning can be approved or triggered through svc-index",
        "quickchain_phase5_da_fallback_pointer_boundary",
    ] {
        assert_contains(&doc, required, "svc-index quickchain-preflight.md");
    }
}

#[test]
fn index_da_fallback_pointer_is_lookup_only_reference_metadata() {
    let pointer = IndexDaFallbackPointer::sample();
    pointer
        .validate()
        .expect("DA fallback pointer should validate");

    assert!(pointer.lookup_only);
    assert!(pointer.read_only);
    assert!(pointer.display_only);
    assert!(pointer.b3_reference_only);
    assert!(pointer.evidence_reference_only);
    assert!(pointer.pruning_blocked);

    assert!(!pointer.da_truth);
    assert!(!pointer.archive_restore_truth);
    assert!(!pointer.missing_data_challenge_truth);
    assert!(!pointer.pruning_authority);
    assert!(!pointer.paid_unlock_authority);
    assert!(!pointer.payment_truth);
    assert!(!pointer.reward_truth);
    assert!(!pointer.settlement_truth);
    assert!(!pointer.finality_truth);
    assert!(!pointer.wallet_side_effect);
    assert!(!pointer.ledger_side_effect);
    assert!(!pointer.balance_truth);
    assert!(!pointer.receipt_truth);
    assert!(!pointer.outside_da_truth);
    assert!(!pointer.outside_chain_truth);

    let value = serde_json::to_value(&pointer).expect("pointer should serialize");

    for forbidden in [
        "wallet_receipt",
        "ledger_receipt",
        "balance_minor",
        "receipt_hash",
        "paid_unlock",
        "reward_payout",
        "settlement_status",
        "finalized",
        "prune_now",
        "pruning_receipt",
        "da_truth_proof",
        "archive_unlock_authority",
        "outside_da_settlement",
        "outside_chain_roc_truth",
    ] {
        assert_no_key(&value, forbidden);
    }
}

#[test]
fn index_da_fallback_pointer_rejects_authority_flags_unknown_fields_and_bad_b3() {
    for field in [
        "da_truth",
        "archive_restore_truth",
        "missing_data_challenge_truth",
        "pruning_authority",
        "paid_unlock_authority",
        "payment_truth",
        "reward_truth",
        "settlement_truth",
        "finality_truth",
        "wallet_side_effect",
        "ledger_side_effect",
        "balance_truth",
        "receipt_truth",
        "outside_da_truth",
        "outside_chain_truth",
    ] {
        let mut value = serde_json::to_value(IndexDaFallbackPointer::sample())
            .expect("sample should serialize");
        value
            .as_object_mut()
            .expect("sample should be object")
            .insert(field.to_string(), json!(true));

        let parsed: IndexDaFallbackPointer =
            serde_json::from_value(value).expect("known field should deserialize");
        assert!(
            parsed.validate().is_err(),
            "authority flag `{field}` must reject when true"
        );
    }

    for poison in [
        "wallet_mutation",
        "ledger_mutation",
        "paid_unlock",
        "payment_truth_proof",
        "reward_truth_proof",
        "balance_minor",
        "wallet_receipt",
        "ledger_receipt",
        "settlement_status",
        "finality_proof",
        "prune_now",
        "pruning_receipt",
        "archive_unlock_authority",
        "outside_da_settlement",
        "outside_chain_roc_truth",
    ] {
        let mut value = serde_json::to_value(IndexDaFallbackPointer::sample())
            .expect("sample should serialize");
        value
            .as_object_mut()
            .expect("sample should be object")
            .insert(poison.to_string(), json!(true));

        assert!(
            serde_json::from_value::<IndexDaFallbackPointer>(value).is_err(),
            "unknown authority field `{poison}` must reject"
        );
    }

    for field in [
        "da_bundle_cid",
        "archive_cid",
        "restore_artifact_cid",
        "challenge_evidence_cid",
    ] {
        let mut pointer = IndexDaFallbackPointer::sample();
        match field {
            "da_bundle_cid" => pointer.da_bundle_cid = "b3:ABC".to_string(),
            "archive_cid" => pointer.archive_cid = "not-b3".to_string(),
            "restore_artifact_cid" => {
                pointer.restore_artifact_cid = format!("b3:{}", "A".repeat(64))
            }
            "challenge_evidence_cid" => {
                pointer.challenge_evidence_cid = format!("b3:{}", "1".repeat(63))
            }
            _ => unreachable!("fixed test fields"),
        }

        assert!(
            pointer.validate().is_err(),
            "bad canonical b3 value in `{field}` must reject"
        );
    }
}

#[test]
fn index_route_surface_does_not_expose_phase5_da_archive_challenge_or_pruning_runtime_routes() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/http/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/router.rs"), &mut files);

    let forbidden_route_fragments = [
        ".route(\"/da",
        ".route(\"/data-availability",
        ".route(\"/archive",
        ".route(\"/carrier",
        ".route(\"/missing-data",
        ".route(\"/challenge",
        ".route(\"/retention",
        ".route(\"/prune",
        ".route(\"/pruning",
        ".route(\"/external-da",
        ".route(\"/external-settlement",
    ];

    for path in files {
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
fn index_source_does_not_construct_da_archive_challenge_or_pruning_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/http/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/pipeline"), &mut files);
    collect_rust_files(&crate_root().join("src/store"), &mut files);
    collect_rust_files(&crate_root().join("src/cache"), &mut files);
    collect_rust_files(&crate_root().join("src/types.rs"), &mut files);
    collect_rust_files(&crate_root().join("src/router.rs"), &mut files);

    let forbidden_compact_markers = [
        "index_da_truth:true",
        "\"index_da_truth\":true",
        "da_truth:true",
        "\"da_truth\":true",
        "da_paid_unlock_authority:true",
        "\"da_paid_unlock_authority\":true",
        "lookup_as_da_truth",
        "lookup_as_archive_truth",
        "index_unlock_from_da",
        "index_unlock_from_archive",
        "unlock_from_da",
        "unlock_from_archive",
        "paid_from_da",
        "paid_from_archive",
        "payment_from_da",
        "reward_from_da",
        "receipt_from_da",
        "balance_from_da",
        "finality_from_da",
        "settlement_from_da",
        "pruning_authority_from_index",
        "approve_pruning_from_index",
        "trigger_pruning_from_index",
        "prune_from_index",
        "outside_da_roc_truth",
        "outside_chain_roc_truth",
        "wallet_mutation_from_da",
        "ledger_mutation_from_da",
        "svc_wallet::",
        "ron_ledger::",
        "solana_sdk",
        "solana_client",
        "anchor_lang",
        "spl_token",
    ];

    for path in files {
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
fn index_manifest_does_not_add_phase5_round2_runtime_economy_or_settlement_dependencies() {
    let cargo = normalized(&read_rel("Cargo.toml"));

    for forbidden in [
        "ron-ledger",
        "svc-wallet",
        "ron-accounting",
        "svc-rewarder",
        "anchor-lang",
        "spl-token",
        "ethers",
        "web3",
        "solana-sdk",
        "solana-client",
    ] {
        assert_not_contains(&cargo, forbidden, "svc-index Cargo.toml");
    }
}

#[test]
fn preflight_runner_dynamically_discovers_phase5_da_fallback_pointer_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "svc-index dev-quickchain-preflight.sh");
    }
}
