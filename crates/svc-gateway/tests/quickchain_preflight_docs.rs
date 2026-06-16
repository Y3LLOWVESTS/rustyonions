#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — `QuickChain` Phase-0 documentation boundary tests for svc-gateway.
//! RO:WHY — P6/P12; concerns: GOV/DX/SEC. The crate must carry its own boundary notes.
//! RO:INTERACTS — `docs/quickchain-preflight.md` and `scripts/dev-quickchain-preflight.sh`.
//! RO:INVARIANTS — docs must say gateway is proxy-only and must name forbidden authority creep.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — prevents future patches from erasing Phase-0 gateway doctrine.
//! RO:TEST — `cargo test -p svc-gateway --test quickchain_preflight_docs`.

use std::{fs, path::Path};

fn doc() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("quickchain-preflight.md");

    fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "quickchain preflight doc must exist at {}: {err}",
            path.display()
        )
    })
}

#[test]
fn docs_state_gateway_is_proxy_boundary_not_chain_runtime() {
    let doc = doc().to_ascii_lowercase();

    for required in [
        "public route surface",
        "proxy to omnigate",
        "not a chain runtime",
        "not a validator",
        "not a bridge",
        "not an anchor writer",
        "not a checkpoint writer",
        "not a root producer",
        "not a wallet mutation authority",
        "not a ledger mutation authority",
        "not finality authority",
    ] {
        assert!(
            doc.contains(required),
            "quickchain preflight doc must contain boundary phrase: {required}"
        );
    }
}

#[test]
fn docs_name_allowed_and_forbidden_phase_zero_scope() {
    let doc = doc().to_ascii_lowercase();

    for required in [
        "route exposure",
        "header filtering",
        "proxying to omnigate",
        "proxying raw object reads to svc-storage",
        "wallet hold route proxying to omnigate only",
        "no direct svc-wallet mutation implementation",
        "no direct ron-ledger mutation implementation",
        "no fake balances",
        "no fake receipts",
        "no local entitlement truth",
        "no root-producing code",
        "no checkpoint-producing code",
        "no validator code",
        "no bridge or external settlement code",
        "no finality claims",
    ] {
        assert!(
            doc.contains(required),
            "quickchain preflight doc must preserve scope phrase: {required}"
        );
    }
}

#[test]
fn docs_define_header_boundary_and_operation_identity_rule() {
    let doc = doc().to_ascii_lowercase();

    for required in [
        "x-ron-operation-id",
        "x-ron-account-sequence",
        "x-ron-state-root",
        "x-ron-receipt-root",
        "x-ron-checkpoint-hash",
        "x-ron-validator-*",
        "x-ron-bridge-*",
        "x-ron-quickchain-*",
        "operation_id is backend-assigned",
        "idempotency-key is retry identity only",
        "account_sequence is ledger-assigned",
        "gateway cannot decide paid access from caller-supplied claims alone",
    ] {
        assert!(
            doc.contains(required),
            "quickchain preflight doc must preserve header/identity phrase: {required}"
        );
    }
}

#[test]
fn docs_list_focused_preflight_and_proxy_suites() {
    let doc = doc();

    for suite in [
        "quickchain_preflight_boundary",
        "quickchain_preflight_docs",
        "product_routes_proxy",
        "paid_storage_estimate_proxy",
        "paid_storage_write_proxy",
        "app_proxy",
        "`scripts/dev-quickchain-preflight.sh`",
    ] {
        assert!(
            doc.contains(suite),
            "quickchain preflight doc must list focused suite or runner: {suite}"
        );
    }
}

#[test]
fn docs_keep_future_quickchain_work_parked_outside_gateway() {
    let doc = doc().to_ascii_lowercase();

    for parked in [
        "canonical bytes and locked vectors",
        "state/account merkle roots",
        "receipt roots",
        "checkpoint signing",
        "validator-set logic",
        "external da",
        "public anchors",
        "bridges",
        "staking or liquidity",
        "crablink chain authority",
        "gateway/omnigate ledger mutation",
    ] {
        assert!(
            doc.contains(parked),
            "quickchain preflight doc must keep future work parked: {parked}"
        );
    }
}
