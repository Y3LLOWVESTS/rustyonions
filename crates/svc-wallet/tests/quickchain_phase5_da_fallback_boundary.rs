#![cfg(feature = "quickchain-preflight")]
#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 5 Round 2 DA/archive/challenge fallback boundary tests for svc-wallet.
//! RO:WHY — Wallet receipts may be referenced by fallback evidence, but DA/archive/challenge material must never become spend, unlock, balance, receipt, or outside-truth authority.
//! RO:INTERACTS — svc_wallet::quickchain wallet receipt projection and DA fallback evidence report helper.
//! RO:INVARIANTS — evidence-only; archive/challenge checked; deletion shortcuts blocked; no wallet/ledger mutation; no paid unlock; no outside settlement.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — prevents DA/archive/challenge evidence from becoming wallet authority.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase5_da_fallback_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use svc_wallet::{
    dto::{
        requests::{AmountMinor, BurnRequest, IssueRequest, TransferRequest},
        responses::{Receipt, ReceiptSettlementStatus, WalletOp},
    },
    quickchain::{
        project_wallet_receipt_for_quickchain_preflight, QuickChainWalletDaFallbackEvidenceReport,
        QuickChainWalletDaFallbackEvidenceStatus, QuickChainWalletReceiptProjectionContext,
        SVC_WALLET_QUICKCHAIN_DA_FALLBACK_EVIDENCE_SCHEMA,
    },
    util::blake3_receipt::finalize_receipt,
};

const CHECKPOINT_HASH: &str = "b3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const DA_ROOT: &str = "b3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
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
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "rs")
        {
            files.push(path);
        }
    }
}

fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with("///"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_no_key(value: &Value, key: &str) {
    match value {
        Value::Object(map) => {
            assert!(
                !map.contains_key(key),
                "serialized DA fallback wallet evidence must not expose authority key `{key}`: {value}"
            );
            for child in map.values() {
                assert_no_key(child, key);
            }
        }
        Value::Array(items) => {
            for child in items {
                assert_no_key(child, key);
            }
        }
        _ => {}
    }
}

fn dummy_wallet_receipt() -> Receipt {
    finalize_receipt(Receipt {
        txid: "tx_phase5_da_fallback_wallet_receipt".to_string(),
        op: WalletOp::Transfer,
        from: Some("acct_phase5_da_alice".to_string()),
        to: Some("acct_phase5_da_bob".to_string()),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(42),
        nonce: Some(8),
        idem: "idem_phase5_da_fallback_wallet_receipt".to_string(),
        ts: 1_777_600_000_000,
        ledger_seq_start: Some(300),
        ledger_seq_end: Some(301),
        ledger_root: "33".repeat(32),
        settlement_status: ReceiptSettlementStatus::Accepted,
        receipt_hash: String::new(),
    })
    .expect("dummy wallet receipt should hash")
}

fn fallback_report() -> QuickChainWalletDaFallbackEvidenceReport {
    let receipt = dummy_wallet_receipt();
    let context = QuickChainWalletReceiptProjectionContext::accepted(
        "roc-dev",
        "op:wallet:transfer:phase5-da-fallback",
    )
    .expect("explicit wallet projection context should validate");
    let projection = project_wallet_receipt_for_quickchain_preflight(&receipt, &context)
        .expect("wallet receipt should project");

    QuickChainWalletDaFallbackEvidenceReport::new_from_projection(
        &projection,
        "fallback:plan:phase5:r2:wallet",
        CHECKPOINT_HASH,
        DA_ROOT,
        Some("chunk:economic-receipts:0001".to_string()),
        1_777_600_001_000,
    )
    .expect("fallback evidence report should validate")
}

#[test]
fn wallet_da_fallback_evidence_is_report_only_and_blocks_pruning_authority() {
    let report = fallback_report();

    assert_eq!(
        report.schema,
        SVC_WALLET_QUICKCHAIN_DA_FALLBACK_EVIDENCE_SCHEMA
    );
    assert_eq!(
        report.status,
        QuickChainWalletDaFallbackEvidenceStatus::ArchiveChallengeEvidenceOnly
    );
    assert_eq!(report.checkpoint_hash, CHECKPOINT_HASH);
    assert_eq!(report.data_availability_root, DA_ROOT);
    assert!(report.wallet_receipt_hash.starts_with("b3:"));
    assert!(report.evidence_only);
    assert!(report.archive_fallback_checked);
    assert!(report.missing_data_challenge_checked);
    assert!(report.restore_path_checked);
    assert!(report.pruning_blocked);

    assert!(!report.wallet_side_effect);
    assert!(!report.receipt_side_effect);
    assert!(!report.balance_side_effect);
    assert!(!report.hold_side_effect);
    assert!(!report.paid_unlock_authority);
    assert!(!report.pruning_authority);
    assert!(!report.outside_data_availability_truth);
    assert!(!report.outside_settlement);
    assert!(!report.outside_chain_truth);

    let value = serde_json::to_value(&report).expect("report should serialize");

    for forbidden in [
        "wallet_mutation",
        "ledger_mutation",
        "balance_mutation",
        "receipt_mutation",
        "hold_mutation",
        "paid_unlock",
        "settlement_status",
        "finalized",
        "anchored",
        "bridge_settlement",
        "external_settlement",
        "solana",
        "rox",
    ] {
        assert_no_key(&value, forbidden);
    }
}

#[test]
fn wallet_da_fallback_evidence_rejects_authority_flags_and_unknown_fields() {
    let clean = serde_json::to_value(fallback_report()).expect("report should serialize");

    for field in [
        "evidence_only",
        "archive_fallback_checked",
        "missing_data_challenge_checked",
        "restore_path_checked",
        "pruning_blocked",
    ] {
        let mut poisoned =
            serde_json::from_value::<QuickChainWalletDaFallbackEvidenceReport>(clean.clone())
                .expect("clean report should deserialize");

        match field {
            "evidence_only" => poisoned.evidence_only = false,
            "archive_fallback_checked" => poisoned.archive_fallback_checked = false,
            "missing_data_challenge_checked" => poisoned.missing_data_challenge_checked = false,
            "restore_path_checked" => poisoned.restore_path_checked = false,
            "pruning_blocked" => poisoned.pruning_blocked = false,
            _ => unreachable!("covered above"),
        }

        assert!(
            poisoned.validate().is_err(),
            "required blocker/check flag must reject when disabled: {field}"
        );
    }

    for field in [
        "wallet_side_effect",
        "receipt_side_effect",
        "balance_side_effect",
        "hold_side_effect",
        "paid_unlock_authority",
        "pruning_authority",
        "outside_data_availability_truth",
        "outside_settlement",
        "outside_chain_truth",
    ] {
        let mut poisoned =
            serde_json::from_value::<QuickChainWalletDaFallbackEvidenceReport>(clean.clone())
                .expect("clean report should deserialize");

        match field {
            "wallet_side_effect" => poisoned.wallet_side_effect = true,
            "receipt_side_effect" => poisoned.receipt_side_effect = true,
            "balance_side_effect" => poisoned.balance_side_effect = true,
            "hold_side_effect" => poisoned.hold_side_effect = true,
            "paid_unlock_authority" => poisoned.paid_unlock_authority = true,
            "pruning_authority" => poisoned.pruning_authority = true,
            "outside_data_availability_truth" => poisoned.outside_data_availability_truth = true,
            "outside_settlement" => poisoned.outside_settlement = true,
            "outside_chain_truth" => poisoned.outside_chain_truth = true,
            _ => unreachable!("covered above"),
        }

        assert!(
            poisoned.validate().is_err(),
            "authority flag must reject when enabled: {field}"
        );
    }

    for field in [
        "wallet_mutation",
        "ledger_mutation",
        "balance_minor",
        "available_minor",
        "wallet_receipt_created",
        "paid_unlock",
        "settlement_status",
        "finalized",
        "anchored",
        "external_settlement",
        "bridge_settlement",
        "solana",
        "rox",
    ] {
        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("report JSON should be object")
            .insert(field.to_string(), json!("client-supplied-da-authority"));

        assert!(
            serde_json::from_value::<QuickChainWalletDaFallbackEvidenceReport>(poisoned).is_err(),
            "report DTO must reject unknown authority field: {field}"
        );
    }
}

#[test]
fn wallet_da_fallback_evidence_rejects_bad_hashes_or_missing_challenge_context() {
    let mut report = fallback_report();
    report.checkpoint_hash = "not-a-b3-hash".to_string();
    assert!(
        report.validate().is_err(),
        "checkpoint hash must be canonical b3"
    );

    let mut report = fallback_report();
    report.data_availability_root = "b3:ABC".to_string();
    assert!(
        report.validate().is_err(),
        "DA root must be canonical b3 lowercase"
    );

    let mut report = fallback_report();
    report.challenged_chunk_id = Some("bad chunk id with spaces".to_string());
    assert!(
        report.validate().is_err(),
        "challenged chunk id must be bounded visible token"
    );

    let mut report = fallback_report();
    report.produced_at_ms = 0;
    assert!(report.validate().is_err(), "produced_at_ms must be nonzero");
}

#[test]
fn live_wallet_requests_reject_da_fallback_authority_poison_fields() {
    for field in [
        "fallback_plan_id",
        "data_availability_root",
        "challenged_chunk_id",
        "archive_fallback_checked",
        "missing_data_challenge_checked",
        "restore_path_checked",
        "pruning_blocked",
        "pruning_authority",
        "outside_data_availability_truth",
        "outside_settlement",
    ] {
        let mut issue = json!({
            "to": "acct_phase5_da_issue",
            "asset": "roc",
            "amount_minor": "10",
            "idempotency_key": "idem_phase5_da_issue",
            "memo": null
        });
        issue
            .as_object_mut()
            .expect("issue body should be object")
            .insert(field.to_string(), json!("smuggled-da-authority"));
        assert!(
            serde_json::from_value::<IssueRequest>(issue).is_err(),
            "IssueRequest must reject DA fallback poison field: {field}"
        );

        let mut transfer = json!({
            "from": "acct_phase5_da_a",
            "to": "acct_phase5_da_b",
            "asset": "roc",
            "amount_minor": "10",
            "nonce": 1,
            "idempotency_key": "idem_phase5_da_transfer",
            "memo": null
        });
        transfer
            .as_object_mut()
            .expect("transfer body should be object")
            .insert(field.to_string(), json!("smuggled-da-authority"));
        assert!(
            serde_json::from_value::<TransferRequest>(transfer).is_err(),
            "TransferRequest must reject DA fallback poison field: {field}"
        );

        let mut burn = json!({
            "from": "acct_phase5_da_a",
            "asset": "roc",
            "amount_minor": "10",
            "nonce": 1,
            "idempotency_key": "idem_phase5_da_burn",
            "memo": null
        });
        burn.as_object_mut()
            .expect("burn body should be object")
            .insert(field.to_string(), json!("smuggled-da-authority"));
        assert!(
            serde_json::from_value::<BurnRequest>(burn).is_err(),
            "BurnRequest must reject DA fallback poison field: {field}"
        );
    }
}

#[test]
fn wallet_routes_do_not_expose_da_fallback_mutation_surface() {
    let routes_mod = strip_line_comments(&read(crate_dir().join("src/routes/v1/mod.rs")));
    let routes_dir = crate_dir().join("src/routes/v1");

    for forbidden_route in [
        "\"/da-fallback",
        "\"/da_fallback",
        "\"/archive-challenge",
        "\"/missing-data",
        "\"/prune",
        "\"/pruning",
        "\"/external-da",
    ] {
        assert!(
            !routes_mod.contains(forbidden_route),
            "svc-wallet route module must not expose DA fallback authority route: {forbidden_route}"
        );
    }

    for route in [
        "issue.rs",
        "transfer.rs",
        "burn.rs",
        "escrow.rs",
        "receipt.rs",
        "balance.rs",
    ] {
        let code = strip_line_comments(&read(routes_dir.join(route)));
        for forbidden in [
            "QuickChainWalletDaFallbackEvidenceReport",
            "da_fallback",
            "data_availability",
            "archive_restore",
            "missing_data_challenge",
            "pruning_authority",
        ] {
            assert!(
                !code.contains(forbidden),
                "{route} must not become a hidden DA fallback authority path via {forbidden}"
            );
        }
    }
}

#[test]
fn wallet_source_does_not_construct_da_fallback_wallet_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-wallet Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "issue_from_da_fallback",
            "transfer_from_da_fallback",
            "burn_from_da_fallback",
            "hold_from_da_fallback",
            "capture_from_da_fallback",
            "release_from_da_fallback",
            "unlock_from_da_fallback",
            "settle_from_da_fallback",
            "bridge_from_da_fallback",
            "wallet_receipt_created: true",
            "wallet_side_effect: true",
            "receipt_side_effect: true",
            "balance_side_effect: true",
            "hold_side_effect: true",
            "paid_unlock_authority: true",
            "pruning_authority: true",
            "outside_data_availability_truth: true",
            "outside_settlement: true",
            "outside_chain_truth: true",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-wallet source must not construct DA fallback wallet authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
