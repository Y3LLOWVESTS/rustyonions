#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Phase 2 Round 1 read-only verifier artifact boundary tests for svc-index.
//! RO:WHY — svc-index may point to replay/proof artifacts but must not become proof, verifier, finality, or settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, manifest pointer DTOs, route/source boundary.
//! RO:INVARIANTS — index truth is lookup/pointer truth only; artifact pointers are references, not proof authority.
//! RO:METRICS — none; source/docs/DTO boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks replay/proof/verifier/quorum/committee/finality authority creep through index records.
//! RO:TEST — cargo test -p svc-index --test quickchain_phase2_replay_boundary.

use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

use svc_index::types::{
    normalize_b3_cid, AssetManifestPointer, PutAssetManifestPointer, PutSiteManifestPointer,
    SiteManifestPointer,
};

const ASSET_CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const MANIFEST_CID: &str = "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const REPLAY_ARTIFACT_CID: &str =
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

fn read_all_src_without_comments() -> String {
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
        "{label} must contain required Phase 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 2 authority marker: {needle}"
    );
}

fn assert_unknown_field_rejects<T>(mut value: Value, field: &str)
where
    T: serde::de::DeserializeOwned,
{
    value
        .as_object_mut()
        .expect("test fixture must be a JSON object")
        .insert(field.to_owned(), json!("forbidden-phase2-authority-claim"));

    let rendered = match serde_json::from_value::<T>(value) {
        Ok(_) => panic!("authority-shaped field {field:?} must reject"),
        Err(err) => err.to_string(),
    };

    assert!(
        rendered.contains("unknown field"),
        "expected unknown-field rejection for {field:?}, got: {rendered}"
    );
}

fn assert_json_has_no_phase2_authority_fields<T: Serialize>(value: &T, label: &str) {
    let rendered = serde_json::to_string(value)
        .expect("serialize pointer")
        .to_ascii_lowercase();

    for forbidden in [
        "replay_result",
        "replay_root",
        "verifier_result",
        "verifier_attestation",
        "committee_attestation",
        "committee_signature",
        "quorum",
        "quorum_reached",
        "fork_choice",
        "fork_choice_winner",
        "finalized",
        "finality",
        "settlement_status",
        "settlement_finality",
        "checkpoint_hash",
        "validator_signature",
        "bridge_proof",
        "paid_unlock",
        "unlock_granted",
    ] {
        assert_not_contains(&rendered, forbidden, label);
    }
}

#[test]
fn docs_name_phase2_round1_index_pointer_only_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 2 round 1 verifier artifact / read-only replication",
        "svc-index may lookup artifact pointers only",
        "svc-index may point to read-only replay/proof artifacts",
        "artifact pointers are references only",
        "index artifact pointer is not proof authority",
        "index artifact pointer is not verifier truth",
        "index artifact pointer is not quorum truth",
        "index artifact pointer is not committee truth",
        "index artifact pointer is not fork choice",
        "index artifact pointer is not finality",
        "index artifact pointer cannot unlock paid content",
        "b3 proves bytes, not verifier truth",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "quickchain_phase2_replay_boundary",
    ] {
        assert_contains(&doc, required, "svc-index quickchain-preflight.md");
    }
}

#[test]
fn b3_artifact_cids_remain_byte_references_not_proof_authority() {
    assert_eq!(
        normalize_b3_cid(REPLAY_ARTIFACT_CID).expect("canonical replay artifact cid"),
        REPLAY_ARTIFACT_CID
    );

    let pointer = AssetManifestPointer {
        version: 1,
        asset_cid: ASSET_CID.to_owned(),
        asset_kind: "image".to_owned(),
        manifest_cid: REPLAY_ARTIFACT_CID.to_owned(),
        owner_passport_subject: Some("passport:main:creator".to_owned()),
        owner_wallet_account: Some("acct_creator".to_owned()),
        updated_at_ms: 1_776_000_000_000,
    };

    assert_eq!(pointer.manifest_cid, REPLAY_ARTIFACT_CID);
    assert_json_has_no_phase2_authority_fields(&pointer, "asset manifest pointer");
}

