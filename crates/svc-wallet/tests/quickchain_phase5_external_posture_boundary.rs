#![cfg(feature = "quickchain-preflight")]
#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 5 Round 3 external-posture boundary tests for svc-wallet.
//! RO:WHY — Wallet receipts may be referenced by the selected anchor-only posture, but that posture must never become spend, unlock, balance, receipt, bridge, market, or outside-settlement authority.
//! RO:INTERACTS — svc_wallet::quickchain wallet receipt projection and external posture evidence helper.
//! RO:INVARIANTS — anchor-only; evidence-only; wallet/ledger truth canonical; no wallet mutation; no paid unlock; no public market.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — prevents selected external posture from becoming wallet authority.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase5_external_posture_boundary.

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
        project_wallet_receipt_for_quickchain_preflight,
        QuickChainWalletExternalPostureEvidenceReport,
        QuickChainWalletExternalPostureEvidenceStatus, QuickChainWalletReceiptProjectionContext,
        SVC_WALLET_QUICKCHAIN_EXTERNAL_POSTURE_EVIDENCE_SCHEMA,
    },
    util::blake3_receipt::finalize_receipt,
};

const CHECKPOINT_HASH: &str = "b3:abababababababababababababababababababababababababababababababab";

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
        txid: "tx_phase5_external_posture_wallet_receipt".to_string(),
        op: WalletOp::Transfer,
        from: Some("acct_phase5_alice".to_string()),
        to: Some("acct_phase5_bob".to_string()),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(42),
        nonce: Some(7),
        idem: "idem_phase5_external_posture_wallet_receipt".to_string(),
        ts: 1_777_500_000_000,
        ledger_seq_start: Some(200),
        ledger_seq_end: Some(201),
        ledger_root: "22".repeat(32),
        settlement_status: ReceiptSettlementStatus::Accepted,
        receipt_hash: String::new(),
    })
    .expect("dummy wallet receipt should hash")
}

fn posture_report() -> QuickChainWalletExternalPostureEvidenceReport {
    let receipt = dummy_wallet_receipt();
    let context = QuickChainWalletReceiptProjectionContext::accepted(
        "roc-dev",
        "op:wallet:transfer:phase5-external-posture",
    )
    .expect("explicit wallet projection context should validate");
    let projection = project_wallet_receipt_for_quickchain_preflight(&receipt, &context)
        .expect("wallet receipt should project");

    QuickChainWalletExternalPostureEvidenceReport::new_from_projection(
        &projection,
        "posture:phase5-round3:anchor-only",
        CHECKPOINT_HASH,
        1_777_500_000_123,
    )
    .expect("external posture evidence report should validate")
}

