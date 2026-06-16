//! RO:WHAT — QuickChain Phase-0 boundary tests for omnigate.
//! RO:WHY — Omnigate hydrates and coordinates product flows but must not become chain/runtime authority.
//! RO:INTERACTS — omnigate src/routes, Cargo.toml, QuickChain preflight docs.
//! RO:INVARIANTS — no direct ron-ledger import; no root/checkpoint/validator/bridge route authority; wallet path is front-door only.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — prevents direct ledger/runtime authority from creeping into omnigate.
//! RO:TEST — cargo test -p omnigate --test quickchain_preflight_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(path: &str) -> String {
    let full = crate_root().join(path);
    fs::read_to_string(&full).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", full.display());
    })
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|err| {
        panic!("failed to read directory {}: {err}", root.display());
    });

    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read directory entry: {err}"))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn production_sources() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);
    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            (path, text)
        })
        .collect()
}

#[test]
fn omnigate_cargo_does_not_depend_on_ron_ledger_or_quickchain_runtime_crates() {
    let cargo = read_rel("Cargo.toml");

    let forbidden_dependency_markers = [
        "ron-ledger =",
        "ron_ledger =",
        "quickchain =",
        "svc-bridge =",
        "svc-validator =",
    ];

    for marker in forbidden_dependency_markers {
        assert!(
            !cargo.contains(marker),
            "omnigate Cargo.toml must not depend on direct ledger/runtime authority marker `{marker}`"
        );
    }

    assert!(
        cargo.contains("ron-proto"),
        "omnigate may keep normal DTO dependency surface through ron-proto"
    );
}

#[test]
fn omnigate_source_does_not_import_direct_ledger_or_chain_authority() {
    let forbidden_source_markers = [
        "use ron_ledger",
        "ron_ledger::",
        "extern crate ron_ledger",
        "use quickchain",
        "quickchain::",
        "CheckpointProducer",
        "ValidatorSet",
        "BridgeSettlement",
        "StateRootProducer",
        "ReceiptRootProducer",
    ];

    for (path, source) in production_sources() {
        for marker in forbidden_source_markers {
            assert!(
                !source.contains(marker),
                "{} must not import or construct direct ledger/chain authority marker `{marker}`",
                path.display()
            );
        }
    }
}

#[test]
fn public_v1_router_does_not_publish_quickchain_authority_routes() {
    let route_sources = ["src/routes/v1/mod.rs", "src/routes/mod.rs", "src/lib.rs"];

    let forbidden_route_fragments = [
        "\"/quickchain",
        "\"/checkpoint",
        "\"/checkpoints",
        "\"/state-root",
        "\"/receipt-root",
        "\"/accounting-root",
        "\"/reward-root",
        "\"/validator",
        "\"/validators",
        "\"/bridge",
        "\"/bridges",
        "\"/staking",
        "\"/liquidity",
        "\"/settlement",
        "\"/external-settlement",
    ];

    for rel in route_sources {
        let source = read_rel(rel);
        for line in source.lines() {
            let routeish = line.contains(".route(") || line.contains(".nest(");
            if !routeish {
                continue;
            }

            for fragment in forbidden_route_fragments {
                assert!(
                    !line.contains(fragment),
                    "{rel} must not expose QuickChain authority route fragment `{fragment}` in line `{line}`"
                );
            }
        }
    }
}

#[test]
fn wallet_surface_is_display_balance_and_explicit_hold_front_door_only() {
    let route_mod = read_rel("src/routes/v1/mod.rs");
    let wallet = read_rel("src/routes/v1/wallet.rs");

    assert!(
        route_mod.contains("\"/wallet/:account/balance\""),
        "wallet display balance route should remain explicit"
    );
    assert!(
        route_mod.contains("\"/wallet/hold\""),
        "wallet hold route should remain explicit"
    );

    assert!(
        wallet.contains("DEFAULT_WALLET_BASE_URL"),
        "wallet route must use configured svc-wallet front-door base URL"
    );
    assert!(
        wallet.contains("/v1/balance"),
        "wallet balance display must be delegated to svc-wallet"
    );
    assert!(
        wallet.contains("/v1/hold"),
        "wallet hold must be delegated to svc-wallet"
    );

    for forbidden in [
        "ron_ledger::",
        "use ron_ledger",
        "ledger.commit",
        "ledger_mutate",
    ] {
        assert!(
            !wallet.contains(forbidden),
            "wallet façade must not mutate ron-ledger directly via `{forbidden}`"
        );
    }
}

#[test]
fn route_docs_keep_omnigate_as_hydrator_and_coordinator_not_chain_runtime() {
    let v1_mod = read_rel("src/routes/v1/mod.rs");

    assert!(
        v1_mod.contains("no ledger mutation here"),
        "v1 route aggregator must keep the no-ledger-mutation invariant visible"
    );
    assert!(
        v1_mod.contains("wallet mutations are proxied only through svc-wallet"),
        "v1 route aggregator must preserve svc-wallet as the mutation front-door"
    );
}
