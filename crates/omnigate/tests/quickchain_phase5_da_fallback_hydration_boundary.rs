#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 5 Round 2 DA/archive/challenge fallback hydration boundary tests for omnigate.
//! RO:WHY — Omnigate may hydrate backend-derived DA/archive/challenge fallback status as display metadata,
//! but must not become wallet, ledger, paid-unlock, pruning, settlement, outside-DA, or outside-chain authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, routes/v1/header_policy.rs, v1 route/product hydration source.
//! RO:INVARIANTS — hydrated DA fallback status is display/evidence-only; pruning remains blocked until fallback is green elsewhere.
//! RO:METRICS — none; source/docs/header-policy boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks DA/archive/challenge/pruning authority smuggling into hydration or access decisions.
//! RO:TEST — cargo test -p omnigate --test quickchain_phase5_da_fallback_hydration_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const HYDRATION_SCHEMA: &str = "omnigate.quickchain-da-fallback-hydration.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HydratedDaFallbackStatus {
    schema: String,
    produced_at_ms: u64,
    hydration_source: String,
    chain_id: String,
    epoch_id: String,
    da_bundle_id: String,
    archive_cid: String,
    restore_artifact_cid: String,
    challenge_id: String,
    display_label: String,
    retention_window_epochs: u64,
    backend_derived: bool,
    read_only: bool,
    display_only: bool,
    evidence_only: bool,
    archive_restore_possible: bool,
    missing_data_challenge_possible: bool,
    pruning_blocked: bool,
    wallet_side_effect: bool,
    ledger_side_effect: bool,
    paid_unlock_authority: bool,
    balance_truth: bool,
    receipt_truth: bool,
    finality_truth: bool,
    settlement_truth: bool,
    pruning_authority: bool,
    outside_da_truth: bool,
    outside_chain_truth: bool,
}

