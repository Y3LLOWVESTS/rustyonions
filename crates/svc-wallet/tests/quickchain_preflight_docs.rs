//! RO:WHAT — QuickChain Phase-0 documentation boundary tests for svc-wallet.
//! RO:WHY — svc-wallet must document its preflight posture before being parked.
//! RO:INTERACTS — crates/svc-wallet/docs/quickchain-preflight.md and parking script.
//! RO:INVARIANTS — wallet mutation front-door only; no roots/checkpoints/validators/settlement/anchors/bridges.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — prevents docs drift toward external settlement or chain authority.
//! RO:TEST — cargo test -p svc-wallet --test quickchain_preflight_docs.

use std::{fs, path::PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl Into<PathBuf>) -> String {
    let path = path.into();
    fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
}

#[test]
fn quickchain_runbook_exists_and_states_wallet_truth_boundary() {
    let text = read(crate_dir().join("docs").join("quickchain-preflight.md"));

    for required in [
        "svc-wallet is the ROC wallet mutation front-door",
        "QuickChain is future settlement infrastructure",
        "ron-ledger remains economic truth",
        "wallet receipts are backend-derived",
        "no fake balances",
        "no fake receipts",
        "no silent spend",
    ] {
        assert!(
            text.contains(required),
            "runbook must preserve wallet truth boundary phrase: {required}"
        );
    }
}

#[test]
fn quickchain_runbook_preserves_forbidden_scope_markers() {
    let text = read(crate_dir().join("docs").join("quickchain-preflight.md"));

    for forbidden_marker in [
        "no roots",
        "no receipt roots",
        "no account state roots",
        "no checkpoints",
        "no validators",
        "no settlement",
        "no anchors",
        "no external anchors",
        "no bridges",
        "no staking",
        "no liquidity",
        "no pruning",
        "no public-chain authority",
        "no Solana/ROX/external settlement path",
    ] {
        assert!(
            text.contains(forbidden_marker),
            "runbook must preserve forbidden scope marker: {forbidden_marker}"
        );
    }
}

#[test]
fn quickchain_runbook_documents_idempotency_and_operation_identity_split() {
    let text = read(crate_dir().join("docs").join("quickchain-preflight.md"));

    for required in [
        "idempotency_key = retry key only",
        "operation_id = backend-assigned durable ledger-operation identity",
        "account_sequence = ledger-assigned sequence identity",
        "hold_id = one hold lifecycle identity",
    ] {
        assert!(
            text.contains(required),
            "runbook must document identity split: {required}"
        );
    }
}

#[test]
fn parking_gate_checks_docs_before_declaring_success() {
    let script = read(crate_dir().join("scripts").join("dev-quickchain-park.sh"));

    for required in [
        "crates/svc-wallet/scripts/dev-quickchain-preflight.sh",
        "crates/svc-wallet/docs/quickchain-preflight.md",
        "svc-wallet is the ROC wallet mutation front-door",
        "QuickChain is future settlement infrastructure",
        "no roots",
        "no checkpoints",
        "no validators",
        "no settlement",
        "no external anchors",
        "no bridges",
        "svc-wallet QuickChain parking gate passed",
    ] {
        assert!(
            script.contains(required),
            "parking gate must check or print required marker: {required}"
        );
    }
}
