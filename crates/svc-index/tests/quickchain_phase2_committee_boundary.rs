#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

use serde_json::{json, Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

use svc_index::types::{PutAssetManifestPointer, PutSiteManifestPointer};

const ASSET_MANIFEST_CID: &str =
    "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const SITE_MANIFEST_CID: &str =
    "b3:1111111111111111111111111111111111111111111111111111111111111111";

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

fn production_source_text() -> String {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);

    let mut out = String::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        out.push_str(&strip_line_comments(&source));
        out.push('\n');
    }

    out.to_ascii_lowercase()
}

fn assert_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        haystack.contains(needle),
        "{label} must contain required Phase 2 Round 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, forbidden: &str, label: &str) {
    assert!(
        !haystack.contains(forbidden),
        "{label} must not contain forbidden Phase 2 Round 2 authority marker: {forbidden}"
    );
}

#[test]
fn docs_name_phase2_round2_index_committee_readiness_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 2 round 2 committee readiness boundary",
        "svc-index may point to backend-derived verifier readiness artifacts only as references",
        "svc-index may point to backend-derived committee readiness artifacts only as references",
        "index artifact pointer is not verifier truth",
        "index artifact pointer is not committee truth",
        "index artifact pointer is not quorum truth",
        "index artifact pointer is not fork choice",
        "index artifact pointer is not finality",
        "index artifact pointer is not settlement",
        "index artifact pointer cannot unlock paid content",
        "b3 proves bytes, not committee truth",
        "names are pointers, not proof",
        "manifest lookup is not paid unlock",
        "provider lookup is not settlement finality",
        "owner wallet fields are references only",
        "owner passport fields are references only",
        "svc-index does not produce signed verification attestations",
        "svc-index does not decide committee membership",
        "svc-index does not decide quorum",
        "svc-index does not decide fork choice",
        "svc-index does not decide finality",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "quickchain_phase2_committee_boundary",
    ] {
        assert_contains(&doc, required, "svc-index quickchain-preflight.md");
    }
}

#[test]
fn manifest_pointer_dtos_reject_committee_quorum_and_finality_smuggling() {
    for extra_key in [
        "verifier_attestation",
        "verifier_signature",
        "committee_attestation",
        "committee_signature",
        "committee_member",
        "committee_vote",
        "quorum_reached",
        "quorum_certificate",
        "fork_choice",
        "fork_choice_winner",
        "finalized",
        "settlement_finality",
    ] {
        let mut asset = Map::new();
        asset.insert("asset_kind".to_owned(), json!("image"));
        asset.insert("manifest_cid".to_owned(), json!(ASSET_MANIFEST_CID));
        asset.insert(
            "owner_passport_subject".to_owned(),
            json!("passport:creator"),
        );
        asset.insert("owner_wallet_account".to_owned(), json!("acct_creator"));
        asset.insert("updated_at_ms".to_owned(), json!(1_u64));
        asset.insert(extra_key.to_owned(), json!("forbidden"));

        let asset_result = serde_json::from_value::<PutAssetManifestPointer>(Value::Object(asset));
        assert!(
            asset_result.is_err(),
            "asset manifest pointer must reject authority-shaped field: {extra_key}"
        );

        let mut site = Map::new();
        site.insert("manifest_cid".to_owned(), json!(SITE_MANIFEST_CID));
        site.insert(
            "owner_passport_subject".to_owned(),
            json!("passport:creator"),
        );
        site.insert("owner_wallet_account".to_owned(), json!("acct_creator"));
        site.insert("updated_at_ms".to_owned(), json!(1_u64));
        site.insert(extra_key.to_owned(), json!("forbidden"));

        let site_result = serde_json::from_value::<PutSiteManifestPointer>(Value::Object(site));
        assert!(
            site_result.is_err(),
            "site manifest pointer must reject authority-shaped field: {extra_key}"
        );
    }
}

#[test]
fn routes_do_not_expose_committee_quorum_validator_or_bridge_authority_surfaces() {
    let source = production_source_text();

    for forbidden in [
        "\"/quickchain",
        "\"/committee",
        "\"/committees",
        "\"/quorum",
        "\"/quorum-certificate",
        "\"/fork-choice",
        "\"/finality",
        "\"/finalized",
        "\"/validator",
        "\"/validators",
        "\"/attestation",
        "\"/attestations",
        "\"/settlement",
        "\"/external-settlement",
        "\"/anchor",
        "\"/bridge",
        "\"/staking",
        "\"/liquidity",
        "\"/rox",
        "\"/solana",
        "\"/paid/unlock",
    ] {
        assert_not_contains(
            &source,
            forbidden,
            "svc-index production route/source strings",
        );
    }
}

#[test]
fn production_source_does_not_construct_committee_quorum_or_finality_truth() {
    let source = production_source_text();

    for forbidden in [
        "svc_wallet::",
        "ron_ledger::",
        "ron_proto::quickchain",
        "quickchain::",
        "verifier_attestation:",
        "\"verifier_attestation\"",
        "verifier_signature:",
        "\"verifier_signature\"",
        "committee_attestation:",
        "\"committee_attestation\"",
        "committee_signature:",
        "\"committee_signature\"",
        "committee_member:",
        "\"committee_member\"",
        "committee_vote:",
        "\"committee_vote\"",
        "quorum_reached:",
        "\"quorum_reached\"",
        "quorum_certificate:",
        "\"quorum_certificate\"",
        "fork_choice:",
        "\"fork_choice\"",
        "fork_choice_winner:",
        "\"fork_choice_winner\"",
        "settlement_finality:",
        "\"settlement_finality\"",
        "index_pointer_proves_committee_truth",
        "index_pointer_proves_quorum_truth",
        "index_pointer_proves_finality",
        "pointer_proves_attestation",
        "cache_hit_proves_committee",
        "produce_attestation(",
        "sign_attestation(",
        "decide_committee(",
        "decide_quorum(",
        "decide_fork_choice(",
        "mark_finalized(",
        "set_settlement_status(",
        "commit_checkpoint(",
        "anchor_checkpoint(",
        "bridge_settlement(",
        "grant_paid_access(",
    ] {
        assert_not_contains(&source, forbidden, "svc-index production source");
    }
}

#[test]
fn manifest_does_not_add_committee_validator_or_external_settlement_dependencies() {
    let cargo = read_rel("Cargo.toml").to_ascii_lowercase();

    for forbidden in [
        "ron-ledger",
        "svc-wallet",
        "ron-accounting",
        "svc-rewarder",
        "quickchain-runtime",
        "quickchain-validator",
        "quickchain-consensus",
        "tendermint",
        "cometbft",
        "solana-client",
        "solana-sdk",
        "anchor-lang",
        "spl-token",
        "ethers",
        "web3",
    ] {
        assert_not_contains(&cargo, forbidden, "svc-index Cargo.toml");
    }
}

#[test]
fn dynamic_preflight_will_pick_up_the_phase2_committee_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "basename \"$test_file\" .rs",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(required, required, "literal self-check");
        assert_contains(&script, required, "svc-index dev-quickchain-preflight.sh");
    }
}
