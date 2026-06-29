#![cfg(feature = "quickchain-preflight")]
#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 5 Round 1 anchor evidence boundary tests for svc-wallet.
//! RO:WHY — Wallet receipts may be referenced by anchor dry-run metadata, but anchors must not mutate wallet/ledger truth.
//! RO:INTERACTS — svc_wallet::quickchain wallet receipt projection and anchor evidence report helper.
//! RO:INVARIANTS — anchor evidence is read-only; no balance, hold, receipt, paid unlock, bridge, or outside-settlement authority.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — prevents anchor evidence from becoming spend, receipt, unlock, bridge, or external-chain truth.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase5_anchor_evidence_boundary.

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
        project_wallet_receipt_for_quickchain_preflight, QuickChainWalletAnchorEvidenceReport,
        QuickChainWalletAnchorEvidenceStatus, QuickChainWalletReceiptProjectionContext,
        SVC_WALLET_QUICKCHAIN_ANCHOR_EVIDENCE_SCHEMA,
    },
    util::blake3_receipt::finalize_receipt,
};

const CHECKPOINT_HASH: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

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

fn dummy_wallet_receipt() -> Receipt {
    finalize_receipt(Receipt {
        txid: "tx_phase5_anchor_wallet_receipt".to_string(),
        op: WalletOp::Transfer,
        from: Some("acct_phase5_alice".to_string()),
        to: Some("acct_phase5_bob".to_string()),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(42),
        nonce: Some(7),
        idem: "idem_phase5_anchor_wallet_receipt".to_string(),
        ts: 1_777_500_000_000,
        ledger_seq_start: Some(200),
        ledger_seq_end: Some(201),
        ledger_root: "22".repeat(32),
        settlement_status: ReceiptSettlementStatus::Accepted,
        receipt_hash: String::new(),
    })
    .expect("dummy wallet receipt should hash")
}

fn anchor_report() -> QuickChainWalletAnchorEvidenceReport {
    let receipt = dummy_wallet_receipt();
    let context = QuickChainWalletReceiptProjectionContext::accepted(
        "roc-dev",
        "op:wallet:transfer:phase5-anchor-evidence",
    )
    .expect("explicit wallet projection context should validate");
    let projection = project_wallet_receipt_for_quickchain_preflight(&receipt, &context)
        .expect("wallet receipt should project");

    QuickChainWalletAnchorEvidenceReport::new_from_projection(
        &projection,
        "anchor-dry-run:phase5:r1:wallet",
        CHECKPOINT_HASH,
        1_777_500_001_000,
    )
    .expect("anchor evidence report should build")
}

