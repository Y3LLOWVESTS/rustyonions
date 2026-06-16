//! RO:WHAT — QuickChain Phase-0 documentation boundary tests for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: GOV/DX/SEC. The crate must carry its own boundary notes, not rely on session memory.
//! RO:INTERACTS — docs/quickchain-preflight.md and scripts/dev-quickchain-preflight.sh.
//! RO:INVARIANTS — docs must say rewarder is planning-only and must name forbidden authority creep.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — prevents future patches from silently erasing Phase-0 safety doctrine.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_preflight_docs.

use std::fs;
use std::path::Path;

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
fn docs_state_rewarder_is_planning_only_not_chain_runtime() {
    let doc = doc().to_ascii_lowercase();

    for required in [
        "deterministic roc payout planner",
        "not a chain runtime",
        "not a validator",
        "not a bridge",
        "not a checkpoint writer",
        "not a root producer",
        "not a ledger mutation authority",
        "svc-wallet is the mutation front-door",
        "ron-ledger is durable economic truth",
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
        "strict serde dtos",
        "integer minor-unit money strings only",
        "canonical lowercase",
        "explicit funding provenance",
        "wallet issue request planning",
        "no root-producing code",
        "no checkpoint-producing code",
        "no validator code",
        "no bridge or external settlement code",
        "no direct ledger mutation",
        "no fake balances",
        "no fake receipts",
        "no fake finality",
    ] {
        assert!(
            doc.contains(required),
            "quickchain preflight doc must preserve scope phrase: {required}"
        );
    }
}

#[test]
fn docs_name_raw_engagement_replay_and_funding_boundaries() {
    let doc = doc().to_ascii_lowercase();

    for required in [
        "protocol_pool",
        "governance_budget",
        "require a signed policy",
        "funding provenance is not settlement finality",
        "raw engagement fields must stay rejected",
        "raw views",
        "raw watch seconds",
        "views-to-roc formulas",
        "idempotency keys are replay/dedupe tools",
        "not ledger operation identity",
        "not validator consensus",
    ] {
        assert!(
            doc.contains(required),
            "quickchain preflight doc must preserve economic boundary phrase: {required}"
        );
    }
}

#[test]
fn docs_list_every_focused_preflight_suite() {
    let doc = doc();

    for suite in [
        "quickchain_preflight_boundary",
        "quickchain_preflight_raw_engagement",
        "quickchain_preflight_replay_no_double_issue",
        "quickchain_preflight_funding_source",
        "quickchain_preflight_no_direct_mutation",
        "quickchain_preflight_docs",
    ] {
        assert!(
            doc.contains(suite),
            "quickchain preflight doc must list focused suite: {suite}"
        );
    }
}

#[test]
fn docs_keep_future_quickchain_work_parked_outside_rewarder() {
    let doc = doc().to_ascii_lowercase();

    for parked in [
        "canonical bytes and locked vectors",
        "state/account merkle roots",
        "receipt roots",
        "validator-set logic",
        "checkpoint signing",
        "external da",
        "public anchors",
        "bridges",
        "staking or liquidity",
        "crablink chain authority",
        "gateway/omnigate/rewarder ledger mutation",
    ] {
        assert!(
            doc.contains(parked),
            "quickchain preflight doc must keep future work parked: {parked}"
        );
    }
}
