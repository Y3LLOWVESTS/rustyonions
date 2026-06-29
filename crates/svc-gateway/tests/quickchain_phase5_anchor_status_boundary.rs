#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 5 Round 1 anchor-status boundary tests for svc-gateway.
//! RO:WHY — Gateway may expose backend-derived anchor dry-run status as read-only metadata,
//! but must not become wallet, ledger, paid-unlock, finality, settlement, bridge, or outside-chain authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, headers::proxy, gateway route/admission source.
//! RO:INVARIANTS — anchor status is dry-run/evidence-only/display metadata; svc-wallet/ron-ledger remain truth.
//! RO:METRICS — none; source/docs/header boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks Phase 5 anchor/status authority smuggling through public gateway routes and headers.
//! RO:TEST — cargo test -p svc-gateway --test quickchain_phase5_anchor_status_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use http::HeaderName;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use svc_gateway::headers::proxy;

const STATUS_SCHEMA: &str = "svc-gateway.quickchain-anchor-status.v1";

const PHASE5_ANCHOR_AUTHORITY_HEADERS: &[&str] = &[
    "x-ron-anchor",
    "x-ron-anchored",
    "x-ron-anchor-dry-run",
    "x-ron-anchor-evidence",
    "x-ron-anchor-proof",
    "x-ron-anchor-status",
    "x-ron-anchor-finality",
    "x-ron-anchor-settlement",
    "x-ron-anchor-paid-unlock",
    "x-quickchain-anchor",
    "x-qc-anchor",
    "x-ron-quickchain-anchor",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GatewayAnchorStatus {
    schema: String,
    produced_at_ms: u64,
    source: String,
    chain_id: String,
    epoch_id: String,
    anchor_id: String,
    checkpoint_commitment: String,
    evidence_cid: String,
    status_label: String,
    dry_run: bool,
    backend_derived: bool,
    read_only: bool,
    display_only: bool,
    evidence_only: bool,
    compact_commitment_only: bool,
    wallet_side_effect: bool,
    ledger_side_effect: bool,
    paid_unlock_authority: bool,
    balance_truth: bool,
    receipt_truth: bool,
    finality_truth: bool,
    settlement_truth: bool,
    outside_chain_truth: bool,
}

impl GatewayAnchorStatus {
    fn sample() -> Self {
        Self {
            schema: STATUS_SCHEMA.to_string(),
            produced_at_ms: 1_777_700_001_000,
            source: "backend-derived:svc-gateway".to_string(),
            chain_id: "roc-dev".to_string(),
            epoch_id: "epoch:phase5:r1:gateway".to_string(),
            anchor_id: "anchor-dry-run:phase5:r1:gateway".to_string(),
            checkpoint_commitment: format!("b3:{}", "a".repeat(64)),
            evidence_cid: format!("b3:{}", "b".repeat(64)),
            status_label: "anchor_dry_run".to_string(),
            dry_run: true,
            backend_derived: true,
            read_only: true,
            display_only: true,
            evidence_only: true,
            compact_commitment_only: true,
            wallet_side_effect: false,
            ledger_side_effect: false,
            paid_unlock_authority: false,
            balance_truth: false,
            receipt_truth: false,
            finality_truth: false,
            settlement_truth: false,
            outside_chain_truth: false,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != STATUS_SCHEMA {
            return Err("invalid gateway anchor status schema".to_string());
        }

        if self.produced_at_ms == 0 {
            return Err("produced_at_ms must be nonzero".to_string());
        }

        for (name, value) in [
            ("source", self.source.as_str()),
            ("chain_id", self.chain_id.as_str()),
            ("epoch_id", self.epoch_id.as_str()),
            ("anchor_id", self.anchor_id.as_str()),
            ("status_label", self.status_label.as_str()),
        ] {
            validate_visible_token(name, value)?;
        }

        validate_b3("checkpoint_commitment", &self.checkpoint_commitment)?;
        validate_b3("evidence_cid", &self.evidence_cid)?;

        if self.status_label != "anchor_dry_run" {
            return Err("gateway anchor status must be labeled anchor_dry_run".to_string());
        }

        if !self.dry_run {
            return Err("gateway anchor status must remain dry-run".to_string());
        }

        if !self.backend_derived {
            return Err("gateway anchor status must be backend-derived".to_string());
        }

        if !self.read_only {
            return Err("gateway anchor status must be read-only".to_string());
        }

        if !self.display_only {
            return Err("gateway anchor status must be display-only".to_string());
        }

        if !self.evidence_only {
            return Err("gateway anchor status must be evidence-only".to_string());
        }

        if !self.compact_commitment_only {
            return Err("gateway anchor status must be compact-commitment-only".to_string());
        }

        if self.wallet_side_effect {
            return Err("gateway anchor status must not mutate wallet state".to_string());
        }

        if self.ledger_side_effect {
            return Err("gateway anchor status must not mutate ledger state".to_string());
        }

        if self.paid_unlock_authority {
            return Err("gateway anchor status must not unlock paid content".to_string());
        }

        if self.balance_truth {
            return Err("gateway anchor status must not become balance truth".to_string());
        }

        if self.receipt_truth {
            return Err("gateway anchor status must not become receipt truth".to_string());
        }

        if self.finality_truth {
            return Err("gateway anchor status must not become finality truth".to_string());
        }

        if self.settlement_truth {
            return Err("gateway anchor status must not become settlement truth".to_string());
        }

        if self.outside_chain_truth {
            return Err("gateway anchor status must not make outside-chain ROC truth".to_string());
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
        "{label} must contain required Phase 5 Round 1 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 5 Round 1 authority marker: {needle}"
    );
}

fn assert_no_key(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert_ne!(
                    key, forbidden,
                    "gateway anchor status must not expose forbidden authority key `{forbidden}`"
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

fn header(raw: &str) -> HeaderName {
    raw.parse::<HeaderName>()
        .unwrap_or_else(|err| panic!("parse header {raw}: {err}"))
}

#[test]
fn docs_name_phase5_round1_gateway_anchor_status_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 5 round 1 anchor-only dry-run boundary",
        "svc-gateway may expose backend-derived anchor dry-run evidence/status only as read-only metadata",
        "svc-gateway does not mutate wallet or ledger from anchor evidence",
        "anchor evidence cannot unlock paid content through svc-gateway",
        "anchor evidence cannot become balance truth, receipt truth, finality truth, or settlement truth",
        "outside chains cannot become roc truth through gateway",
        "quickchain_phase5_anchor_status_boundary",
    ] {
        assert_contains(&doc, required, "svc-gateway quickchain-preflight.md");
    }
}

#[test]
fn gateway_anchor_status_is_read_only_metadata() {
    let report = GatewayAnchorStatus::sample();
    report.validate().expect("anchor status should validate");

    assert!(report.dry_run);
    assert!(report.backend_derived);
    assert!(report.read_only);
    assert!(report.display_only);
    assert!(report.evidence_only);
    assert!(report.compact_commitment_only);

    assert!(!report.wallet_side_effect);
    assert!(!report.ledger_side_effect);
    assert!(!report.paid_unlock_authority);
    assert!(!report.balance_truth);
    assert!(!report.receipt_truth);
    assert!(!report.finality_truth);
    assert!(!report.settlement_truth);
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
        "bridge_settlement",
        "solana_runtime",
        "rox_runtime",
    ] {
        assert_no_key(&value, forbidden);
    }
}

#[test]
fn gateway_anchor_status_rejects_authority_flags_unknown_fields_and_bad_b3() {
    for field in [
        "wallet_side_effect",
        "ledger_side_effect",
        "paid_unlock_authority",
        "balance_truth",
        "receipt_truth",
        "finality_truth",
        "settlement_truth",
        "outside_chain_truth",
    ] {
        let mut value = serde_json::to_value(GatewayAnchorStatus::sample())
            .expect("sample report should serialize");
        value
            .as_object_mut()
            .expect("sample report should be an object")
            .insert(field.to_string(), json!(true));

        let parsed: GatewayAnchorStatus =
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
        "external_chain_roc_truth",
        "bridge_settlement",
    ] {
        let mut value = serde_json::to_value(GatewayAnchorStatus::sample())
            .expect("sample report should serialize");
        value
            .as_object_mut()
            .expect("sample report should be an object")
            .insert(poison.to_string(), json!(true));

        assert!(
            serde_json::from_value::<GatewayAnchorStatus>(value).is_err(),
            "unknown authority field `{poison}` must be rejected"
        );
    }

    for field in ["checkpoint_commitment", "evidence_cid"] {
        let mut report = GatewayAnchorStatus::sample();
        match field {
            "checkpoint_commitment" => report.checkpoint_commitment = "b3:ABC".to_string(),
            "evidence_cid" => report.evidence_cid = "not-b3".to_string(),
            _ => unreachable!("test field set is fixed"),
        }

        assert!(
            report.validate().is_err(),
            "bad canonical b3 value in `{field}` must be rejected"
        );
    }
}

#[test]
fn gateway_filters_phase5_anchor_authority_headers() {
    for raw in PHASE5_ANCHOR_AUTHORITY_HEADERS {
        let name = header(raw);

        assert!(
            !proxy::should_forward_passthrough_header(&name),
            "gateway passthrough proxy must reject Phase 5 anchor authority header {raw}"
        );
        assert!(
            !proxy::should_forward_product_header(&name),
            "gateway product proxy must reject Phase 5 anchor authority header {raw}"
        );
        assert!(
            !proxy::should_copy_response_header(&name),
            "gateway response proxy must reject Phase 5 anchor authority header {raw}"
        );
    }
}

#[test]
fn gateway_route_surface_does_not_expose_phase5_anchor_runtime_routes() {
    let mut route_files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut route_files);

    let forbidden_route_fragments = [
        ".route(\"/anchor",
        ".route(\"/anchors",
        ".route(\"/anchor-status",
        ".route(\"/anchor-proof",
        ".route(\"/external-anchor",
        ".route(\"/external-settlement",
        ".route(\"/bridge",
        ".route(\"/solana",
        ".route(\"/rox",
        ".route(\"/public-chain",
        ".route(\"/exchange",
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
                &format!("svc-gateway route source {}", path.display()),
            );
        }
    }
}

