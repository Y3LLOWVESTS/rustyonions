#![cfg(feature = "quickchain-preflight")]
#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 4 Round 3 controlled bond enforcement confirmation boundary tests for svc-wallet.
//! RO:WHY — Wallet-side enforcement confirmation must be explicit and internal-only;
//! it must not become silent spend, fake receipt creation, live route authority,
//! public staking, liquidity, bridge, or external settlement behavior.
//! RO:INTERACTS — svc_wallet::quickchain enforcement confirmation helper and strict v1 request DTOs.
//! RO:INVARIANTS — confirmation-only; backend wallet/ledger path required;
//! no wallet receipt, no local balance side effect, no automatic penalty.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — prevents Phase 4 Round 3 enforcement from leaking wallet authority.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase4_bond_enforcement_confirmation_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::json;
use svc_wallet::{
    dto::requests::{BurnRequest, IssueRequest, TransferRequest},
    quickchain::{
        QuickChainWalletBondEnforcementAction, QuickChainWalletBondEnforcementConfirmation,
        QuickChainWalletBondEnforcementConfirmationStatus,
        SVC_WALLET_QUICKCHAIN_BOND_ENFORCEMENT_CONFIRMATION_SCHEMA,
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

fn valid_confirmation() -> QuickChainWalletBondEnforcementConfirmation {
    QuickChainWalletBondEnforcementConfirmation::confirmed_internal_only(
        "roc-dev",
        "bond-enforcement:phase4:r3:reserve",
        "bond-account:phase4:r3:alice",
        "acct_phase4_bond_alice",
        "operator:alice",
        50,
        "idem_phase4_bond_enforcement_confirm",
        QuickChainWalletBondEnforcementAction::ReserveSlash,
    )
    .expect("valid internal-only enforcement confirmation should build")
}

fn clean_issue_request() -> serde_json::Value {
    json!({
        "to": "acct_phase4_bond_alice",
        "asset": "roc",
        "amount_minor": "50",
        "idempotency_key": "idem_phase4_issue",
        "memo": "phase4 bond enforcement boundary issue smoke"
    })
}

fn clean_transfer_request() -> serde_json::Value {
    json!({
        "from": "acct_phase4_bond_alice",
        "to": "acct_phase4_bond_treasury",
        "asset": "roc",
        "amount_minor": "50",
        "nonce": 1,
        "idempotency_key": "idem_phase4_transfer",
        "memo": "phase4 bond enforcement boundary transfer smoke"
    })
}

fn clean_burn_request() -> serde_json::Value {
    json!({
        "from": "acct_phase4_bond_alice",
        "asset": "roc",
        "amount_minor": "1",
        "nonce": 1,
        "idempotency_key": "idem_phase4_burn",
        "memo": "phase4 bond enforcement boundary burn smoke"
    })
}

#[test]
fn enforcement_confirmation_is_explicit_internal_only_and_not_wallet_receipt() {
    let confirmation = valid_confirmation();

    assert_eq!(
        confirmation.schema,
        SVC_WALLET_QUICKCHAIN_BOND_ENFORCEMENT_CONFIRMATION_SCHEMA
    );
    assert_eq!(confirmation.chain_id, "roc-dev");
    assert_eq!(
        confirmation.enforcement_id,
        "bond-enforcement:phase4:r3:reserve"
    );
    assert_eq!(confirmation.bond_account_id, "bond-account:phase4:r3:alice");
    assert_eq!(confirmation.actor_account_id, "acct_phase4_bond_alice");
    assert_eq!(confirmation.amount_minor.get(), 50);
    assert_eq!(
        confirmation.action,
        QuickChainWalletBondEnforcementAction::ReserveSlash
    );
    assert_eq!(
        confirmation.status,
        QuickChainWalletBondEnforcementConfirmationStatus::ConfirmedInternalOnly
    );
    assert!(confirmation.explicit_confirmation);
    assert!(confirmation.backend_ledger_path_required);
    assert!(!confirmation.live_wallet_mutation);
    assert!(!confirmation.wallet_receipt_created);
    assert!(!confirmation.balance_side_effect);
    assert!(!confirmation.auto_penalty_enabled);
    assert!(!confirmation.public_market);
    assert!(!confirmation.liquidity_enabled);
    assert!(!confirmation.outside_settlement);

    confirmation
        .validate()
        .expect("valid confirmation should validate");

    let encoded = serde_json::to_string(&confirmation).expect("confirmation should serialize");
    assert!(
        encoded.contains(r#""schema":"svc-wallet.quickchain-bond-enforcement-confirmation.v1""#)
    );
    assert!(encoded.contains(r#""explicit_confirmation":true"#));
    assert!(encoded.contains(r#""backend_ledger_path_required":true"#));
    assert!(encoded.contains(r#""wallet_receipt_created":false"#));
}

#[test]
fn enforcement_confirmation_rejects_hidden_or_external_authority_flags() {
    let mut no_confirm = valid_confirmation();
    no_confirm.explicit_confirmation = false;
    assert!(
        no_confirm.validate().is_err(),
        "confirmation must not be implicit"
    );

    let mut no_backend_path = valid_confirmation();
    no_backend_path.backend_ledger_path_required = false;
    assert!(
        no_backend_path.validate().is_err(),
        "confirmation must require backend wallet/ledger path"
    );

    let mut live_mutation = valid_confirmation();
    live_mutation.live_wallet_mutation = true;
    assert!(
        live_mutation.validate().is_err(),
        "confirmation artifact must not itself be a live wallet mutation"
    );

    let mut receipt = valid_confirmation();
    receipt.wallet_receipt_created = true;
    assert!(
        receipt.validate().is_err(),
        "confirmation artifact must not create a wallet receipt"
    );

    let mut balance = valid_confirmation();
    balance.balance_side_effect = true;
    assert!(
        balance.validate().is_err(),
        "confirmation artifact must not directly affect balances"
    );

    let mut auto_penalty = valid_confirmation();
    auto_penalty.auto_penalty_enabled = true;
    assert!(
        auto_penalty.validate().is_err(),
        "confirmation must not enable automatic penalties"
    );

    let mut public_market = valid_confirmation();
    public_market.public_market = true;
    assert!(
        public_market.validate().is_err(),
        "confirmation must not enable public staking market behavior"
    );

    let mut liquidity = valid_confirmation();
    liquidity.liquidity_enabled = true;
    assert!(
        liquidity.validate().is_err(),
        "confirmation must not enable liquidity behavior"
    );

    let mut external = valid_confirmation();
    external.outside_settlement = true;
    assert!(
        external.validate().is_err(),
        "confirmation must not enable external settlement"
    );
}

#[test]
fn enforcement_confirmation_rejects_unknown_fields_and_bad_identity() {
    let mut value = serde_json::to_value(valid_confirmation()).expect("valid json");
    value["settlement_finality"] = json!("finalized");
    assert!(
        serde_json::from_value::<QuickChainWalletBondEnforcementConfirmation>(value).is_err(),
        "unknown finality field must reject"
    );

    let mut bad_chain = valid_confirmation();
    bad_chain.chain_id = "roc dev".to_owned();
    assert!(bad_chain.validate().is_err());

    let mut bad_enforcement = valid_confirmation();
    bad_enforcement.enforcement_id = "bond enforcement with spaces".to_owned();
    assert!(bad_enforcement.validate().is_err());

    let mut bad_account = valid_confirmation();
    bad_account.actor_account_id = String::new();
    assert!(bad_account.validate().is_err());

    let mut bad_operator = valid_confirmation();
    bad_operator.operator_subject = String::new();
    assert!(bad_operator.validate().is_err());

    assert!(
        QuickChainWalletBondEnforcementConfirmation::confirmed_internal_only(
            "roc-dev",
            "bond-enforcement:phase4:r3:zero",
            "bond-account:phase4:r3:alice",
            "acct_phase4_bond_alice",
            "operator:alice",
            0,
            "idem_phase4_bond_enforcement_zero",
            QuickChainWalletBondEnforcementAction::ReleaseSlashReserve,
        )
        .is_err(),
        "zero amount must reject"
    );
}

#[test]
fn ordinary_wallet_requests_cannot_smuggle_enforcement_authority() {
    for field in [
        "bond_enforcement",
        "enforcement_id",
        "bond_account_id",
        "operator_subject",
        "explicit_confirmation",
        "backend_ledger_path_required",
        "wallet_receipt_created",
        "balance_side_effect",
        "auto_penalty_enabled",
        "public_market",
        "liquidity_enabled",
        "outside_settlement",
        "settlement_finality",
        "bridge_settlement",
    ] {
        let mut issue = clean_issue_request();
        issue[field] = json!(true);
        assert!(
            serde_json::from_value::<IssueRequest>(issue).is_err(),
            "issue request must reject smuggled enforcement field: {field}"
        );

        let mut transfer = clean_transfer_request();
        transfer[field] = json!(true);
        assert!(
            serde_json::from_value::<TransferRequest>(transfer).is_err(),
            "transfer request must reject smuggled enforcement field: {field}"
        );

        let mut burn = clean_burn_request();
        burn[field] = json!(true);
        assert!(
            serde_json::from_value::<BurnRequest>(burn).is_err(),
            "burn request must reject smuggled enforcement field: {field}"
        );
    }
}

#[test]
fn wallet_routes_do_not_expose_public_bond_enforcement_surface() {
    let routes_mod = strip_line_comments(&read(crate_dir().join("src/routes/v1/mod.rs")));
    let routes_dir = crate_dir().join("src/routes/v1");

    for forbidden_route in [
        "\"/bond-enforcement",
        "\"/bond_enforcement",
        "\"/slash",
        "\"/slashing",
        "\"/stake",
        "\"/staking",
        "\"/liquidity",
        "\"/bridge",
        "\"/external-settlement",
        "\"/settlement-finality",
    ] {
        assert!(
            !routes_mod.contains(forbidden_route),
            "svc-wallet route module must not expose public enforcement route: {forbidden_route}"
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
            "QuickChainWalletBondEnforcementConfirmation",
            "wallet_receipt_created",
            "backend_ledger_path_required",
            "bond_enforcement",
            "public_market",
            "liquidity_enabled",
            "outside_settlement",
        ] {
            assert!(
                !code.contains(forbidden),
                "{route} must not become a hidden bond enforcement authority path via {forbidden}"
            );
        }
    }
}

#[test]
fn wallet_source_does_not_construct_public_staking_or_bridge_enforcement_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-wallet Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "public_staking_market",
            "liquidity_pool",
            "exchange_facing",
            "bridge_settlement",
            "solana",
            "rox",
            "external_chain",
            "validator_reward",
            "mint_from_enforcement",
            "issue_from_enforcement",
            "settle_from_enforcement",
            "finalize_from_enforcement",
            "anchor_from_enforcement",
            "wallet_receipt_created: true",
            "balance_side_effect: true",
            "auto_penalty_enabled: true",
            "public_market: true",
            "liquidity_enabled: true",
            "outside_settlement: true",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-wallet source must not construct public/external enforcement authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