#[test]
fn pointer_dtos_reject_replay_verifier_and_committee_authority_smuggling_fields() {
    let forbidden_fields = [
        "replay_result",
        "replay_root",
        "replay_verified",
        "replay_verdict",
        "verifier_result",
        "verifier_attestation",
        "verifier_signature",
        "committee_attestation",
        "committee_signature",
        "committee_vote",
        "quorum",
        "quorum_reached",
        "fork_choice",
        "fork_choice_winner",
        "finality",
        "finalized",
        "settlement_status",
        "settlement_finality",
        "checkpoint_hash",
        "validator_signature",
        "bridge_proof",
        "paid_unlock",
        "unlock_granted",
    ];

    for field in forbidden_fields {
        assert_unknown_field_rejects::<PutAssetManifestPointer>(
            json!({
                "asset_kind": "image",
                "manifest_cid": MANIFEST_CID,
                "owner_passport_subject": "passport:creator",
                "owner_wallet_account": "acct_creator",
                "updated_at_ms": 1
            }),
            field,
        );

        assert_unknown_field_rejects::<PutSiteManifestPointer>(
            json!({
                "manifest_cid": MANIFEST_CID,
                "owner_passport_subject": "passport:creator",
                "owner_wallet_account": "acct_creator",
                "updated_at_ms": 1
            }),
            field,
        );

        assert_unknown_field_rejects::<AssetManifestPointer>(
            json!({
                "version": 1,
                "asset_cid": ASSET_CID,
                "asset_kind": "image",
                "manifest_cid": MANIFEST_CID,
                "owner_passport_subject": "passport:creator",
                "owner_wallet_account": "acct_creator",
                "updated_at_ms": 1
            }),
            field,
        );

        assert_unknown_field_rejects::<SiteManifestPointer>(
            json!({
                "version": 1,
                "name": "creator-site",
                "manifest_cid": MANIFEST_CID,
                "owner_passport_subject": "passport:creator",
                "owner_wallet_account": "acct_creator",
                "updated_at_ms": 1
            }),
            field,
        );
    }
}

#[test]
fn public_routes_do_not_expose_replay_verifier_committee_or_finality_surfaces() {
    let source = read_all_src_without_comments();

    for forbidden in [
        "\"/quickchain",
        "\"/replay",
        "\"/proof",
        "\"/verifier",
        "\"/committee",
        "\"/quorum",
        "\"/fork-choice",
        "\"/checkpoint",
        "\"/finality",
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
        assert_not_contains(&source, forbidden, "svc-index public route/source literals");
    }
}

#[test]
fn runtime_source_does_not_construct_phase2_replay_or_verifier_authority() {
    let source = read_all_src_without_comments();

    for forbidden in [
        "ron_policy::",
        "svc_wallet::",
        "ron_ledger::",
        "ron_proto::quickchain",
        "quickchain::",
        "replay_result:",
        "\"replay_result\"",
        "replay_verified:",
        "\"replay_verified\"",
        "verifier_result:",
        "\"verifier_result\"",
        "verifier_attestation:",
        "\"verifier_attestation\"",
        "committee_attestation:",
        "\"committee_attestation\"",
        "committee_vote:",
        "\"committee_vote\"",
        "quorum_reached:",
        "\"quorum_reached\"",
        "fork_choice_winner:",
        "\"fork_choice_winner\"",
        "settlement_finality:",
        "\"settlement_finality\"",
        "index_artifact_proves_replay",
        "index_artifact_proves_proof",
        "index_artifact_proves_finality",
        "pointer_proves_verifier_truth",
        "artifact_pointer_proves_payment",
        "unlock_from_replay_artifact",
        "unlock_from_proof_artifact",
        "unlock_from_index_artifact",
        "produce_proof(",
        "verify_finality(",
        "sign_attestation(",
        "decide_quorum(",
        "decide_fork_choice(",
        "commit_checkpoint(",
        "anchor_checkpoint(",
        "bridge_settlement(",
    ] {
        assert_not_contains(&source, forbidden, "svc-index production source");
    }
}
