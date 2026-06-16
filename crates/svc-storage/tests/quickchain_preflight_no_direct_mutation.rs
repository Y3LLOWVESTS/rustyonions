//! RO:WHAT — Source/dependency tests proving svc-storage is not a direct wallet/ledger mutation authority.
//! RO:WHY — QuickChain Phase 0 keeps svc-wallet as mutation front-door and ron-ledger as durable truth.
//! RO:INTERACTS — Cargo.toml, src/http/server.rs, src/policy/settlement.rs, src/accounting/exporter.rs.
//! RO:INVARIANTS — no production ron-ledger/svc-wallet dependency; no storage-exposed wallet mutation routes.
//! RO:METRICS — none.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — wallet HTTP settlement seam is allowed; direct ledger mutation is not.
//! RO:TEST — cargo test -p svc-storage --test quickchain_preflight_no_direct_mutation.

use std::{fs, path::PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    fs::read_to_string(crate_dir().join(relative)).unwrap_or_else(|err| {
        panic!("failed to read {relative}: {err}");
    })
}

fn cargo_section<'a>(manifest: &'a str, section: &str) -> &'a str {
    let marker = format!("[{section}]");
    let start = manifest
        .find(&marker)
        .unwrap_or_else(|| panic!("missing Cargo.toml section [{section}]"));
    let after = &manifest[start + marker.len()..];
    let end = after.find("\n[").unwrap_or(after.len());
    &after[..end]
}

#[test]
fn production_dependencies_do_not_include_wallet_or_ledger_mutation_crates() {
    let manifest = read("Cargo.toml");
    let dependencies = cargo_section(&manifest, "dependencies");
    let dev_dependencies = cargo_section(&manifest, "dev-dependencies");

    for forbidden in ["ron-ledger", "svc-wallet"] {
        assert!(
            !dependencies.contains(forbidden),
            "svc-storage production dependencies must not include direct mutation crate {forbidden}"
        );
    }

    assert!(
        dev_dependencies.contains("svc-wallet"),
        "svc-wallet may remain a dev-dependency for in-process paid-storage smoke tests"
    );
}

#[test]
fn storage_router_does_not_expose_wallet_or_ledger_mutation_endpoints() {
    let server = read("src/http/server.rs");

    for forbidden in [
        "/v1/issue",
        "/v1/transfer",
        "/v1/burn",
        "/v1/hold",
        "/v1/capture",
        "/v1/release",
        "/ledger",
        "/balance",
    ] {
        assert!(
            !server.contains(forbidden),
            "svc-storage router must not expose wallet/ledger mutation endpoint {forbidden}"
        );
    }
}

#[test]
fn wallet_capture_release_appear_only_inside_explicit_settlement_adapter() {
    let settlement = read("src/policy/settlement.rs");
    assert!(
        settlement.contains("/v1/capture") && settlement.contains("/v1/release"),
        "paid-storage wallet settlement seam should remain explicit and reviewable"
    );

    let route_sources = [
        "src/http/server.rs",
        "src/http/routes/put_object.rs",
        "src/http/routes/post_object.rs",
        "src/http/routes/get_object.rs",
        "src/http/routes/head_object.rs",
        "src/http/routes/paid_estimate.rs",
        "src/http/routes/paid_object.rs",
    ];

    for path in route_sources {
        let source = read(path);
        for forbidden in ["/v1/capture", "/v1/release", "Ledger", "ron_ledger"] {
            assert!(
                !source.contains(forbidden),
                "{path} must not contain direct wallet/ledger mutation marker {forbidden}"
            );
        }
    }
}

#[test]
fn accounting_export_is_metering_not_balance_truth() {
    let accounting_mod = read("src/accounting/mod.rs");
    let accounting_exporter = read("src/accounting/exporter.rs");
    let combined = format!("{accounting_mod}\n{accounting_exporter}");

    for required in ["usage", "metering", "no balances", "no ledger mutation"] {
        assert!(
            combined.to_lowercase().contains(required),
            "accounting export docs should keep storage usage as metering only: missing {required}"
        );
    }

    for forbidden in [
        "balance_minor",
        "available_minor",
        "finalized",
        "checkpoint_hash",
    ] {
        assert!(
            !combined.contains(forbidden),
            "accounting export must not claim economic/finality truth via {forbidden}"
        );
    }
}
