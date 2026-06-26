#![cfg(feature = "quickchain-preflight")]
#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 4 Round 1 bond confirmation boundary tests for svc-wallet.
//! RO:WHY — Bond-related wallet UX must require explicit review and must not
//! become silent spend, live bond lock, automatic penalty, public market,
//! liquidity, receipt, finality, bridge, or external settlement authority.
//! RO:INTERACTS — svc_wallet::quickchain review helper and strict wallet request DTOs.
//! RO:INVARIANTS — Phase 4 Round 1 is review-only in svc-wallet; no live bond
//! mutation route, fake receipt, or hidden lock is introduced.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — prevents bond/stake/penalty/liquidity authority creep into the wallet.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase4_bond_confirmation_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::json;
use svc_wallet::{
    dto::requests::{BurnRequest, IssueRequest, TransferRequest},
    quickchain::{
        QuickChainWalletBondAction, QuickChainWalletBondReview, QuickChainWalletBondReviewStatus,
        SVC_WALLET_QUICKCHAIN_BOND_REVIEW_SCHEMA,
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

fn valid_review() -> QuickChainWalletBondReview {
    QuickChainWalletBondReview::review_only(
        "roc-dev",
        "bond-intent:phase4:r1:open",
        "bond-account:phase4:r1:alice",
        "acct_phase4_bond_alice",
        "operator:alice",
        500,
        "idem_phase4_bond_review",
        QuickChainWalletBondAction::OpenBond,
    )
    .expect("valid review-only bond artifact should build")
}

fn clean_issue_request() -> serde_json::Value {
    json!({
        "to": "acct_phase4_bond_alice",
        "asset": "roc",
        "amount_minor": "500",
        "idempotency_key": "idem_phase4_issue",
        "memo": "phase4 bond boundary issue smoke"
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
        "memo": "phase4 bond boundary transfer smoke"
    })
}

fn clean_burn_request() -> serde_json::Value {
    json!({
        "from": "acct_phase4_bond_alice",
        "asset": "roc",
        "amount_minor": "1",
        "nonce": 1,
        "idempotency_key": "idem_phase4_burn",
        "memo": "phase4 bond boundary burn smoke"
    })
}

#[test]
fn bond_review_is_explicit_review_only_and_not_live_wallet_mutation() {
    let review = valid_review();

    assert_eq!(review.schema, SVC_WALLET_QUICKCHAIN_BOND_REVIEW_SCHEMA);
    assert_eq!(review.chain_id, "roc-dev");
    assert_eq!(review.intent_id, "bond-intent:phase4:r1:open");
    assert_eq!(review.bond_account_id, "bond-account:phase4:r1:alice");
    assert_eq!(review.actor_account_id, "acct_phase4_bond_alice");
    assert_eq!(review.amount_minor.get(), 500);
    assert_eq!(review.action, QuickChainWalletBondAction::OpenBond);
    assert_eq!(review.status, QuickChainWalletBondReviewStatus::ReviewOnly);
    assert!(review.requires_explicit_confirmation);
    assert!(!review.live_wallet_mutation);
    assert!(!review.auto_penalty_enabled);
    assert!(!review.public_market);
    assert!(!review.liquidity_enabled);

    review
        .validate()
        .expect("valid review-only bond artifact should validate");

    let encoded = serde_json::to_string(&review).expect("review should serialize");
    assert!(encoded.contains(r#""schema":"svc-wallet.quickchain-bond-review.v1""#));
    assert!(encoded.contains(r#""requires_explicit_confirmation":true"#));
    assert!(encoded.contains(r#""live_wallet_mutation":false"#));
}

#[test]
fn bond_review_rejects_silent_or_public_economy_flags() {
    let mut no_confirm = valid_review();
    no_confirm.requires_explicit_confirmation = false;
    assert!(
        no_confirm.validate().is_err(),
        "review must not allow hidden or implicit confirmation"
    );

    let mut live_mutation = valid_review();
    live_mutation.live_wallet_mutation = true;
    assert!(
        live_mutation.validate().is_err(),
        "review must not represent a live wallet mutation"
    );

    let mut automatic_penalty = valid_review();
    automatic_penalty.auto_penalty_enabled = true;
    assert!(
        automatic_penalty.validate().is_err(),
        "review must not enable automatic economic penalties"
    );

    let mut public_market = valid_review();
    public_market.public_market = true;
    assert!(
        public_market.validate().is_err(),
        "review must not enable public market behavior"
    );

    let mut liquidity = valid_review();
    liquidity.liquidity_enabled = true;
    assert!(
        liquidity.validate().is_err(),
        "review must not enable liquidity behavior"
    );

    let mut wrong_asset = valid_review();
    wrong_asset.asset = "rox".to_owned();
    assert!(
        wrong_asset.validate().is_err(),
        "review must remain internal ROC-only"
    );
}

#[test]
fn bond_review_rejects_unknown_authority_fields() {
    let clean = serde_json::to_value(valid_review()).expect("review should serialize");

    for (field, value) in [
        ("wallet_receipt", json!("tx_fake_bond")),
        ("ledger_commit", json!(true)),
        ("balance_lock", json!(true)),
        ("stake_validator", json!(true)),
        ("penalty_evidence", json!("evidence:auto")),
        ("public_staking_market", json!(true)),
        ("liquidity_pool", json!("pool:fake")),
        ("bridge_settlement", json!("solana")),
        ("external_settlement", json!("rox")),
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
            serde_json::from_value::<QuickChainWalletBondReview>(poisoned).is_err(),
            "bond review DTO must reject unknown authority field: {field}"
        );
    }
}

#[test]
fn live_wallet_mutation_requests_reject_phase4_bond_authority_fields() {
    for field in [
        "bond_intent_id",
        "bond_account_id",
        "bond_lifecycle_decision",
        "bond_review_status",
        "requires_explicit_confirmation",
        "live_wallet_mutation",
        "auto_penalty_enabled",
        "public_market",
        "liquidity_enabled",
        "stake_validator",
        "penalty_evidence",
        "public_staking_market",
        "liquidity_pool",
    ] {
        let mut issue = clean_issue_request();
        issue
            .as_object_mut()
            .expect("issue JSON should be object")
            .insert(field.to_owned(), json!("client-supplied-bond-authority"));
        assert!(
            serde_json::from_value::<IssueRequest>(issue).is_err(),
            "IssueRequest must reject Phase 4 bond authority field: {field}"
        );

        let mut transfer = clean_transfer_request();
        transfer
            .as_object_mut()
            .expect("transfer JSON should be object")
            .insert(field.to_owned(), json!("client-supplied-bond-authority"));
        assert!(
            serde_json::from_value::<TransferRequest>(transfer).is_err(),
            "TransferRequest must reject Phase 4 bond authority field: {field}"
        );

        let mut burn = clean_burn_request();
        burn.as_object_mut()
            .expect("burn JSON should be object")
            .insert(field.to_owned(), json!("client-supplied-bond-authority"));
        assert!(
            serde_json::from_value::<BurnRequest>(burn).is_err(),
            "BurnRequest must reject Phase 4 bond authority field: {field}"
        );
    }
}

#[test]
fn wallet_source_does_not_implement_phase4_bond_runtime_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-wallet Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path));

        for forbidden in [
            "execute_bond_review(",
            "commit_bond_review(",
            "apply_bond_review(",
            "create_bond_receipt(",
            "bond_receipt",
            "stake_validator(",
            "penalize_validator(",
            "validator_reward",
            "bridge_settlement",
            "external_settlement",
            "auto_penalty_enabled: true",
            "public_market: true",
            "liquidity_enabled: true",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-wallet source must not implement Phase 4 bond runtime authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