impl HydratedDaFallbackStatus {
    fn sample() -> Self {
        Self {
            schema: HYDRATION_SCHEMA.to_string(),
            produced_at_ms: 1_777_800_002_000,
            hydration_source: "backend-derived:omnigate".to_string(),
            chain_id: "roc-dev".to_string(),
            epoch_id: "epoch:phase5:r2:omnigate".to_string(),
            da_bundle_id: "da-bundle:phase5:r2:omnigate".to_string(),
            archive_cid: format!("b3:{}", "a".repeat(64)),
            restore_artifact_cid: format!("b3:{}", "b".repeat(64)),
            challenge_id: "missing-data-challenge:phase5:r2:omnigate".to_string(),
            display_label: "da_fallback_display_only".to_string(),
            retention_window_epochs: 32,
            backend_derived: true,
            read_only: true,
            display_only: true,
            evidence_only: true,
            archive_restore_possible: true,
            missing_data_challenge_possible: true,
            pruning_blocked: true,
            wallet_side_effect: false,
            ledger_side_effect: false,
            paid_unlock_authority: false,
            balance_truth: false,
            receipt_truth: false,
            finality_truth: false,
            settlement_truth: false,
            pruning_authority: false,
            outside_da_truth: false,
            outside_chain_truth: false,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != HYDRATION_SCHEMA {
            return Err("invalid omnigate DA fallback hydration schema".to_string());
        }

        if self.produced_at_ms == 0 {
            return Err("produced_at_ms must be nonzero".to_string());
        }

        for (name, value) in [
            ("hydration_source", self.hydration_source.as_str()),
            ("chain_id", self.chain_id.as_str()),
            ("epoch_id", self.epoch_id.as_str()),
            ("da_bundle_id", self.da_bundle_id.as_str()),
            ("challenge_id", self.challenge_id.as_str()),
            ("display_label", self.display_label.as_str()),
        ] {
            validate_visible_token(name, value)?;
        }

        validate_b3("archive_cid", &self.archive_cid)?;
        validate_b3("restore_artifact_cid", &self.restore_artifact_cid)?;

        if self.display_label != "da_fallback_display_only" {
            return Err(
                "omnigate DA fallback hydration must use an honest display-only label".to_string(),
            );
        }

        if self.retention_window_epochs == 0 {
            return Err("retention_window_epochs must be nonzero".to_string());
        }

        if !self.backend_derived {
            return Err("omnigate DA fallback hydration must be backend-derived".to_string());
        }

        if !self.read_only {
            return Err("omnigate DA fallback hydration must be read-only".to_string());
        }

        if !self.display_only {
            return Err("omnigate DA fallback hydration must be display-only".to_string());
        }

        if !self.evidence_only {
            return Err("omnigate DA fallback hydration must be evidence-only".to_string());
        }

        if !self.archive_restore_possible {
            return Err(
                "omnigate DA fallback hydration must preserve archive restore visibility"
                    .to_string(),
            );
        }

        if !self.missing_data_challenge_possible {
            return Err(
                "omnigate DA fallback hydration must preserve missing-data challenge visibility"
                    .to_string(),
            );
        }

        if !self.pruning_blocked {
            return Err("omnigate DA fallback hydration must keep pruning blocked".to_string());
        }

        if self.wallet_side_effect {
            return Err("omnigate DA fallback hydration must not mutate wallet state".to_string());
        }

        if self.ledger_side_effect {
            return Err("omnigate DA fallback hydration must not mutate ledger state".to_string());
        }

        if self.paid_unlock_authority {
            return Err("omnigate DA fallback hydration must not unlock paid content".to_string());
        }

        if self.balance_truth {
            return Err("omnigate DA fallback hydration must not become balance truth".to_string());
        }

        if self.receipt_truth {
            return Err("omnigate DA fallback hydration must not become receipt truth".to_string());
        }

        if self.finality_truth {
            return Err(
                "omnigate DA fallback hydration must not become finality truth".to_string(),
            );
        }

        if self.settlement_truth {
            return Err(
                "omnigate DA fallback hydration must not become settlement truth".to_string(),
            );
        }

        if self.pruning_authority {
            return Err(
                "omnigate DA fallback hydration must not become pruning authority".to_string(),
            );
        }

        if self.outside_da_truth {
            return Err(
                "omnigate DA fallback hydration must not make outside DA ROC truth".to_string(),
            );
        }

        if self.outside_chain_truth {
            return Err(
                "omnigate DA fallback hydration must not make outside-chain ROC truth".to_string(),
            );
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
                    "omnigate DA fallback hydration must not expose forbidden authority key `{forbidden}`"
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
fn docs_name_phase5_round2_omnigate_da_fallback_hydration_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 5 round 2 da/archive/challenge fallback boundary",
        "omnigate may hydrate backend-derived da/archive/challenge fallback evidence/status only as display metadata",
        "omnigate does not mutate wallet or ledger from da/archive/challenge evidence",
        "da fallback evidence cannot unlock paid content through omnigate",
        "da fallback evidence cannot become balance truth, receipt truth, finality truth, settlement truth, pruning authority, outside-da truth, or outside-chain truth",
        "no pruning can be triggered through omnigate unless archive/da/challenge fallback is proven green elsewhere",
        "quickchain_phase5_da_fallback_hydration_boundary",
    ] {
        assert_contains(&doc, required, "omnigate quickchain-preflight.md");
    }
}

#[test]
fn omnigate_da_fallback_hydration_is_display_only_metadata_and_pruning_blocker() {
    let report = HydratedDaFallbackStatus::sample();
    report
        .validate()
        .expect("DA fallback hydration should validate");

    assert!(report.backend_derived);
    assert!(report.read_only);
    assert!(report.display_only);
    assert!(report.evidence_only);
    assert!(report.archive_restore_possible);
    assert!(report.missing_data_challenge_possible);
    assert!(report.pruning_blocked);

    assert!(!report.wallet_side_effect);
    assert!(!report.ledger_side_effect);
    assert!(!report.paid_unlock_authority);
    assert!(!report.balance_truth);
    assert!(!report.receipt_truth);
    assert!(!report.finality_truth);
    assert!(!report.settlement_truth);
    assert!(!report.pruning_authority);
    assert!(!report.outside_da_truth);
    assert!(!report.outside_chain_truth);

    let value = serde_json::to_value(&report).expect("report should serialize");

    for forbidden in [
        "wallet_receipt",
        "ledger_receipt",
        "balance_minor",
        "receipt_hash",
        "paid_unlock",
        "settlement_status",
        "finalized",
        "prune_now",
        "pruning_receipt",
        "bridge_settlement",
        "external_da_truth",
        "outside_chain_roc_truth",
    ] {
        assert_no_key(&value, forbidden);
    }
}

#[test]
fn omnigate_da_fallback_hydration_rejects_authority_flags_unknown_fields_and_bad_b3() {
    for field in [
        "wallet_side_effect",
        "ledger_side_effect",
        "paid_unlock_authority",
        "balance_truth",
        "receipt_truth",
        "finality_truth",
        "settlement_truth",
        "pruning_authority",
        "outside_da_truth",
        "outside_chain_truth",
    ] {
        let mut value = serde_json::to_value(HydratedDaFallbackStatus::sample())
            .expect("sample hydration should serialize");
        value
            .as_object_mut()
            .expect("sample hydration should be an object")
            .insert(field.to_string(), json!(true));

        let parsed: HydratedDaFallbackStatus =
            serde_json::from_value(value).expect("known field should deserialize");
        assert!(
            parsed.validate().is_err(),
            "authority flag `{field}` must be rejected when true"
        );
    }

    for poison in [
        "wallet_mutation",
        "ledger_mutation",
        "paid_unlock",
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
        let mut value = serde_json::to_value(HydratedDaFallbackStatus::sample())
            .expect("sample hydration should serialize");
        value
            .as_object_mut()
            .expect("sample hydration should be an object")
            .insert(poison.to_string(), json!(true));

        assert!(
            serde_json::from_value::<HydratedDaFallbackStatus>(value).is_err(),
            "unknown authority field `{poison}` must be rejected"
        );
    }

    for field in ["archive_cid", "restore_artifact_cid"] {
        let mut report = HydratedDaFallbackStatus::sample();
        match field {
            "archive_cid" => report.archive_cid = "b3:ABC".to_string(),
            "restore_artifact_cid" => report.restore_artifact_cid = "not-b3".to_string(),
            _ => unreachable!("test field set is fixed"),
        }

        assert!(
            report.validate().is_err(),
            "bad canonical b3 value in `{field}` must be rejected"
        );
    }
}

#[test]
fn omnigate_header_policy_names_phase5_da_fallback_authority_families() {
    let source = normalized(&read_rel("src/routes/v1/header_policy.rs"));

    for required in [
        "\"x-ron-da\"",
        "\"x-ron-da-bundle\"",
        "\"x-ron-data-availability\"",
        "\"x-ron-data-availability-root\"",
        "\"x-ron-archive\"",
        "\"x-ron-archive-proof\"",
        "\"x-ron-carrier-proof\"",
        "\"x-ron-missing-data-challenge\"",
        "\"x-ron-retention-window\"",
        "\"x-ron-pruning\"",
        "\"x-ron-prune\"",
        "raw.starts_with(\"x-ron-da-\")",
        "raw.starts_with(\"x-ron-data-availability-\")",
        "raw.starts_with(\"x-ron-archive-\")",
        "raw.starts_with(\"x-ron-pruning-\")",
        "raw.starts_with(\"x-ron-prune-\")",
        "raw.starts_with(\"x-quickchain-\")",
        "raw.starts_with(\"x-qc-\")",
        "raw.starts_with(\"x-ron-quickchain-\")",
    ] {
        assert_contains(&source, required, "omnigate header policy");
    }
}

#[test]
fn omnigate_route_surface_does_not_expose_phase5_da_or_pruning_runtime_routes() {
    let mut route_files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut route_files);

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

    for path in route_files {
        let source = normalized(&strip_line_comments(
            &fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("read route source {}: {err}", path.display())),
        ));

        for forbidden in forbidden_route_fragments {
            assert_not_contains(
                &source,
                forbidden,
                &format!("omnigate route source {}", path.display()),
            );
        }
    }
}

