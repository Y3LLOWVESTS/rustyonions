//! RO:WHAT — Phase 2 Round 2 committee-readiness boundary tests for svc-rewarder.
//! RO:WHY — Rewarder artifacts may be replay inputs later, but rewarder must not become committee, quorum, fork-choice, finality, settlement, or validator-economy authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, outputs::{attestation,intents,manifest,wallet}, source boundary.
//! RO:INVARIANTS — rewarder plans only; svc-wallet commits approved payout intents; ron-ledger remains durable economic truth.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects committee/quorum poison fields and blocks validator-economy authority creep.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_phase2_committee_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use svc_rewarder::outputs::{Attestation, WalletIssueRequest};

const COMMITTEE_AUTHORITY_KEYS: &[&str] = &[
    "committee_member_id",
    "committee_epoch",
    "committee_round",
    "committee_signature",
    "committee_signatures",
    "signed_verification_attestation",
    "verification_attestation",
    "attestation_signature",
    "attestation_public_key",
    "attestation_weight",
    "quorum_certificate",
    "quorum_threshold",
    "quorum_reached",
    "validator_signature",
    "validator_set",
    "validator_index",
    "fork_choice",
    "double_attestation_evidence",
    "equivocation_evidence",
    "bonded_stake",
    "stake_weight",
    "slash_evidence",
    "slashing",
    "external_anchor",
    "external_settlement",
    "bridge_finality",
    "settlement_finality",
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
}

fn normalized(text: &str) -> String {
    text.to_ascii_lowercase().replace('`', "")
}

fn collect_rs_files(root: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|err| {
        panic!("failed to read directory {}: {err}", root.display());
    });

    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();

        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "target")
        {
            continue;
        }

        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
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

fn assert_contains(haystack: &str, needle: &str, context: &str) {
    assert!(
        haystack.contains(needle),
        "{context} must preserve required marker: {needle}"
    );
}

#[test]
fn docs_name_phase2_round2_committee_readiness_boundary() {
    let doc = normalized(&read(crate_dir().join("docs/quickchain-preflight.md")));

    for required in [
        "phase 2 round 2 committee readiness boundary",
        "reward manifests remain payout planning artifacts",
        "wallet issue requests remain explicit svc-wallet handoff previews",
        "svc-rewarder is not a committee member",
        "svc-rewarder does not produce signed verification attestations",
        "svc-rewarder does not decide quorum",
        "svc-rewarder cannot claim fork choice",
        "svc-rewarder cannot claim finality",
        "svc-rewarder cannot create validator rewards from raw engagement",
        "svc-rewarder cannot mutate ledger truth",
        "svc-wallet commits approved payout intents",
        "ron-ledger remains durable economic truth",
        "quickchain_phase2_committee_boundary",
    ] {
        assert_contains(&doc, required, "svc-rewarder quickchain-preflight.md");
    }
}

#[test]
fn rewarder_wire_edges_reject_committee_attestation_poison_fields() {
    let clean_wallet_issue = json!({
        "to": "acct_phase2_round2_rewarder_recipient",
        "asset": "roc",
        "amount_minor": "77",
        "idempotency_key": "idem_phase2_round2_rewarder_wallet_preview",
        "memo": "svc-rewarder:phase2-round2:acct_phase2_round2_rewarder_recipient"
    });

    let clean_manifest_attestation = json!({
        "sig_ed25519": null,
        "sig_pq": null,
        "signed_at_millis": 1_777_500_000_000_u64
    });

    serde_json::from_value::<WalletIssueRequest>(clean_wallet_issue.clone())
        .expect("clean wallet issue preview should deserialize");
    serde_json::from_value::<Attestation>(clean_manifest_attestation.clone())
        .expect("clean manifest attestation should deserialize");

    for field in COMMITTEE_AUTHORITY_KEYS {
        let mut poisoned_issue = clean_wallet_issue.clone();
        poisoned_issue
            .as_object_mut()
            .expect("wallet issue JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-committee-authority"),
            );

        assert!(
            serde_json::from_value::<WalletIssueRequest>(poisoned_issue).is_err(),
            "WalletIssueRequest must reject Phase 2 Round 2 committee poison field: {field}"
        );

        let mut poisoned_attestation = clean_manifest_attestation.clone();
        poisoned_attestation
            .as_object_mut()
            .expect("attestation JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-committee-authority"),
            );

        assert!(
            serde_json::from_value::<Attestation>(poisoned_attestation).is_err(),
            "rewarder manifest Attestation must reject Phase 2 Round 2 committee poison field: {field}"
        );
    }
}

