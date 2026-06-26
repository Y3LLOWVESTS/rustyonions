#![cfg(feature = "quickchain-preflight")]
#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 4 Round 2 disputed-bond review boundary tests for svc-wallet.
//! RO:WHY — Slashing/challenge simulation may be displayed/reviewed by the wallet,
//! but must not become silent spend, balance locking, live penalty execution,
//! fake receipt creation, finality, bridge, liquidity, or public staking authority.
//! RO:INTERACTS — svc_wallet::quickchain disputed-bond review helper and v1 request DTOs.
//! RO:INVARIANTS — review-only; explicit confirmation; no live wallet mutation;
//! no automatic irreversible slash; no fake receipt.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — prevents Phase 4 Round 2 simulation from becoming wallet authority.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase4_bond_dispute_review_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::json;
use svc_wallet::{
    dto::requests::{BurnRequest, IssueRequest, TransferRequest},
    quickchain::{
        QuickChainWalletBondDisputeAction, QuickChainWalletBondDisputeReview,
        QuickChainWalletBondDisputeReviewStatus, SVC_WALLET_QUICKCHAIN_BOND_DISPUTE_REVIEW_SCHEMA,
    },
};

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

fn valid_review() -> QuickChainWalletBondDisputeReview {
    QuickChainWalletBondDisputeReview::review_only(
        "roc-dev",
        "bond-dispute:phase4:r2:alice",
        "bond-account:phase4:r2:alice",
        "acct_phase4_bond_alice",
        "operator:alice",
        500,
        125,
        "idem_phase4_bond_dispute_review",
        QuickChainWalletBondDisputeAction::FreezePendingAppeal,
    )
    .expect("valid disputed-bond review should build")
}

fn clean_issue_request() -> serde_json::Value {
    json!({
        "to": "acct_phase4_bond_alice",
        "asset": "roc",
        "amount_minor": "500",
        "idempotency_key": "idem_phase4_issue",
        "memo": "phase4 dispute boundary issue smoke"
    })
}

fn clean_transfer_request() -> serde_json::Value {
    json!({
        "from": "acct_phase4_bond_alice",
        "to": "acct_phase4_bond_treasury",
        "asset": "roc",
        "amount_minor": "500",
        "nonce": 1,
        "idempotency_key": "idem_phase4_transfer",
        "memo": "phase4 dispute boundary transfer smoke"
    })
}

fn clean_burn_request() -> serde_json::Value {
    json!({
        "from": "acct_phase4_bond_alice",
        "asset": "roc",
        "amount_minor": "1",
        "nonce": 1,
        "idempotency_key": "idem_phase4_burn",
        "memo": "phase4 dispute boundary burn smoke"
    })
}

#[test]
fn disputed_bond_review_is_explicit_review_only() {
    let review = valid_review();

    assert_eq!(
        review.schema,
        SVC_WALLET_QUICKCHAIN_BOND_DISPUTE_REVIEW_SCHEMA
    );
    assert_eq!(review.chain_id, "roc-dev");
    assert_eq!(review.dispute_id, "bond-dispute:phase4:r2:alice");
    assert_eq!(review.bond_account_id, "bond-account:phase4:r2:alice");
    assert_eq!(review.actor_account_id, "acct_phase4_bond_alice");
    assert_eq!(review.disputed_amount_minor.get(), 500);
    assert_eq!(review.frozen_minor, "125");
    assert_eq!(
        review.action,
        QuickChainWalletBondDisputeAction::FreezePendingAppeal
    );
    assert_eq!(
        review.status,
        QuickChainWalletBondDisputeReviewStatus::ReviewOnly
    );
    assert!(review.requires_explicit_confirmation);
    assert!(!review.live_wallet_mutation);
    assert!(!review.balance_side_effect);
    assert!(!review.auto_penalty_enabled);
    assert!(!review.finality_claim);

    review
        .validate()
        .expect("valid disputed-bond review should validate");
}