#[test]
fn omnigate_source_does_not_construct_phase5_da_or_pruning_runtime_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/hydration"), &mut files);
    collect_rust_files(&crate_root().join("src/middleware"), &mut files);

    let forbidden_compact_markers = [
        "da_wallet_mutation",
        "da_ledger_mutation",
        "da_balance_truth",
        "da_receipt_truth",
        "da_finality_truth",
        "da_settlement_truth",
        "da_paid_unlock_authority:true",
        "\"da_paid_unlock_authority\":true",
        "paid_unlock_from_da",
        "paid_unlock_from_archive",
        "unlock_from_da",
        "unlock_from_archive",
        "cache_unlock_from_da",
        "cache_unlock_from_archive",
        "wallet_mutation_from_da",
        "ledger_mutation_from_da",
        "balance_from_da",
        "receipt_from_da",
        "finality_from_da",
        "settlement_from_da",
        "outside_da_roc_truth",
        "outside_chain_roc_truth",
        "omnigate_da_truth:true",
        "omnigate_pruning_authority:true",
        "hydrate_as_da_truth",
        "hydrate_as_archive_unlock",
        "trigger_pruning_from_omnigate",
        "prune_from_omnigate",
    ];

    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        let compact = compact_without_comments(&source);

        for forbidden in forbidden_compact_markers {
            assert_not_contains(
                &compact,
                forbidden,
                &format!("omnigate source {}", path.display()),
            );
        }
    }
}

#[test]
fn existing_paid_and_access_routes_do_not_unlock_from_da_fallback_material() {
    for rel in [
        "src/routes/v1/content_view.rs",
        "src/routes/v1/site_visit.rs",
        "src/routes/v1/chat.rs",
        "src/routes/v1/streams.rs",
        "src/routes/v1/paid.rs",
        "src/routes/v1/wallet.rs",
    ] {
        let path = crate_root().join(rel);
        if !path.exists() {
            continue;
        }

        let source = normalized(&strip_line_comments(&read_rel(rel)));

        for forbidden in [
            "unlock_from_da",
            "unlock_from_archive",
            "paid_by_da",
            "paid_by_archive",
            "receipt_from_da",
            "balance_from_da",
            "finality_from_da",
            "settlement_from_da",
            "cache_unlock_from_da",
            "policy_unlock_from_da",
            "da_paid_unlock_authority",
            "archive_paid_unlock_authority",
        ] {
            assert_not_contains(&source, forbidden, rel);
        }
    }
}

#[test]
fn preflight_runner_dynamically_discovers_phase5_da_fallback_hydration_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "omnigate dev-quickchain-preflight.sh");
    }
}
