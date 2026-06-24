//! RO:WHAT — QuickChain Phase-0 documentation boundary tests for ron-accounting.
//! RO:WHY — Accounting must be documented as derivative metering/snapshot infrastructure, not balance truth.
//! RO:INTERACTS — crates/ron-accounting/docs/quickchain-preflight.md and crate-local preflight/parking scripts.
//! RO:INVARIANTS — no wallet/ledger mutation, no roots, no settlement, no validators, no fake balances/receipts.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — prevents docs/tooling drift toward accounting authority or root-producing behavior.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_preflight_docs.

use std::{
    fs,
    path::{Path, PathBuf},
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

fn runbook() -> String {
    read(crate_dir().join("docs").join("quickchain-preflight.md"))
}

fn assert_contains(text: &str, needle: &str) {
    assert!(
        text.contains(needle),
        "QuickChain accounting runbook must contain required marker: {needle}"
    );
}

fn assert_contains_case_insensitive(text: &str, needle: &str) {
    let lower_text = text.to_lowercase();
    let lower_needle = needle.to_lowercase();
    assert!(
        lower_text.contains(&lower_needle),
        "QuickChain accounting runbook must contain required marker, ignoring case: {needle}"
    );
}

#[test]
fn quickchain_runbook_exists_and_declares_accounting_non_authority() {
    let text = runbook();

    for required in [
        "Accounting is not balance truth",
        "`ron-accounting` is not a chain",
        "`ron-accounting` is not a ledger",
        "`ron-accounting` is not a wallet",
        "`ron-accounting` is not a root producer",
        "`ron-accounting` is not a settlement service",
        "usage / metering input",
        "derivative reward snapshot artifacts",
        "svc-wallet",
        "ron-ledger",
    ] {
        assert_contains(&text, required);
    }
}

#[test]
fn quickchain_runbook_preserves_no_wallet_or_ledger_mutation_boundary() {
    let text = runbook();

    for required in [
        "must not perform or claim",
        "mutate wallet state",
        "mutate ledger state",
        "issue wallet receipts",
        "invent fake receipts",
        "invent fake balances",
        "unlock paid content",
        "no fake balances",
        "no fake receipts",
        "no silent spend",
        "no wallet mutation",
        "no ledger mutation",
        "no payout execution",
    ] {
        assert_contains(&text, required);
    }
}

#[test]
fn quickchain_runbook_preserves_forbidden_quickchain_scope_markers() {
    let text = runbook();

    for required in [
        "no roots",
        "no receipt roots",
        "no account state roots",
        "no accounting_root",
        "no reward_root",
        "no checkpoints",
        "no validators",
        "no settlement",
        "no anchors",
        "no external anchors",
        "no bridges",
        "no staking",
        "no liquidity",
        "no Solana/ROX/external settlement path",
        "No DB-order roots",
        "No wall-clock roots",
        "No placeholder hashes",
        "No fake hashes",
    ] {
        assert_contains(&text, required);
    }
}

#[test]
fn quickchain_runbook_documents_artifact_cids_are_not_roots() {
    let text = runbook();

    for required in [
        "Artifact CID vs QuickChain root",
        "A reward snapshot CID is an artifact hash",
        "A canonical snapshot CID is an artifact CID",
        "These are not allowed in Phase 0 runtime output",
        "accounting_root",
        "reward_root",
        "state_root",
        "receipt_root",
        "checkpoint_root",
        "Artifact CIDs help prove exact bytes",
        "They do not make accounting a QuickChain root producer",
    ] {
        assert_contains(&text, required);
    }
}

#[test]
fn quickchain_runbook_documents_event_classes_and_raw_engagement_boundary() {
    let text = runbook();

    for required in [
        "Event-class doctrine",
        "economic_receipt",
        "metering",
        "proof_eligible",
        "ad_budgeted",
        "analytics_only",
        "Raw engagement must never directly mint, allocate, transfer, or mutate protocol ROC",
        "views",
        "likes",
        "impressions",
        "clicks",
        "watch time",
        "raw site visits",
    ] {
        assert_contains(&text, required);
    }
}

#[test]
fn quickchain_runbook_documents_rewarder_handoff_as_planning_only() {
    let text = runbook();

    for required in [
        "Reward projection is planning input",
        "Handoff to svc-rewarder",
        "Handoff to `svc-rewarder` is not wallet execution",
        "feed svc-rewarder",
        "must not:",
        "execute payout",
        "issue payout receipt",
        "mutate wallet",
        "mutate ledger",
        "claim balance truth",
        "claim settlement",
        "claim finality",
        "produce QuickChain roots",
    ] {
        assert_contains(&text, required);
    }
}

#[test]
fn quickchain_scripts_keep_docs_and_dynamic_discovery_in_the_gate() {
    let preflight = read(
        crate_dir()
            .join("scripts")
            .join("dev-quickchain-preflight.sh"),
    );
    let park = read(crate_dir().join("scripts").join("dev-quickchain-park.sh"));

    for required in [
        "find \"$CRATE_DIR/tests\"",
        "-name 'quickchain*.rs'",
        "quickchain_count",
        "quickchain_preflight_docs",
        "expected at least 13 ron-accounting QuickChain test targets",
        "ron-accounting quickchain exhaustive preflight gate passed: tests=",
    ] {
        assert_contains(&preflight, required);
    }

    for required in [
        "crates/ron-accounting/scripts/dev-quickchain-preflight.sh",
        "test -p ron-accounting --test quickchain_preflight_docs",
        "Accounting is not balance truth",
        "Handoff to svc-rewarder",
        "no roots",
        "no checkpoints",
        "no validators",
        "no settlement",
        "no fake balances",
        "no fake receipts",
        "no wallet mutation",
        "no ledger mutation",
        "ron-accounting QuickChain parking gate passed",
    ] {
        assert_contains(&park, required);
    }

    assert_contains_case_insensitive(&park, "workspace check");
}
