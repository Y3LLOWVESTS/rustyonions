#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Pair-level QuickChain value-loop boundary tests for svc-gateway.
//! RO:WHY — Locks the public boundary between client intent, omnigate coordination, wallet mutation, and ledger truth.
//! RO:INTERACTS — docs/quickchain-preflight.md and scripts/dev-quickchain-preflight.sh.
//! RO:INVARIANTS — gateway proxies intent only; wallet/ledger own accepted mutation; accepted is not future finality.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — prevents gateway from becoming receipt, balance, root, checkpoint, validator, bridge, or finality authority.
//! RO:TEST — cargo test -p svc-gateway --test quickchain_preflight_value_loop_boundary.

use std::{fs, path::PathBuf};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(path: &str) -> String {
    let full = crate_root().join(path);
    fs::read_to_string(&full).unwrap_or_else(|err| panic!("read {}: {err}", full.display()))
}

fn assert_contains_all(haystack: &str, label: &str, phrases: &[&str]) {
    for phrase in phrases {
        assert!(
            haystack.contains(phrase),
            "{label} must contain value-loop phrase `{phrase}`"
        );
    }
}

#[test]
fn docs_lock_public_to_product_value_loop_without_authority_transfer() {
    let docs = read_rel("docs/quickchain-preflight.md");

    assert_contains_all(
        &docs,
        "svc-gateway quickchain-preflight.md",
        &[
            "svc-gateway public route boundary -> omnigate product hydration/access coordination -> svc-wallet mutation front-door -> ron-ledger durable economic truth",
            "client intent -> svc-gateway public boundary -> omnigate quote/access/hydration coordinator -> svc-wallet hold/transfer/capture/release/receipt path -> ron-ledger accepted receipt -> paid unlock/render using backend-derived truth",
            "gateway and omnigate may coordinate paid access, but neither is wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, external settlement, or finality authority",
        ],
    );
}

#[test]
fn docs_separate_current_accepted_receipt_from_future_quickchain_status() {
    let docs = read_rel("docs/quickchain-preflight.md");

    assert_contains_all(
        &docs,
        "svc-gateway quickchain-preflight.md",
        &[
            "accepted backend receipt can unlock local paid content",
            "accepted is not finalized",
            "accepted is not epoch_included",
            "accepted is not anchored",
            "future statuses remain parked: accepted, epoch_included, finalized, anchored",
            "current paid unlock is backend-derived local access, not future QuickChain epoch inclusion",
        ],
    );
}

#[test]
fn docs_keep_gateway_out_of_receipt_balance_and_finality_truth() {
    let docs = read_rel("docs/quickchain-preflight.md");

    assert_contains_all(
        &docs,
        "svc-gateway quickchain-preflight.md",
        &[
            "gateway is not receipt truth",
            "gateway is not balance truth",
            "gateway is not settlement finality",
            "gateway never converts accepted wallet/ledger receipt into QuickChain finality",
            "no root-producing code, no checkpoint-producing code, no validator code, no bridge code, no external settlement code",
        ],
    );
}

#[test]
fn preflight_runner_discovers_and_names_value_loop_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    assert_contains_all(
        &script,
        "svc-gateway dev-quickchain-preflight.sh",
        &[
            "quickchain_preflight_value_loop_boundary",
            "find \"$TEST_DIR\"",
            "-name 'quickchain*.rs'",
            "test -p \"$PKG\" --test \"$test_name\"",
        ],
    );
}