#[test]
fn gateway_source_does_not_construct_phase5_anchor_runtime_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/admission"), &mut files);
    collect_rust_files(&crate_root().join("src/layers"), &mut files);
    collect_rust_files(&crate_root().join("src/headers"), &mut files);

    let forbidden_compact_markers = [
        "anchor_wallet_mutation",
        "anchor_ledger_mutation",
        "anchor_balance_truth",
        "anchor_receipt_truth",
        "anchor_finality_truth",
        "anchor_settlement_truth",
        "anchor_paid_unlock_authority:true",
        "\"anchor_paid_unlock_authority\":true",
        "paid_unlock_from_anchor",
        "unlock_from_anchor",
        "cache_unlock_from_anchor",
        "wallet_mutation_from_anchor",
        "ledger_mutation_from_anchor",
        "balance_from_anchor",
        "receipt_from_anchor",
        "finality_from_anchor",
        "settlement_from_anchor",
        "external_chain_roc_truth",
        "gateway_anchor_truth:true",
        "gateway_anchor_finality:true",
        "solana_runtime",
        "solana_settlement",
        "solana_anchor_authority",
        "rox_runtime",
        "rox_settlement",
    ];

    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        let compact = compact_without_comments(&source);

        for forbidden in forbidden_compact_markers {
            assert_not_contains(
                &compact,
                forbidden,
                &format!("svc-gateway source {}", path.display()),
            );
        }
    }
}

#[test]
fn existing_paid_and_product_routes_do_not_unlock_from_anchor_material() {
    for rel in [
        "src/routes/paid_storage.rs",
        "src/routes/product.rs",
        "src/routes/app.rs",
        "src/routes/objects.rs",
        "src/routes/objects_range.rs",
    ] {
        let path = crate_root().join(rel);
        if !path.exists() {
            continue;
        }

        let source = normalized(&strip_line_comments(&read_rel(rel)));

        for forbidden in [
            "unlock_from_anchor",
            "paid_by_anchor",
            "receipt_from_anchor",
            "balance_from_anchor",
            "finality_from_anchor",
            "settlement_from_anchor",
            "cache_unlock_from_anchor",
            "policy_unlock_from_anchor",
            "anchor_paid_unlock_authority",
        ] {
            assert_not_contains(&source, forbidden, rel);
        }
    }
}

#[test]
fn preflight_runner_dynamically_discovers_phase5_anchor_status_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "svc-gateway dev-quickchain-preflight.sh");
    }
}
