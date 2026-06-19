//! RO:WHAT — Pair-level QuickChain value-loop boundary tests for svc-storage.
//! RO:WHY — Locks storage as bytes/b3 + metering input before accounting/rewarder/wallet/ledger authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, policy::{economics,paid_write,settlement}, accounting exporter, paid /paid/o route.
//! RO:INVARIANTS — b3 is byte truth only; cache is not paid authority; metering feeds accounting; wallet/ledger remain truth.
//! RO:METRICS — none; source/docs boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents storage from becoming receipt, balance, root, finality, bridge, or payout authority.
//! RO:TEST — cargo test -p svc-storage --test quickchain_preflight_value_loop_boundary.

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
fn docs_lock_storage_position_in_internal_value_loop() {
    let doc = normalized(&read("docs/quickchain-preflight.md"));

    for required in [
        "svc-storage paid admission and b3 byte integrity",
        "storage/access metering",
        "ron-accounting derivative snapshots",
        "svc-rewarder deterministic payout planning",
        "explicit approved payout intent",
        "svc-wallet",
        "ron-ledger",
        "b3 hashes identify bytes only",
        "cache must not decide paid access by itself",
        "storage metering is derivative accounting input only",
    ] {
        assert!(
            doc.contains(required),
            "quickchain-preflight.md must lock storage value-loop phrase: {required}"
        );
    }
}

#[test]
fn pricing_and_metering_sources_are_side_effect_free_planning_inputs() {
    let economics = read("src/policy/economics.rs");
    let accounting = read("src/accounting/mod.rs");
    let exporter = read("src/accounting/exporter.rs");

    for required in [
        "PaidStoragePriceEstimate",
        "side-effect free",
        "does not call wallet, ledger",
        "UsageEventDto",
        "usage only; no balances; no ledger mutation",
        "export failure never mutates ledger or wallet state",
        "no wallet receipt/body bytes exported",
    ] {
        let haystack = format!("{economics}\n{accounting}\n{exporter}");
        assert!(
            haystack.contains(required),
            "storage pricing/metering sources must preserve non-authority marker: {required}"
        );
    }
}

#[test]
fn paid_write_accepts_wallet_hold_evidence_without_becoming_receipt_or_ledger_truth() {
    let paid_write = read("src/policy/paid_write.rs");

    for required in [
        "WalletReceipt",
        "validate_as_paid_write_hold",
        "PaidWriteProof",
        "paid_storage_context_idem",
        "wallet receipt hash must be b3:<64 lowercase hex>",
        "paid proof must reference a wallet hold receipt",
    ] {
        assert!(
            paid_write.contains(required),
            "paid-write verifier must preserve wallet-hold evidence marker: {required}"
        );
    }

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
            !paid_write.contains(forbidden),
            "paid-write verifier must not accept chain/finality authority field: {forbidden}"
        );
    }
}

#[test]
fn settlement_source_targets_wallet_capture_release_without_direct_chain_authority() {
    let settlement = read("src/policy/settlement.rs");

    for required in [
        "SETTLEMENT_MODE_WALLET_CAPTURE",
        "PaidStorageSettlementPlan",
        "WalletSettlementHttpClient",
        "capture_idem",
        "release_idem",
        "failed_write_release_idem",
    ] {
        assert!(
            settlement.contains(required),
            "storage settlement source must preserve explicit wallet-settlement marker: {required}"
        );
    }

    for forbidden in [
        "ron_ledger::",
        "LedgerClient",
        "ledger_commit",
        "quickchain_root",
        "state_root",
        "receipt_root",
        "checkpoint_hash",
        "validator_signature",
        "validator_set",
        "external_anchor",
        "bridge_txid",
        "staking",
        "liquidity",
    ] {
        assert!(
            !settlement.contains(forbidden),
            "storage settlement source must not import direct ledger/chain authority: {forbidden}"
        );
    }
}

#[test]
fn selected_storage_value_loop_sources_do_not_gain_quickchain_runtime_authority() {
    let src = read_sources(&[
        "src/policy/economics.rs",
        "src/policy/paid_write.rs",
        "src/policy/settlement.rs",
        "src/accounting/mod.rs",
        "src/accounting/exporter.rs",
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
            "storage value-loop sources must not grow QuickChain runtime authority field: {forbidden}"
        );
    }
}