fn assert_no_key(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert_ne!(
                    key, forbidden,
                    "wallet anchor evidence must not expose forbidden authority key `{forbidden}`"
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

fn clean_issue_request() -> Value {
    json!({
        "to": "acct_phase5_anchor_to",
        "asset": "roc",
        "amount_minor": "1",
        "idempotency_key": "idem-phase5-anchor-issue",
        "memo": "phase5 anchor poison test"
    })
}

fn clean_transfer_request() -> Value {
    json!({
        "from": "acct_phase5_anchor_from",
        "to": "acct_phase5_anchor_to",
        "asset": "roc",
        "amount_minor": "1",
        "nonce": 1,
        "idempotency_key": "idem-phase5-anchor-transfer",
        "memo": "phase5 anchor poison test"
    })
}

fn clean_burn_request() -> Value {
    json!({
        "from": "acct_phase5_anchor_from",
        "asset": "roc",
        "amount_minor": "1",
        "nonce": 1,
        "idempotency_key": "idem-phase5-anchor-burn",
        "memo": "phase5 anchor poison test"
    })
}

#[test]
fn wallet_anchor_evidence_report_is_read_only_metadata() {
    let report = anchor_report();
    report.validate().expect("report should validate");

    assert_eq!(report.schema, SVC_WALLET_QUICKCHAIN_ANCHOR_EVIDENCE_SCHEMA);
    assert_eq!(
        report.status,
        QuickChainWalletAnchorEvidenceStatus::DryRunEvidenceOnly
    );
    assert_eq!(report.checkpoint_hash, CHECKPOINT_HASH);
    assert!(report.wallet_receipt_hash.starts_with("b3:"));
    assert!(report.evidence_only);
    assert!(!report.wallet_side_effect);
    assert!(!report.receipt_side_effect);
    assert!(!report.balance_side_effect);
    assert!(!report.hold_side_effect);
    assert!(!report.paid_unlock_authority);
    assert!(!report.outside_settlement);
    assert!(!report.outside_chain_truth);

    let value = serde_json::to_value(&report).expect("report should serialize");

    for forbidden in [
        "balance_mutation",
        "receipt_mutation",
        "hold_mutation",
        "wallet_mutation",
        "paid_unlock",
        "state_root",
        "receipt_root",
        "settlement_status",
        "finalized",
        "anchored",
        "external_settlement",
        "bridge_settlement",
        "solana",
        "rox",
    ] {
        assert_no_key(&value, forbidden);
    }
}

#[test]
fn wallet_anchor_evidence_rejects_authority_flags_and_unknown_fields() {
    let clean = serde_json::to_value(anchor_report()).expect("report should serialize");

    for field in [
        "wallet_side_effect",
        "receipt_side_effect",
        "balance_side_effect",
        "hold_side_effect",
        "paid_unlock_authority",
        "outside_settlement",
        "outside_chain_truth",
    ] {
        let mut poisoned =
            serde_json::from_value::<QuickChainWalletAnchorEvidenceReport>(clean.clone())
                .expect("clean report should deserialize");

        match field {
            "wallet_side_effect" => poisoned.wallet_side_effect = true,
            "receipt_side_effect" => poisoned.receipt_side_effect = true,
            "balance_side_effect" => poisoned.balance_side_effect = true,
            "hold_side_effect" => poisoned.hold_side_effect = true,
            "paid_unlock_authority" => poisoned.paid_unlock_authority = true,
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
            .insert(field.to_string(), json!("client-supplied-anchor-authority"));

        assert!(
            serde_json::from_value::<QuickChainWalletAnchorEvidenceReport>(poisoned).is_err(),
            "report DTO must reject unknown authority field: {field}"
        );
    }
}

#[test]
fn wallet_anchor_evidence_rejects_bad_checkpoint_hash_or_timestamp() {
    let mut report = anchor_report();
    report.checkpoint_hash = "not-a-b3-hash".to_string();
    assert!(
        report.validate().is_err(),
        "checkpoint hash must be canonical b3"
    );

    let mut report = anchor_report();
    report.produced_at_ms = 0;
    assert!(
        report.validate().is_err(),
        "produced_at_ms must be explicit and nonzero"
    );
}

#[test]
fn wallet_mutation_requests_reject_anchor_authority_poison_fields() {
    for field in [
        "anchor_id",
        "checkpoint_hash",
        "anchor_receipt",
        "anchor_status",
        "anchor_unlock",
        "anchor_settlement",
        "wallet_side_effect",
        "receipt_side_effect",
        "balance_side_effect",
        "hold_side_effect",
        "paid_unlock_authority",
        "outside_settlement",
        "outside_chain_truth",
        "external_settlement",
        "bridge_settlement",
    ] {
        let mut issue = clean_issue_request();
        issue
            .as_object_mut()
            .expect("issue JSON should be object")
            .insert(field.to_string(), json!("client-supplied-anchor-authority"));
        assert!(
            serde_json::from_value::<IssueRequest>(issue).is_err(),
            "IssueRequest must reject Phase 5 anchor authority field: {field}"
        );

        let mut transfer = clean_transfer_request();
        transfer
            .as_object_mut()
            .expect("transfer JSON should be object")
            .insert(field.to_string(), json!("client-supplied-anchor-authority"));
        assert!(
            serde_json::from_value::<TransferRequest>(transfer).is_err(),
            "TransferRequest must reject Phase 5 anchor authority field: {field}"
        );

        let mut burn = clean_burn_request();
        burn.as_object_mut()
            .expect("burn JSON should be object")
            .insert(field.to_string(), json!("client-supplied-anchor-authority"));
        assert!(
            serde_json::from_value::<BurnRequest>(burn).is_err(),
            "BurnRequest must reject Phase 5 anchor authority field: {field}"
        );
    }
}

#[test]
fn wallet_source_does_not_implement_phase5_anchor_runtime_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-wallet Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path));

        for forbidden in [
            "commit_from_anchor(",
            "apply_anchor(",
            "mutate_from_anchor(",
            "unlock_from_anchor(",
            "create_anchor_receipt(",
            "bridge_settlement",
            "external_settlement",
            "paid_unlock_authority: true",
            "wallet_side_effect: true",
            "receipt_side_effect: true",
            "balance_side_effect: true",
            "hold_side_effect: true",
            "outside_settlement: true",
            "outside_chain_truth: true",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-wallet source must not implement Phase 5 anchor runtime authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