#[test]
fn wallet_external_posture_evidence_is_anchor_only_and_read_only() {
    let report = posture_report();

    assert_eq!(
        report.schema,
        SVC_WALLET_QUICKCHAIN_EXTERNAL_POSTURE_EVIDENCE_SCHEMA
    );
    assert_eq!(report.chain_id, "roc-dev");
    assert_eq!(report.posture_id, "posture:phase5-round3:anchor-only");
    assert_eq!(report.checkpoint_hash, CHECKPOINT_HASH);
    assert_eq!(
        report.status,
        QuickChainWalletExternalPostureEvidenceStatus::AnchorOnlyEvidenceOnly
    );

    assert!(report.anchor_only_selected);
    assert!(report.evidence_only);
    assert!(report.wallet_ledger_truth_canonical);

    assert!(!report.wallet_side_effect);
    assert!(!report.receipt_side_effect);
    assert!(!report.balance_side_effect);
    assert!(!report.hold_side_effect);
    assert!(!report.paid_unlock_authority);
    assert!(!report.outside_settlement);
    assert!(!report.bridge_authority);
    assert!(!report.outside_program_authority);
    assert!(!report.listing_authority);
    assert!(!report.public_market);
    assert!(!report.liquidity_enabled);
    assert!(!report.bonded_economy_authority);

    let encoded = serde_json::to_string(&report).expect("report should serialize");
    assert!(encoded.contains(r#""schema":"svc-wallet.quickchain-external-posture-evidence.v1""#));
    assert!(encoded.contains(r#""status":"anchor_only_evidence_only""#));
    assert!(encoded.contains(r#""anchor_only_selected":true"#));
}

#[test]
fn wallet_external_posture_evidence_rejects_bad_shape_and_missing_truth_flags() {
    let mut report = posture_report();
    report.produced_at_ms = 0;
    assert!(report.validate().is_err(), "produced_at_ms must be nonzero");

    let mut report = posture_report();
    report.anchor_only_selected = false;
    assert!(
        report.validate().is_err(),
        "anchor-only selection must be explicit"
    );

    let mut report = posture_report();
    report.evidence_only = false;
    assert!(report.validate().is_err(), "evidence-only must be explicit");

    let mut report = posture_report();
    report.wallet_ledger_truth_canonical = false;
    assert!(
        report.validate().is_err(),
        "wallet/ledger truth must remain canonical"
    );

    let mut report = posture_report();
    report.checkpoint_hash = "B3:BAD".to_string();
    assert!(
        report.validate().is_err(),
        "checkpoint hash must be canonical b3"
    );
}

#[test]
fn wallet_external_posture_evidence_rejects_authority_flags_and_unknown_fields() {
    let clean = serde_json::to_value(posture_report()).expect("report should serialize");

    for field in [
        "anchor_only_selected",
        "evidence_only",
        "wallet_ledger_truth_canonical",
    ] {
        let mut poisoned = clean.clone();
        poisoned[field] = Value::Bool(false);

        let decoded =
            serde_json::from_value::<QuickChainWalletExternalPostureEvidenceReport>(poisoned)
                .expect("known field should deserialize before validation");

        assert!(
            decoded.validate().is_err(),
            "external posture report must reject false required flag {field}"
        );
    }

    for field in [
        "wallet_side_effect",
        "receipt_side_effect",
        "balance_side_effect",
        "hold_side_effect",
        "paid_unlock_authority",
        "outside_settlement",
        "bridge_authority",
        "outside_program_authority",
        "listing_authority",
        "public_market",
        "liquidity_enabled",
        "bonded_economy_authority",
    ] {
        let mut poisoned = clean.clone();
        poisoned[field] = Value::Bool(true);

        let decoded =
            serde_json::from_value::<QuickChainWalletExternalPostureEvidenceReport>(poisoned)
                .expect("known authority field should deserialize before validation");

        assert!(
            decoded.validate().is_err(),
            "external posture report must reject authority field {field}"
        );
    }

    let mut unknown = clean;
    unknown["external_settlement_authorized"] = json!(true);
    assert!(
        serde_json::from_value::<QuickChainWalletExternalPostureEvidenceReport>(unknown).is_err(),
        "unknown external posture authority fields must reject"
    );
}

#[test]
fn live_wallet_requests_reject_external_posture_authority_poison_fields() {
    let issue = json!({
        "to": "acct_phase5_bob",
        "asset": "roc",
        "amount_minor": "1",
        "idempotency_key": "idem_phase5_posture_issue",
        "external_posture": "anchor_only",
        "external_settlement_authorized": true
    });
    assert!(
        serde_json::from_value::<IssueRequest>(issue).is_err(),
        "issue request must reject external posture authority poison fields"
    );

    let transfer = json!({
        "from": "acct_phase5_alice",
        "to": "acct_phase5_bob",
        "asset": "roc",
        "amount_minor": "1",
        "nonce": 1,
        "idempotency_key": "idem_phase5_posture_transfer",
        "bridge_authority": true
    });
    assert!(
        serde_json::from_value::<TransferRequest>(transfer).is_err(),
        "transfer request must reject bridge authority poison fields"
    );

    let burn = json!({
        "from": "acct_phase5_alice",
        "asset": "roc",
        "amount_minor": "1",
        "nonce": 1,
        "idempotency_key": "idem_phase5_posture_burn",
        "paid_unlock_authority": true
    });
    assert!(
        serde_json::from_value::<BurnRequest>(burn).is_err(),
        "burn request must reject paid unlock poison fields"
    );
}

#[test]
fn wallet_routes_do_not_expose_external_posture_mutation_surfaces() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src").join("routes"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-wallet route files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "external_posture",
            "posture_authority",
            "bridge_authority",
            "outside_settlement",
            "public_market",
            "liquidity_enabled",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-wallet routes must not expose external posture mutation surface `{forbidden}` in {}",
                path.display()
            );
        }
    }
}

#[test]
fn wallet_source_does_not_construct_hidden_external_posture_runtime_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-wallet Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "commit_from_external_posture",
            "apply_external_posture",
            "mutate_from_external_posture",
            "unlock_from_external_posture",
            "create_external_posture_receipt",
            "bridge_settlement",
            "external_settlement",
            "exchange_facing",
            "solana",
            "rox",
            "paid_unlock_authority: true",
            "wallet_side_effect: true",
            "receipt_side_effect: true",
            "balance_side_effect: true",
            "hold_side_effect: true",
            "outside_settlement: true",
            "bridge_authority: true",
            "outside_program_authority: true",
            "listing_authority: true",
            "public_market: true",
            "liquidity_enabled: true",
            "bonded_economy_authority: true",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-wallet source must not construct hidden external posture authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
