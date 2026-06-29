#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 5 Round 1 anchor-pointer boundary tests for svc-index.
//! RO:WHY — Index may point to anchor dry-run artifacts by b3/reference only,
//! but pointers must not become proof, settlement, finality, paid unlock, wallet, ledger, or outside-chain authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, index routes, pointer/store/cache source.
//! RO:INVARIANTS — index entries are lookup metadata only; svc-wallet/ron-ledger remain truth.
//! RO:METRICS — none; source/docs boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks Phase 5 anchor authority smuggling through names, pointers, manifests, cache, or routes.
//! RO:TEST — cargo test -p svc-index --test quickchain_phase5_anchor_pointer_boundary.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const POINTER_SCHEMA: &str = "svc-index.quickchain-anchor-pointer.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexAnchorPointer {
    schema: String,
    pointer_id: String,
    source: String,
    pointer_kind: String,
    artifact_cid: String,
    checkpoint_commitment: String,
    evidence_cid: String,
    dry_run: bool,
    lookup_only: bool,
    read_only: bool,
    display_only: bool,
    b3_reference_only: bool,
    proof_truth: bool,
    settlement_truth: bool,
    finality_truth: bool,
    payment_truth: bool,
    paid_unlock_authority: bool,
    wallet_side_effect: bool,
    ledger_side_effect: bool,
    balance_truth: bool,
    receipt_truth: bool,
    outside_chain_truth: bool,
}

impl IndexAnchorPointer {
    fn sample() -> Self {
        Self {
            schema: POINTER_SCHEMA.to_string(),
            pointer_id: "anchor-pointer:phase5:r1:svc-index".to_string(),
            source: "backend-derived:svc-index".to_string(),
            pointer_kind: "anchor_dry_run_artifact_pointer".to_string(),
            artifact_cid: format!("b3:{}", "e".repeat(64)),
            checkpoint_commitment: format!("b3:{}", "f".repeat(64)),
            evidence_cid: format!("b3:{}", "1".repeat(64)),
            dry_run: true,
            lookup_only: true,
            read_only: true,
            display_only: true,
            b3_reference_only: true,
            proof_truth: false,
            settlement_truth: false,
            finality_truth: false,
            payment_truth: false,
            paid_unlock_authority: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            balance_truth: false,
            receipt_truth: false,
            outside_chain_truth: false,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != POINTER_SCHEMA {
            return Err("invalid svc-index anchor pointer schema".to_string());
        }

        for (name, value) in [
            ("pointer_id", self.pointer_id.as_str()),
            ("source", self.source.as_str()),
            ("pointer_kind", self.pointer_kind.as_str()),
        ] {
            validate_visible_token(name, value)?;
        }

        validate_b3("artifact_cid", &self.artifact_cid)?;
        validate_b3("checkpoint_commitment", &self.checkpoint_commitment)?;
        validate_b3("evidence_cid", &self.evidence_cid)?;

        if self.pointer_kind != "anchor_dry_run_artifact_pointer" {
            return Err(
                "svc-index anchor pointer must use dry-run artifact pointer kind".to_string(),
            );
        }

        if !self.dry_run {
            return Err("svc-index anchor pointer must remain dry-run".to_string());
        }

        if !self.lookup_only {
            return Err("svc-index anchor pointer must remain lookup-only".to_string());
        }

        if !self.read_only {
            return Err("svc-index anchor pointer must remain read-only".to_string());
        }

        if !self.display_only {
            return Err("svc-index anchor pointer must remain display-only".to_string());
        }

        if !self.b3_reference_only {
            return Err("svc-index anchor pointer must remain b3-reference-only".to_string());
        }

        if self.proof_truth {
            return Err("svc-index anchor pointer must not become proof truth".to_string());
        }

        if self.settlement_truth {
            return Err("svc-index anchor pointer must not become settlement truth".to_string());
        }

        if self.finality_truth {
            return Err("svc-index anchor pointer must not become finality truth".to_string());
        }

        if self.payment_truth {
            return Err("svc-index anchor pointer must not become payment truth".to_string());
        }

        if self.paid_unlock_authority {
            return Err("svc-index anchor pointer must not unlock paid content".to_string());
        }

        if self.wallet_side_effect {
            return Err("svc-index anchor pointer must not mutate wallet state".to_string());
        }

        if self.ledger_side_effect {
            return Err("svc-index anchor pointer must not mutate ledger state".to_string());
        }

        if self.balance_truth {
            return Err("svc-index anchor pointer must not become balance truth".to_string());
        }

        if self.receipt_truth {
            return Err("svc-index anchor pointer must not become receipt truth".to_string());
        }

        if self.outside_chain_truth {
            return Err(
                "svc-index anchor pointer must not make outside-chain ROC truth".to_string(),
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
                    "svc-index anchor pointer must not expose forbidden authority key `{forbidden}`"
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
fn docs_name_phase5_round1_index_anchor_pointer_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 5 round 1 anchor pointer boundary",
        "svc-index may point to anchor dry-run artifacts by b3/reference only",
        "index anchor pointers are lookup metadata only",
        "index anchor pointers are not proof, settlement, finality, payment truth, or paid unlock authority",
        "anchor pointers cannot mutate wallet or ledger",
        "anchor pointers cannot become balance truth, receipt truth, finality truth, or settlement truth",
        "outside chains cannot become roc truth through svc-index",
        "quickchain_phase5_anchor_pointer_boundary",
    ] {
        assert_contains(&doc, required, "svc-index quickchain-preflight.md");
    }
}

#[test]
fn index_anchor_pointer_is_lookup_metadata_only() {
    let pointer = IndexAnchorPointer::sample();
    pointer.validate().expect("anchor pointer should validate");

    assert!(pointer.dry_run);
    assert!(pointer.lookup_only);
    assert!(pointer.read_only);
    assert!(pointer.display_only);
    assert!(pointer.b3_reference_only);

    assert!(!pointer.proof_truth);
    assert!(!pointer.settlement_truth);
    assert!(!pointer.finality_truth);
    assert!(!pointer.payment_truth);
    assert!(!pointer.paid_unlock_authority);
    assert!(!pointer.wallet_side_effect);
    assert!(!pointer.ledger_side_effect);
    assert!(!pointer.balance_truth);
    assert!(!pointer.receipt_truth);
    assert!(!pointer.outside_chain_truth);

    let value = serde_json::to_value(&pointer).expect("pointer should serialize");

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
fn index_anchor_pointer_rejects_authority_flags_unknown_fields_and_bad_b3() {
    for field in [
        "proof_truth",
        "settlement_truth",
        "finality_truth",
        "payment_truth",
        "paid_unlock_authority",
        "wallet_side_effect",
        "ledger_side_effect",
        "balance_truth",
        "receipt_truth",
        "outside_chain_truth",
    ] {
        let mut value = serde_json::to_value(IndexAnchorPointer::sample())
            .expect("sample pointer should serialize");
        value
            .as_object_mut()
            .expect("sample pointer should be an object")
            .insert(field.to_string(), json!(true));

        let parsed: IndexAnchorPointer =
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
        let mut value = serde_json::to_value(IndexAnchorPointer::sample())
            .expect("sample pointer should serialize");
        value
            .as_object_mut()
            .expect("sample pointer should be an object")
            .insert(poison.to_string(), json!(true));

        assert!(
            serde_json::from_value::<IndexAnchorPointer>(value).is_err(),
            "unknown authority field `{poison}` must be rejected"
        );
    }

    for field in ["artifact_cid", "checkpoint_commitment", "evidence_cid"] {
        let mut pointer = IndexAnchorPointer::sample();
        match field {
            "artifact_cid" => pointer.artifact_cid = "b3:ABC".to_string(),
            "checkpoint_commitment" => pointer.checkpoint_commitment = "not-b3".to_string(),
            "evidence_cid" => pointer.evidence_cid = format!("b3:{}", "A".repeat(64)),
            _ => unreachable!("test field set is fixed"),
        }

        assert!(
            pointer.validate().is_err(),
            "bad canonical b3 value in `{field}` must be rejected"
        );
    }
}

#[test]
fn index_route_surface_does_not_expose_phase5_anchor_runtime_routes() {
    let mut route_files = Vec::new();
    collect_rust_files(&crate_root().join("src/http/routes"), &mut route_files);

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
                &format!("svc-index route source {}", path.display()),
            );
        }
    }
}

#[test]
fn index_source_does_not_construct_phase5_anchor_runtime_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/http/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/pipeline"), &mut files);
    collect_rust_files(&crate_root().join("src/store"), &mut files);
    collect_rust_files(&crate_root().join("src/cache"), &mut files);
    collect_rust_files(&crate_root().join("src/dht"), &mut files);

