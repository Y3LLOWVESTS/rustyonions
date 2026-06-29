#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 5 Round 3 selected external-posture boundary tests for svc-gateway.
//! RO:WHY — Gateway may expose selected external posture status/evidence as metadata only,
//! but must not become wallet, ledger, paid-unlock, reward, finality, settlement, bridge, ROX/Solana,
//! staking, liquidity, market, exchange, or outside-program authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, headers::proxy, gateway routes/admission/layers.
//! RO:INVARIANTS — external posture is anchor-only/evidence-only/status-only; wallet/ledger remain truth.
//! RO:METRICS — none; source/docs/header boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks selected external posture authority smuggling through public gateway boundaries.
//! RO:TEST — cargo test -p svc-gateway --test quickchain_phase5_external_posture_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use http::HeaderName;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use svc_gateway::headers::proxy;

const STATUS_SCHEMA: &str = "svc-gateway.quickchain-external-posture-status.v1";

const PHASE5_EXTERNAL_POSTURE_AUTHORITY_HEADERS: &[&str] = &[
    "x-ron-external-posture",
    "x-ron-external-posture-proof",
    "x-ron-external-posture-finality",
    "x-ron-external-posture-settlement",
    "x-ron-outside-program",
    "x-ron-outside-program-authority",
    "x-ron-public-chain",
    "x-ron-public-market",
    "x-ron-market",
    "x-ron-exchange",
    "x-ron-rox",
    "x-ron-rox-runtime",
    "x-ron-solana",
    "x-ron-solana-runtime",
    "x-ron-external-settlement",
    "x-ron-bridge",
    "x-ron-bridge-authority",
    "x-ron-staking",
    "x-ron-liquidity",
    "x-quickchain-external-posture",
    "x-qc-external-posture",
    "x-ron-quickchain-external-posture",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GatewayExternalPostureStatus {
    schema: String,
    produced_at_ms: u64,
    source: String,
    chain_id: String,
    epoch_id: String,
    posture_id: String,
    posture_label: String,
    selected_posture: String,
    anchor_commitment: String,
    evidence_cid: String,
    report_cid: String,
    backend_derived: bool,
    read_only: bool,
    display_only: bool,
    evidence_only: bool,
    anchor_only: bool,
    status_metadata_only: bool,
    external_settlement_enabled: bool,
    public_bridge_enabled: bool,
    rox_runtime_enabled: bool,
    solana_runtime_enabled: bool,
    staking_enabled: bool,
    liquidity_enabled: bool,
    exchange_facing_enabled: bool,
    public_market_authority: bool,
    outside_program_authority: bool,
    wallet_side_effect: bool,
    ledger_side_effect: bool,
    paid_unlock_authority: bool,
    balance_truth: bool,
    receipt_truth: bool,
    reward_truth: bool,
    finality_truth: bool,
    settlement_truth: bool,
    pruning_authority: bool,
    root_authority: bool,
}

impl GatewayExternalPostureStatus {
    fn sample() -> Self {
        Self {
            schema: STATUS_SCHEMA.to_string(),
            produced_at_ms: 1_777_700_003_000,
            source: "backend-derived:svc-gateway".to_string(),
            chain_id: "roc-dev".to_string(),
            epoch_id: "epoch:phase5:r3:gateway".to_string(),
            posture_id: "external-posture:phase5:r3:gateway".to_string(),
            posture_label: "external_posture_anchor_only_evidence".to_string(),
            selected_posture: "anchor-only".to_string(),
            anchor_commitment: format!("b3:{}", "e".repeat(64)),
            evidence_cid: format!("b3:{}", "f".repeat(64)),
            report_cid: format!("b3:{}", "1".repeat(64)),
            backend_derived: true,
            read_only: true,
            display_only: true,
            evidence_only: true,
            anchor_only: true,
            status_metadata_only: true,
            external_settlement_enabled: false,
            public_bridge_enabled: false,
            rox_runtime_enabled: false,
            solana_runtime_enabled: false,
            staking_enabled: false,
            liquidity_enabled: false,
            exchange_facing_enabled: false,
            public_market_authority: false,
            outside_program_authority: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            paid_unlock_authority: false,
            balance_truth: false,
            receipt_truth: false,
            reward_truth: false,
            finality_truth: false,
            settlement_truth: false,
            pruning_authority: false,
            root_authority: false,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != STATUS_SCHEMA {
            return Err("invalid gateway external posture schema".to_string());
        }

        if self.produced_at_ms == 0 {
            return Err("produced_at_ms must be nonzero".to_string());
        }

        for (name, value) in [
            ("source", self.source.as_str()),
            ("chain_id", self.chain_id.as_str()),
            ("epoch_id", self.epoch_id.as_str()),
            ("posture_id", self.posture_id.as_str()),
            ("posture_label", self.posture_label.as_str()),
            ("selected_posture", self.selected_posture.as_str()),
        ] {
            validate_visible_token(name, value)?;
        }

        validate_b3("anchor_commitment", &self.anchor_commitment)?;
        validate_b3("evidence_cid", &self.evidence_cid)?;
        validate_b3("report_cid", &self.report_cid)?;

        if self.posture_label != "external_posture_anchor_only_evidence" {
            return Err("gateway external posture label must remain evidence-only".to_string());
        }

        if self.selected_posture != "anchor-only" {
            return Err("gateway selected posture must remain anchor-only".to_string());
        }

        for (name, value) in [
            ("backend_derived", self.backend_derived),
            ("read_only", self.read_only),
            ("display_only", self.display_only),
            ("evidence_only", self.evidence_only),
            ("anchor_only", self.anchor_only),
            ("status_metadata_only", self.status_metadata_only),
        ] {
            if !value {
                return Err(format!(
                    "{name} must be true for gateway external posture metadata"
                ));
            }
        }

        for (name, value) in [
            (
                "external_settlement_enabled",
                self.external_settlement_enabled,
            ),
            ("public_bridge_enabled", self.public_bridge_enabled),
            ("rox_runtime_enabled", self.rox_runtime_enabled),
            ("solana_runtime_enabled", self.solana_runtime_enabled),
            ("staking_enabled", self.staking_enabled),
            ("liquidity_enabled", self.liquidity_enabled),
            ("exchange_facing_enabled", self.exchange_facing_enabled),
            ("public_market_authority", self.public_market_authority),
            ("outside_program_authority", self.outside_program_authority),
            ("wallet_side_effect", self.wallet_side_effect),
            ("ledger_side_effect", self.ledger_side_effect),
            ("paid_unlock_authority", self.paid_unlock_authority),
            ("balance_truth", self.balance_truth),
            ("receipt_truth", self.receipt_truth),
            ("reward_truth", self.reward_truth),
            ("finality_truth", self.finality_truth),
            ("settlement_truth", self.settlement_truth),
            ("pruning_authority", self.pruning_authority),
            ("root_authority", self.root_authority),
        ] {
            if value {
                return Err(format!(
                    "{name} must remain false for gateway external posture metadata"
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
                    "gateway external posture must not expose forbidden authority key `{forbidden}`"
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
fn docs_name_phase5_round3_gateway_external_posture_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 5 round 3 selected external posture boundary",
        "svc-gateway may expose selected external posture evidence/status only as read-only metadata",
        "svc-gateway does not mutate wallet or ledger from selected external posture evidence",
        "selected external posture evidence cannot unlock paid content through svc-gateway",
        "selected external posture evidence cannot become balance truth, receipt truth, reward truth, finality truth, or settlement truth",
        "outside programs cannot become roc truth through gateway",
        "quickchain_phase5_external_posture_boundary",
    ] {
        assert_contains(&doc, required, "svc-gateway quickchain-preflight.md");
    }
}

#[test]
fn gateway_external_posture_status_is_anchor_only_evidence_metadata() {
    let report = GatewayExternalPostureStatus::sample();
    report
        .validate()
        .expect("external posture status should validate");

    assert!(report.backend_derived);
    assert!(report.read_only);
    assert!(report.display_only);
    assert!(report.evidence_only);
    assert!(report.anchor_only);
    assert!(report.status_metadata_only);

    assert!(!report.external_settlement_enabled);
    assert!(!report.public_bridge_enabled);
    assert!(!report.rox_runtime_enabled);
    assert!(!report.solana_runtime_enabled);
    assert!(!report.staking_enabled);
    assert!(!report.liquidity_enabled);
    assert!(!report.exchange_facing_enabled);
    assert!(!report.public_market_authority);
    assert!(!report.outside_program_authority);
    assert!(!report.wallet_side_effect);
    assert!(!report.ledger_side_effect);
    assert!(!report.paid_unlock_authority);
    assert!(!report.balance_truth);
    assert!(!report.receipt_truth);
    assert!(!report.reward_truth);
    assert!(!report.finality_truth);
    assert!(!report.settlement_truth);
    assert!(!report.pruning_authority);
    assert!(!report.root_authority);

    let value = serde_json::to_value(&report).expect("report should serialize");

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
        "solana_runtime",
        "rox_runtime",
    ] {
        assert_no_key(&value, forbidden);
    }
}

#[test]
fn gateway_external_posture_rejects_authority_flags_unknown_fields_and_bad_b3() {
    for field in [
        "external_settlement_enabled",
        "public_bridge_enabled",
        "rox_runtime_enabled",
        "solana_runtime_enabled",
        "staking_enabled",
        "liquidity_enabled",
        "exchange_facing_enabled",
        "public_market_authority",
        "outside_program_authority",
        "wallet_side_effect",
        "ledger_side_effect",
        "paid_unlock_authority",
        "balance_truth",
        "receipt_truth",
        "reward_truth",
        "finality_truth",
        "settlement_truth",
        "pruning_authority",
        "root_authority",
    ] {
        let mut value = serde_json::to_value(GatewayExternalPostureStatus::sample())
            .expect("sample report should serialize");
        value
            .as_object_mut()
            .expect("sample report should be an object")
            .insert(field.to_string(), json!(true));

        let parsed: GatewayExternalPostureStatus =
            serde_json::from_value(value).expect("known field should deserialize");
        assert!(
            parsed.validate().is_err(),
            "authority flag `{field}` must be rejected when true"
        );
    }

    for poison in [
        "external_posture_unlock",
        "unlock_from_external_posture",
        "external_posture_receipt",
        "external_posture_balance",
        "external_posture_settlement",
        "external_posture_finality",
        "external_posture_reward_truth",
        "wallet_mutation",
        "ledger_mutation",
        "paid_unlock",
        "balance_minor",
        "wallet_receipt",
        "ledger_receipt",
        "reward_payout",
        "settlement_status",
        "finality_proof",
        "outside_chain_truth",
        "outside_program_authority_claim",
        "bridge_authority",
        "exchange_facing_authority",
        "solana_runtime",
        "rox_runtime",
    ] {
        let mut value = serde_json::to_value(GatewayExternalPostureStatus::sample())
            .expect("sample report should serialize");
        value
            .as_object_mut()
            .expect("sample report should be an object")
            .insert(poison.to_string(), json!(true));

        assert!(
            serde_json::from_value::<GatewayExternalPostureStatus>(value).is_err(),
            "unknown authority field `{poison}` must be rejected"
        );
    }

    for field in ["anchor_commitment", "evidence_cid", "report_cid"] {
        let mut report = GatewayExternalPostureStatus::sample();
        match field {
            "anchor_commitment" => report.anchor_commitment = "b3:ABC".to_string(),
            "evidence_cid" => report.evidence_cid = "not-b3".to_string(),
            "report_cid" => report.report_cid = format!("b3:{}", "g".repeat(64)),
            _ => unreachable!("test field set is fixed"),
        }

        assert!(
            report.validate().is_err(),
            "bad canonical b3 value in `{field}` must be rejected"
        );
    }
}

#[test]
fn gateway_filters_phase5_external_posture_authority_headers() {
    for raw in PHASE5_EXTERNAL_POSTURE_AUTHORITY_HEADERS {
        let name = header(raw);

        assert!(
            !proxy::should_forward_passthrough_header(&name),
            "gateway passthrough proxy must reject Phase 5 external posture authority header {raw}"
        );
        assert!(
            !proxy::should_forward_product_header(&name),
            "gateway product proxy must reject Phase 5 external posture authority header {raw}"
        );
        assert!(
            !proxy::should_copy_response_header(&name),
            "gateway response proxy must reject Phase 5 external posture authority header {raw}"
        );
    }
}

#[test]
fn gateway_route_surface_does_not_expose_phase5_external_posture_runtime_routes() {
    let mut route_files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut route_files);

    let forbidden_route_fragments = [
        ".route(\"/external-posture",
        ".route(\"/external-settlement",
        ".route(\"/outside-program",
        ".route(\"/public-chain",
        ".route(\"/bridge",
        ".route(\"/rox",
        ".route(\"/solana",
        ".route(\"/staking",
        ".route(\"/liquidity",
        ".route(\"/market",
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
fn gateway_source_does_not_construct_phase5_external_posture_runtime_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/admission"), &mut files);
    collect_rust_files(&crate_root().join("src/layers"), &mut files);
    collect_rust_files(&crate_root().join("src/headers"), &mut files);

    let forbidden_compact_markers = [
        "external_posture_unlock",
        "unlock_from_external_posture",
        "external_posture_paid_unlock_authority:true",
        "\"external_posture_paid_unlock_authority\":true",
        "paid_unlock_from_external_posture",
        "wallet_mutation_from_external_posture",
        "ledger_mutation_from_external_posture",
        "balance_from_external_posture",
        "receipt_from_external_posture",
        "reward_from_external_posture",
        "finality_from_external_posture",
        "settlement_from_external_posture",
        "pruning_from_external_posture",
        "root_from_external_posture",
        "gateway_external_posture_truth:true",
        "gateway_outside_program_authority:true",
        "external_settlement_enabled:true",
        "\"external_settlement_enabled\":true",
        "public_bridge_enabled:true",
        "\"public_bridge_enabled\":true",
        "rox_runtime_enabled:true",
        "\"rox_runtime_enabled\":true",
        "solana_runtime_enabled:true",
        "\"solana_runtime_enabled\":true",
        "staking_enabled:true",
        "\"staking_enabled\":true",
        "liquidity_enabled:true",
        "\"liquidity_enabled\":true",
        "exchange_facing_enabled:true",
        "\"exchange_facing_enabled\":true",
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
fn existing_paid_and_product_routes_do_not_unlock_from_external_posture_material() {
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
            "unlock_from_external_posture",
            "paid_by_external_posture",
            "receipt_from_external_posture",
            "balance_from_external_posture",
            "reward_from_external_posture",
            "finality_from_external_posture",
            "settlement_from_external_posture",
            "cache_unlock_from_external_posture",
            "policy_unlock_from_external_posture",
            "external_posture_paid_unlock_authority",
        ] {
            assert_not_contains(&source, forbidden, rel);
        }
    }
}

#[test]
fn gateway_does_not_gain_external_settlement_runtime_dependencies() {
    let cargo = normalized(&read_rel("Cargo.toml"));

    for forbidden in [
        "anchor-lang",
        "spl-token",
        "solana-client",
        "solana-sdk",
        "ethers",
        "web3",
        "alloy",
        "roxmltree",
    ] {
        assert_not_contains(&cargo, forbidden, "svc-gateway Cargo.toml");
    }
}

#[test]
fn preflight_runner_dynamically_discovers_phase5_external_posture_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "svc-gateway dev-quickchain-preflight.sh");
    }
}
