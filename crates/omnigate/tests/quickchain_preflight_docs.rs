//! RO:WHAT — Documentation/script alignment tests for omnigate QuickChain Phase-0 preflight.
//! RO:WHY — Boundary docs must remain testable and cannot drift from the focused preflight gate.
//! RO:INTERACTS — docs/quickchain-preflight.md and scripts/dev-quickchain-preflight.sh.
//! RO:INVARIANTS — docs name allowed/forbidden scope, operation identity, paid/cache boundaries.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — keeps future settlement authority out of omnigate.
//! RO:TEST — cargo test -p omnigate --test quickchain_preflight_docs.

use std::{fs, path::PathBuf};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(path: &str) -> String {
    let full = crate_root().join(path);
    fs::read_to_string(&full).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", full.display());
    })
}

#[test]
fn docs_state_omnigate_hydration_and_product_coordination_boundary() {
    let docs = read_rel("docs/quickchain-preflight.md");

    for required in [
        "hydration boundary",
        "product coordination boundary",
        "product hydration and coordination boundary",
        "not become ROC economic truth",
        "no direct ledger mutation",
    ] {
        assert!(
            docs.contains(required),
            "quickchain-preflight.md must contain boundary phrase `{required}`"
        );
    }
}

#[test]
fn docs_name_forbidden_authority_roles() {
    let docs = read_rel("docs/quickchain-preflight.md");

    for required in [
        "not ledger truth",
        "not wallet truth",
        "not receipt truth",
        "not balance truth",
        "not QuickChain runtime",
        "not become a public Layer 1",
        "no roots/checkpoints/validators/bridges",
    ] {
        assert!(
            docs.contains(required),
            "quickchain-preflight.md must name forbidden role `{required}`"
        );
    }

    for required in [
        "root authority",
        "checkpoint authority",
        "finality authority",
        "validator authority",
        "bridge authority",
        "staking authority",
        "liquidity authority",
        "external settlement authority",
    ] {
        assert!(
            docs.contains(required),
            "quickchain-preflight.md must name forbidden authority `{required}`"
        );
    }
}

#[test]
fn docs_preserve_operation_identity_rules() {
    let docs = read_rel("docs/quickchain-preflight.md");

    for required in [
        "operation_id is backend-assigned durable ledger operation identity",
        "idempotency_key is retry identity only",
        "Idempotency-Key is retry identity only",
        "account_sequence is ledger-assigned",
        "hold_id identifies one hold lifecycle",
        "backend receipt is display/validation context",
        "receipt display cache is display-only",
    ] {
        assert!(
            docs.contains(required),
            "quickchain-preflight.md must preserve operation identity rule `{required}`"
        );
    }
}

#[test]
fn docs_define_cache_and_content_addressing_as_non_economic_truth() {
    let docs = read_rel("docs/quickchain-preflight.md");

    for required in [
        "cache cannot unlock paid content",
        "cache is convenience only",
        "b3 hashes are content truth, not economic truth",
        "b3 proves bytes",
        "b3 does not prove payment",
        "manifest hydration does not prove payment",
        "local receipt cache does not prove payment without backend validation",
    ] {
        assert!(
            docs.contains(required),
            "quickchain-preflight.md must define cache/content boundary `{required}`"
        );
    }
}

#[test]
fn preflight_script_lists_focused_tests_and_clippy_gate() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "cargo",
        "fmt -p",
        "quickchain_preflight_boundary",
        "quickchain_preflight_docs",
        "quickchain_preflight_no_fake_receipts",
        "quickchain_preflight_paid_access",
        "quickchain_preflight_cache_boundary",
        "content_view",
        "site_visit",
        "streams",
        "chat_routes",
        "paid_storage_estimate_proxy",
        "paid_storage_prepare",
        "paid_storage_write_proxy",
        "clippy -p",
        "--all-targets",
        "-D warnings",
        "omnigate QuickChain preflight gate passed",
    ] {
        assert!(
            script.contains(required),
            "dev-quickchain-preflight.sh must contain `{required}`"
        );
    }
}