    let forbidden_compact_markers = [
        "index_anchor_truth:true",
        "\"index_anchor_truth\":true",
        "anchor_proof_truth:true",
        "\"anchor_proof_truth\":true",
        "anchor_settlement_truth:true",
        "\"anchor_settlement_truth\":true",
        "anchor_finality_truth:true",
        "\"anchor_finality_truth\":true",
        "anchor_payment_truth:true",
        "\"anchor_payment_truth\":true",
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
        "external_chain_balance_truth",
        "solana_runtime",
        "solana_settlement",
        "solana_anchor_authority",
        "rox_runtime",
        "rox_settlement",
        "bridge_from_anchor",
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
fn existing_pointer_manifest_and_provider_paths_do_not_unlock_from_anchor_material() {
    for rel in [
        "src/http/routes/index_manifests.rs",
        "src/http/routes/resolve.rs",
        "src/http/routes/providers.rs",
        "src/pipeline/resolve.rs",
        "src/pipeline/providers.rs",
        "src/store/mod.rs",
        "src/store/sled_store.rs",
        "src/types.rs",
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
            "anchor_payment_truth",
            "anchor_settlement_truth",
        ] {
            assert_not_contains(&source, forbidden, rel);
        }
    }
}

#[test]
fn index_manifest_does_not_add_phase5_external_runtime_dependencies() {
    let manifest = normalized(&read_rel("Cargo.toml"));

    for forbidden in [
        "ron-ledger",
        "svc-wallet",
        "quickchain-runtime",
        "quickchain-validator",
        "quickchain-consensus",
        "solana-sdk",
        "solana-client",
        "spl-token",
        "anchor-lang",
        "ethers",
        "web3",
    ] {
        assert_not_contains(&manifest, forbidden, "svc-index Cargo.toml");
    }
}

#[test]
fn preflight_runner_dynamically_discovers_phase5_anchor_pointer_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "svc-index dev-quickchain-preflight.sh");
    }
}
