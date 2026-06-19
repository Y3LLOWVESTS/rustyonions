//! RO:WHAT — Pair-level QuickChain value-loop boundary tests for svc-rewarder.
//! RO:WHY — Locks rewarder as payout planning only after storage/accounting inputs and before wallet/ledger mutation.
//! RO:INTERACTS — docs/quickchain-preflight.md, outputs::{intents,wallet}, inputs::accounting, core::compute.
//! RO:INVARIANTS — storage/accounting evidence is planning input; rewarder produces wallet intents; wallet/ledger remain mutation/truth.
//! RO:METRICS — none; source/docs boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents rewarder from silently becoming a root, receipt, balance, bridge, or direct-ledger authority.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_preflight_value_loop_boundary.

use std::{fs, path::PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    fs::read_to_string(crate_dir().join(relative)).unwrap_or_else(|err| {
        panic!("failed to read {relative}: {err}");
    })
}

fn normalized(text: &str) -> String {
    text.to_ascii_lowercase().replace('`', "")
}

fn read_sources(paths: &[&str]) -> String {
    paths
        .iter()
        .map(|path| read(path))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn docs_lock_rewarder_position_in_internal_value_loop() {
    let doc = normalized(&read("docs/quickchain-preflight.md"));

    for required in [
        "svc-storage/svc-gateway/omnigate paid enforcement",
        "ron-accounting snapshots",
        "svc-rewarder payout planning",
        "explicit approved payout intent",
        "svc-wallet",
        "ron-ledger",
        "deterministic roc payout planner",
        "svc-wallet is the mutation front-door",
        "ron-ledger is durable economic truth",
    ] {
        assert!(
            doc.contains(required),
            "quickchain-preflight.md must lock pair-level value-loop phrase: {required}"
        );
    }
}

#[test]
fn settlement_intents_are_wallet_handoff_dtos_not_receipts_or_roots() {
    let intents = read("src/outputs/intents.rs");

    for required in [
        "SettlementIntent",
        "SettlementBatch",
        "WalletIssueRequest",
        "WALLET_ISSUE_PATH",
        "to_wallet_issue_request",
        "#[serde(deny_unknown_fields)]",
    ] {
        assert!(
            intents.contains(required),
            "rewarder settlement intent source must keep wallet-handoff marker: {required}"
        );
    }

    for forbidden in [
        "WalletReceipt",
        "receipt_hash",
        "ledger_root",
        "quickchain_root",
        "state_root",
        "receipt_root",
        "checkpoint_hash",
        "validator_signature",
        "finality",
        "external_anchor",
        "bridge_txid",
    ] {
        assert!(
            !intents.contains(forbidden),
            "rewarder settlement intents must not become receipt/root/finality authority: {forbidden}"
        );
    }
}

#[test]
fn wallet_client_boundary_targets_wallet_without_direct_ledger_or_chain_authority() {
    let wallet = read("src/outputs/wallet.rs");

    for required in [
        "svc-wallet",
        "preview_issue_batch",
        "emit_issue_batch",
        "dry_run",
        "WalletHttpIssueOutcome",
    ] {
        assert!(
            wallet.contains(required),
            "rewarder wallet output seam must preserve explicit wallet boundary marker: {required}"
        );
    }

    for forbidden in [
        "ron_ledger::",
        "LedgerClient",
        "ledger_commit",
        "checkpoint_hash",
        "validator_signature",
        "state_root",
        "receipt_root",
        "external_anchor",
        "bridge_txid",
    ] {
        assert!(
            !wallet.contains(forbidden),
            "rewarder wallet output seam must not import direct ledger/chain authority: {forbidden}"
        );
    }
}

#[test]
fn core_rewarder_sources_do_not_gain_quickchain_runtime_authority_fields() {
    let src = read_sources(&[
        "src/core/compute.rs",
        "src/inputs/accounting.rs",
        "src/outputs/manifest.rs",
        "src/outputs/intents.rs",
    ]);

    for forbidden in [
        "quickchain_root",
        "state_root",
        "receipt_root",
        "checkpoint_hash",
        "validator_signature",
        "validator_set",
        "settlement_finality",
        "external_anchor",
        "bridge_txid",
        "staking",
        "liquidity",
    ] {
        assert!(
            !src.contains(forbidden),
            "reward planning sources must not grow QuickChain runtime authority field: {forbidden}"
        );
    }
}
