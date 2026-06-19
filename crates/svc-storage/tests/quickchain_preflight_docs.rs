//! RO:WHAT — Documentation gate for svc-storage QuickChain Phase-0 preflight.
//! RO:WHY — The crate needs explicit written boundaries before gateway/omnigate paid enforcement work continues.
//! RO:INTERACTS — docs/quickchain-preflight.md and scripts/dev-quickchain-preflight.sh.
//! RO:INVARIANTS — docs must say storage is bytes/b3 only and not chain, wallet, ledger, bridge, or finality authority.
//! RO:METRICS — none.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — catches accidental removal of no-fake-receipt/no-cache-unlock doctrine.
//! RO:TEST — cargo test -p svc-storage --test quickchain_preflight_docs.

use std::{fs, path::PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    fs::read_to_string(crate_dir().join(relative)).unwrap_or_else(|err| {
        panic!("failed to read {relative}: {err}");
    })
}

fn normalized_markdown(text: &str) -> String {
    text.to_lowercase().replace('`', "")
}

#[test]
fn quickchain_preflight_doc_states_storage_boundary() {
    let doc = read("docs/quickchain-preflight.md");
    let lower = normalized_markdown(&doc);

    for required in [
        "svc-storage remains content-addressed byte/object infrastructure",
        "b3 hashes identify bytes only",
        "svc-wallet = economic mutation front-door",
        "ron-ledger = durable replayable truth",
        "cache must not decide paid access by itself",
        "no fake balances",
        "no fake receipts",
        "no roots",
        "no validators",
        "no bridges",
        "no external settlement",
    ] {
        assert!(
            lower.contains(required),
            "quickchain-preflight.md is missing required boundary phrase: {required}"
        );
    }
}

#[test]
fn quickchain_preflight_doc_has_ro_header_and_complete_test_contract() {
    let doc = read("docs/quickchain-preflight.md");

    for required in [
        "RO:WHAT",
        "RO:WHY",
        "RO:INTERACTS",
        "RO:INVARIANTS",
        "RO:SECURITY",
        "RO:TEST",
        "quickchain_preflight_b3_integrity",
        "quickchain_preflight_boundary",
        "quickchain_preflight_docs",
        "quickchain_preflight_economics_quote",
        "quickchain_preflight_no_direct_mutation",
        "quickchain_preflight_observability",
        "quickchain_preflight_paid_cache",
        "quickchain_preflight_range_media",
        "quickchain_preflight_settlement_boundary",
        "quickchain_tooling_boundary",
    ] {
        assert!(
            doc.contains(required),
            "quickchain-preflight.md is missing required marker: {required}"
        );
    }
}

#[test]
fn dev_preflight_script_runs_dynamic_storage_gate() {
    let script = read("scripts/dev-quickchain-preflight.sh");

    for required in [
        "cargo fmt -p svc-storage -- --check",
        "find crates/svc-storage/tests",
        "-name 'quickchain*.rs'",
        "cargo test -p svc-storage --test \"$test_name\"",
        "cargo test -p svc-storage --all-targets",
        "cargo clippy -p svc-storage --all-targets -- -D warnings",
        "svc-storage quickchain exhaustive preflight gate passed: tests=",
        "no roots; no checkpoints; no validators; no settlement; no anchors; no bridges",
    ] {
        assert!(
            script.contains(required),
            "dev-quickchain-preflight.sh missing required dynamic command/marker: {required}"
        );
    }

    assert!(
        !script.contains("QUICKCHAIN_STATIC_TEST_CONTRACT"),
        "dev-quickchain-preflight.sh should no longer carry the old static compatibility heredoc"
    );

    for forbidden in [
        "cargo test -p svc-storage --test quickchain_preflight_b3_integrity\n",
        "cargo test -p svc-storage --test quickchain_preflight_boundary\n",
        "cargo test -p svc-storage --test quickchain_preflight_docs\n",
        "cargo test -p svc-storage --test quickchain_preflight_economics_quote\n",
        "cargo test -p svc-storage --test quickchain_preflight_no_direct_mutation\n",
        "cargo test -p svc-storage --test quickchain_preflight_observability\n",
        "cargo test -p svc-storage --test quickchain_preflight_paid_cache\n",
        "cargo test -p svc-storage --test quickchain_preflight_range_media\n",
        "cargo test -p svc-storage --test quickchain_preflight_settlement_boundary\n",
        "cargo test -p svc-storage --test quickchain_tooling_boundary\n",
    ] {
        assert!(
            !script.contains(forbidden),
            "dev-quickchain-preflight.sh should use dynamic discovery instead of hardcoded focused target: {forbidden}"
        );
    }
}

#[test]
fn dev_preflight_script_stays_bash_cargo_only() {
    let script = read("scripts/dev-quickchain-preflight.sh");

    for forbidden in ["python ", "python3", "python -", "python3 -"] {
        assert!(
            !script.contains(forbidden),
            "dev-quickchain-preflight.sh must not invoke Python helper tooling: {forbidden}"
        );
    }
}
