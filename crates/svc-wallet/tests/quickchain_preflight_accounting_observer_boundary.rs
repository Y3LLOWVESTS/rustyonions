//! RO:WHAT — QuickChain preflight tests for the svc-wallet → accounting observer seam.
//! RO:WHY — svc-wallet may emit derivative accounting observations, but wallet/ledger remain economic authority.
//! RO:INTERACTS — accounting::client, wallet mutation routes, wallet receipts.
//! RO:INVARIANTS — accounting observation happens after backend receipt memory; no roots/finality/balance authority.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — prevents accounting side-effects from becoming spend, receipt, balance, or QuickChain authority.
//! RO:TEST — cargo test -p svc-wallet --test quickchain_preflight_accounting_observer_boundary.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use svc_wallet::{
    accounting::client::{AccountingEvent, NoopAccountingClient},
    dto::responses::WalletOp,
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

fn route_v1_dir() -> PathBuf {
    crate_dir().join("src").join("routes").join("v1")
}

fn route_files_with_accounting_record() -> BTreeSet<String> {
    let mut files = BTreeSet::new();

    for entry in fs::read_dir(route_v1_dir()).expect("route v1 directory should be readable") {
        let entry = entry.expect("route directory entry should be readable");
        let path = entry.path();

        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        let text = read(&path);
        if text.contains("state.accounting.record(") {
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("route file should have UTF-8 name")
                .to_string();
            files.insert(file_name);
        }
    }

    files
}

#[test]
fn wallet_accounting_event_is_derivative_observation_shape_only() {
    let event = AccountingEvent {
        op: WalletOp::Transfer.as_str(),
        asset: "roc".to_string(),
        amount_minor: 42,
    };

    assert_eq!(event.op, "transfer");
    assert_eq!(event.asset, "roc");
    assert_eq!(event.amount_minor, 42);

    let client = NoopAccountingClient;
    client.record(event.clone());

    assert_eq!(
        event.op,
        WalletOp::Transfer.as_str(),
        "recording an accounting observation must not rewrite wallet operation identity"
    );
    assert_eq!(
        event.amount_minor, 42,
        "recording an accounting observation must not mutate amount truth"
    );
}

#[test]
fn accounting_client_source_does_not_define_quickchain_or_receipt_authority_fields() {
    let text = read(crate_dir().join("src").join("accounting").join("client.rs"));

    for required in [
        "No-op accounting client",
        "derivative counters only",
        "never replaces ron-ledger truth",
    ] {
        assert!(
            text.contains(required),
            "accounting seam must preserve observer-only wording: {required}"
        );
    }

    for forbidden in [
        "operation_id",
        "account_sequence",
        "settlement_status",
        "finality",
        "finalized",
        "state_root",
        "receipt_root",
        "accounting_root",
        "reward_root",
        "checkpoint",
        "anchor",
        "validator",
        "bridge",
        "staking",
        "liquidity",
        "receipt_hash",
        "balance_minor",
        "available_balance",
        "spendable_balance",
    ] {
        assert!(
            !text.contains(forbidden),
            "svc-wallet accounting observer seam must not carry authority field: {forbidden}"
        );
    }
}

#[test]
fn accounting_observations_are_emitted_only_by_wallet_mutation_routes() {
    let expected = BTreeSet::from([
        "burn.rs".to_string(),
        "escrow.rs".to_string(),
        "issue.rs".to_string(),
        "transfer.rs".to_string(),
    ]);

    assert_eq!(
        route_files_with_accounting_record(),
        expected,
        "only accepted wallet mutation routes may emit derivative accounting observations"
    );
}

#[test]
fn accounting_observation_happens_after_backend_receipt_memory() {
    for route in ["issue.rs", "transfer.rs", "burn.rs", "escrow.rs"] {
        let path = route_v1_dir().join(route);
        let text = read(&path);

        let remember_pos = text
            .find("state.remember_receipt(receipt.clone());")
            .unwrap_or_else(|| panic!("{route} must remember backend-derived wallet receipt"));

        let record_pos = text
            .find("state.accounting.record(")
            .unwrap_or_else(|| panic!("{route} must emit accounting observation"));

        assert!(
            remember_pos < record_pos,
            "{route} must emit accounting observation only after backend receipt is remembered"
        );
    }
}

#[test]
fn read_only_wallet_routes_do_not_emit_accounting_observations() {
    for route in ["balance.rs", "receipt.rs"] {
        let path = route_v1_dir().join(route);
        let text = read(&path);

        assert!(
            !text.contains("state.accounting.record("),
            "{route} must remain read-only and must not emit accounting observations"
        );
    }
}