#[test]
fn rewarder_preserves_planning_and_wallet_handoff_seams_without_committee_authority() {
    let manifest = read(crate_dir().join("src/outputs/manifest.rs"));
    let intents = read(crate_dir().join("src/outputs/intents.rs"));
    let wallet = read(crate_dir().join("src/outputs/wallet.rs"));
    let attestation = read(crate_dir().join("src/outputs/attestation.rs"));

    for required in [
        "RewardManifest",
        "commitment_for_manifest",
        "RewardPayout",
        "RewardTotals",
        "SettlementBatch",
        "WalletIssueRequest",
        "to_wallet_issue_request",
        "WALLET_ISSUE_PATH",
        "Idempotency-Key",
        "WalletIssueClient",
        "post_issue",
        "#[serde(deny_unknown_fields)]",
    ] {
        assert!(
            manifest.contains(required)
                || intents.contains(required)
                || wallet.contains(required)
                || attestation.contains(required),
            "svc-rewarder planning/wallet handoff source must preserve marker: {required}"
        );
    }

    let combined = strip_line_comments(&format!("{manifest}\n{intents}\n{wallet}\n{attestation}"));

    for forbidden in [
        "CommitteeAttestation",
        "SignedVerificationAttestation",
        "QuorumCertificate",
        "committee_member_id",
        "committee_epoch",
        "committee_round",
        "committee_signature",
        "committee_signatures",
        "signed_verification_attestation",
        "verification_attestation",
        "attestation_signature",
        "quorum_certificate",
        "quorum_threshold",
        "quorum_reached",
        "validator_set",
        "validator_signature",
        "fork_choice",
        "double_attestation_evidence",
        "equivocation_evidence",
        "bonded_stake",
        "stake_weight",
        "slash_evidence",
        "bridge_finality",
        "settlement_finality",
        "external_settlement",
    ] {
        assert!(
            !combined.contains(forbidden),
            "svc-rewarder planning/wallet handoff source must not expose committee/finality authority marker `{forbidden}`"
        );
    }
}

#[test]
fn rewarder_source_does_not_implement_committee_or_validator_economy_runtime() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-rewarder Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path));

        for forbidden in [
            "QuickChainCommittee",
            "CommitteeAttestation",
            "SignedVerificationAttestation",
            "QuorumCertificate",
            "committee_member_id",
            "committee_epoch",
            "committee_round",
            "committee_signature",
            "committee_signatures",
            "signed_verification_attestation",
            "verification_attestation",
            "attestation_signature",
            "quorum_certificate",
            "quorum_threshold",
            "quorum_reached",
            "validator_set",
            "validator_signature",
            "fork_choice",
            "double_attestation_evidence",
            "equivocation_evidence",
            "bonded_stake",
            "stake_weight",
            "slash_evidence",
            "slashing",
            "bridge_finality",
            "settlement_finality",
            "external_settlement",
            "reward_from_raw_engagement",
            "mint_from_views",
            "mint_from_clicks",
            "mint_from_watch_seconds",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-rewarder source must not implement Phase 2 Round 2 committee/validator-economy authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}

#[test]
fn serialized_rewarder_handoff_dtos_have_no_committee_authority_keys() {
    let issue = WalletIssueRequest {
        to: "acct_phase2_round2_rewarder_preview".to_string(),
        asset: "roc".to_string(),
        amount_minor: "123".to_string(),
        idempotency_key: Some("idem_phase2_round2_rewarder_preview".to_string()),
        memo: Some("svc-rewarder:phase2-round2:preview".to_string()),
    };

    let value = serde_json::to_value(issue).expect("wallet issue preview should serialize");

    for forbidden in COMMITTEE_AUTHORITY_KEYS {
        assert_no_key_recursive(&value, forbidden);
    }
}

fn assert_no_key_recursive(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert!(
                    key != forbidden,
                    "rewarder handoff DTO must not expose Phase 2 Round 2 committee authority key `{forbidden}`"
                );
                assert_no_key_recursive(nested, forbidden);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_key_recursive(nested, forbidden);
            }
        }
        _ => {}
    }
}