#[test]
fn disputed_bond_review_rejects_authority_flags_and_bad_amount_shape() {
    let mut no_confirm = valid_review();
    no_confirm.requires_explicit_confirmation = false;
    assert!(
        no_confirm.validate().is_err(),
        "disputed-bond review must not allow hidden confirmation"
    );

    let mut live_mutation = valid_review();
    live_mutation.live_wallet_mutation = true;
    assert!(
        live_mutation.validate().is_err(),
        "disputed-bond review must not represent live wallet mutation"
    );

    let mut balance_side_effect = valid_review();
    balance_side_effect.balance_side_effect = true;
    assert!(
        balance_side_effect.validate().is_err(),
        "disputed-bond review must not represent balance side effects"
    );

    let mut automatic_penalty = valid_review();
    automatic_penalty.auto_penalty_enabled = true;
    assert!(
        automatic_penalty.validate().is_err(),
        "disputed-bond review must not enable automatic penalty execution"
    );

    let mut finality_claim = valid_review();
    finality_claim.finality_claim = true;
    assert!(
        finality_claim.validate().is_err(),
        "disputed-bond review must not claim finality"
    );

    let mut wrong_asset = valid_review();
    wrong_asset.asset = "rox".to_owned();
    assert!(
        wrong_asset.validate().is_err(),
        "disputed-bond review must stay ROC-only"
    );

    let mut excessive_frozen = valid_review();
    excessive_frozen.frozen_minor = "501".to_owned();
    assert!(
        excessive_frozen.validate().is_err(),
        "frozen_minor must not exceed disputed amount"
    );

    let mut leading_zero = valid_review();
    leading_zero.frozen_minor = "0125".to_owned();
    assert!(
        leading_zero.validate().is_err(),
        "frozen_minor must be canonical integer minor units"
    );
}

#[test]
fn disputed_bond_review_rejects_unknown_authority_fields() {
    let clean = serde_json::to_value(valid_review()).expect("review should serialize");

    for (field, value) in [
        ("wallet_receipt", json!("tx_fake_dispute")),
        ("ledger_commit", json!(true)),
        ("balance_lock", json!(true)),
        ("freeze_funds", json!(true)),
        ("capture_bond", json!(true)),
        ("execute_slash", json!(true)),
        ("commit_slash_decision", json!(true)),
        ("validator_reward", json!("reward:false")),
        ("public_staking_market", json!(true)),
        ("liquidity_pool", json!("pool:fake")),
        ("bridge_settlement", json!("solana")),
        ("external_settlement", json!("rox")),
        ("settlement_status", json!("finalized")),
        (
            "state_root",
            json!("b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ),
    ] {
        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("review JSON should be object")
            .insert(field.to_owned(), value);

        assert!(
            serde_json::from_value::<QuickChainWalletBondDisputeReview>(poisoned).is_err(),
            "disputed-bond review DTO must reject unknown authority field: {field}"
        );
    }
}

#[test]
fn live_wallet_mutation_requests_reject_phase4_dispute_authority_fields() {
    for field in [
        "dispute_id",
        "challenge_window",
        "appeal_window",
        "frozen_minor",
        "freeze_funds",
        "capture_bond",
        "execute_slash",
        "commit_slash_decision",
        "reject_irreversible_slash",
        "wallet_receipt",
        "ledger_commit",
        "settlement_status",
        "finality_claim",
        "balance_side_effect",
        "auto_penalty_enabled",
        "public_staking_market",
        "liquidity_pool",
        "bridge_settlement",
        "external_settlement",
    ] {
        let mut issue = clean_issue_request();
        issue
            .as_object_mut()
            .expect("issue JSON should be object")
            .insert(field.to_owned(), json!("client-supplied-dispute-authority"));
        assert!(
            serde_json::from_value::<IssueRequest>(issue).is_err(),
            "IssueRequest must reject Phase 4 dispute authority field: {field}"
        );

        let mut transfer = clean_transfer_request();
        transfer
            .as_object_mut()
            .expect("transfer JSON should be object")
            .insert(field.to_owned(), json!("client-supplied-dispute-authority"));
        assert!(
            serde_json::from_value::<TransferRequest>(transfer).is_err(),
            "TransferRequest must reject Phase 4 dispute authority field: {field}"
        );

        let mut burn = clean_burn_request();
        burn.as_object_mut()
            .expect("burn JSON should be object")
            .insert(field.to_owned(), json!("client-supplied-dispute-authority"));
        assert!(
            serde_json::from_value::<BurnRequest>(burn).is_err(),
            "BurnRequest must reject Phase 4 dispute authority field: {field}"
        );
    }
}

#[test]
fn wallet_source_does_not_implement_phase4_dispute_runtime_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-wallet Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path));

        for forbidden in [
            "\"/slash",
            "\"/slashing",
            "\"/stake",
            "\"/staking",
            "\"/bond/dispute/execute",
            "\"/bond/dispute/capture",
            "execute_slash(",
            "commit_slash_decision(",
            "apply_slash(",
            "freeze_wallet_balance(",
            "capture_bond(",
            "create_dispute_receipt(",
            "reject_irreversible_slash(",
            "validator_reward",
            "bridge_settlement",
            "external_settlement",
            "live_wallet_mutation: true",
            "balance_side_effect: true",
            "auto_penalty_enabled: true",
            "finality_claim: true",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-wallet source must not implement Phase 4 Round 2 dispute runtime authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
