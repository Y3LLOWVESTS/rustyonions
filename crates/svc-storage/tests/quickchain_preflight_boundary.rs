//! RO:WHAT — Static QuickChain boundary tests for svc-storage HTTP/API surface.
//! RO:WHY — QuickChain Phase 0 allows guardrails, not roots/checkpoints/validators in storage.
//! RO:INTERACTS — http::server route table, policy modules, docs/quickchain-preflight.md.
//! RO:INVARIANTS — storage exposes bytes/ranges/paid-write admission only; no chain/bridge/anchor routes.
//! RO:METRICS — none; this is a compile-time/source-boundary preflight suite.
//! RO:CONFIG — reads checked-in source only.
//! RO:SECURITY — prevents accidental authority creep into public storage routes.
//! RO:TEST — cargo test -p svc-storage --test quickchain_preflight_boundary.

use std::{fs, path::PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    fs::read_to_string(crate_dir().join(relative)).unwrap_or_else(|err| {
        panic!("failed to read {relative}: {err}");
    })
}

fn compact(source: &str) -> String {
    source.chars().filter(|ch| !ch.is_whitespace()).collect()
}

#[test]
fn router_exposes_storage_routes_not_quickchain_authority_routes() {
    let server = read("src/http/server.rs");
    let compact_server = compact(&server);

    for expected in [
        ".route(\"/o\",",
        ".route(\"/o/:cid\",",
        ".route(\"/paid/o/estimate\",",
        ".route(\"/paid/o\",",
        ".route(\"/healthz\",",
        ".route(\"/readyz\",",
        ".route(\"/version\",",
    ] {
        assert!(
            compact_server.contains(expected),
            "svc-storage should keep expected byte/object route mounted: {expected}"
        );
    }

    for forbidden in [
        "/quickchain",
        "/checkpoint",
        "/checkpoints",
        "/validator",
        "/validators",
        "/bridge",
        "/anchor",
        "/anchors",
        "/staking",
        "/liquidity",
        "/rox",
        "/solana",
        "/ledger",
        "/wallet/issue",
        "/wallet/transfer",
        "/wallet/burn",
        "/wallet/hold",
        "/wallet/capture",
        "/wallet/release",
    ] {
        assert!(
            !server.contains(forbidden),
            "svc-storage router must not expose QuickChain/economic-authority route {forbidden}"
        );
    }
}

#[test]
fn public_response_shapes_do_not_claim_balance_receipt_root_or_finality_truth() {
    let route_sources = [
        "src/http/routes/put_object.rs",
        "src/http/routes/post_object.rs",
        "src/http/routes/get_object.rs",
        "src/http/routes/head_object.rs",
        "src/http/routes/paid_estimate.rs",
        "src/http/routes/paid_object.rs",
    ];

    let combined = route_sources
        .iter()
        .map(|path| read(path))
        .collect::<Vec<_>>()
        .join("\n--- route boundary ---\n");

    for forbidden in [
        "balance_minor",
        "available_minor",
        "held_minor",
        "state_root",
        "receipt_root",
        "checkpoint_hash",
        "validator_signature",
        "finalized",
        "anchored",
        "bridge_settled",
    ] {
        assert!(
            !combined.contains(forbidden),
            "storage route responses must not claim economic/finality/root truth via `{forbidden}`"
        );
    }

    assert!(
        combined.contains("cid"),
        "storage routes should speak in content identifiers, not ledger authority"
    );
}
