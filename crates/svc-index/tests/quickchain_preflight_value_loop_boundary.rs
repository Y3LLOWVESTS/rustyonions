//! RO:WHAT — Value-loop boundary tests for svc-index QuickChain Phase-0.
//! RO:WHY — Proves lookup/pointer metadata cannot become paid unlock, receipt, balance, finality, or settlement authority.
//! RO:INTERACTS — `src/`, `src/router.rs`, `Cargo.toml`, `docs/quickchain-preflight.md`.
//! RO:INVARIANTS — svc-index resolves references only; wallet/ledger/backend paths remain economic truth.
//! RO:TEST — `cargo test -p svc-index --test quickchain_preflight_value_loop_boundary`.

use std::{
    fs,
    path::{Path, PathBuf},
};

const FORBIDDEN_DIRECT_DEPENDENCIES: &[&str] = &[
    "svc-wallet",
    "ron-ledger",
    "ron-accounting",
    "svc-rewarder",
    "quickchain-runtime",
    "quickchain-validator",
    "quickchain-consensus",
    "solana",
    "solana-sdk",
    "solana-client",
];

const FORBIDDEN_INDEX_AUTHORITY_ENTRYPOINTS: &[&str] = &[
    "unlock_from_index",
    "receipt_from_index",
    "balance_from_index",
    "finality_from_index",
    "checkpoint_from_index",
    "validator_from_index",
    "root_from_index",
    "settle_from_index",
    "mint_from_index",
    "transfer_from_index",
    "paid_from_index",
    "entitlement_from_index",
    "issue_receipt_from_index",
    "claim_finality_from_index",
    "grant_unlock_from_index",
    "capture_hold_from_index",
    "release_hold_from_index",
];

const FORBIDDEN_ROUTE_FRAGMENTS: &[&str] = &[
    "/quickchain/root",
    "/quickchain/checkpoint",
    "/quickchain/finality",
    "/quickchain/validator",
    "/quickchain/bridge",
    "/settle",
    "/settlement",
    "/mint",
    "/transfer",
    "/receipt/fake",
    "/unlock/from-index",
    "/balance/from-index",
    "/wallet/mutate",
    "/ledger/mutate",
];

#[test]
fn svc_index_has_no_direct_value_plane_or_quickchain_runtime_dependencies() {
    let cargo = read(crate_dir().join("Cargo.toml"));

    for dep in FORBIDDEN_DIRECT_DEPENDENCIES {
        assert!(
            !dependency_declared(&cargo, dep),
            "svc-index must not depend directly on value-plane or QuickChain runtime authority crate/dependency: {dep}"
        );
    }
}

#[test]
fn svc_index_source_has_no_authority_shaped_value_loop_entrypoints() {
    let source = production_source().to_ascii_lowercase();

    for token in FORBIDDEN_INDEX_AUTHORITY_ENTRYPOINTS {
        assert!(
            !source.contains(token),
            "svc-index production source must not define authority-shaped value-loop entrypoint: {token}"
        );
    }
}

#[test]
fn svc_index_router_does_not_expose_paid_unlock_or_settlement_authority_routes() {
    let router = read(crate_dir().join("src").join("router.rs")).to_ascii_lowercase();

    for fragment in FORBIDDEN_ROUTE_FRAGMENTS {
        assert!(
            !router.contains(fragment),
            "svc-index router must not expose forbidden value-plane/QuickChain authority route fragment: {fragment}"
        );
    }
}

#[test]
fn docs_record_backend_value_loop_without_delegating_authority_to_index() {
    let docs = read(crate_dir().join("docs").join("quickchain-preflight.md")).to_ascii_lowercase();

    for phrase in [
        "svc-index is a lookup and pointer service",
        "svc-wallet mutation or lookup path",
        "ron-ledger durable receipt truth",
        "svc-storage / gateway paid enforcement",
        "index entry → unlock",
        "manifest lookup → unlock",
        "cache hit → unlock",
        "client says paid → unlock",
        "metadata is allowed; authority is not",
    ] {
        assert!(
            docs.contains(phrase),
            "svc-index QuickChain preflight docs must preserve value-loop boundary phrase: {phrase}"
        );
    }
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn production_source() -> String {
    let mut files = Vec::new();
    collect_rust_files(&crate_dir().join("src"), &mut files);
    files.sort();

    let mut combined = String::new();
    for file in files {
        combined.push_str(&read(&file));
        combined.push('\n');
    }
    combined
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    if !root.exists() {
        return;
    }

    let entries =
        fs::read_dir(root).unwrap_or_else(|err| panic!("read_dir {}: {err}", root.display()));

    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("read_dir entry under {}: {err}", root.display()))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn dependency_declared(cargo_toml: &str, dep: &str) -> bool {
    let quoted = format!("\"{dep}\"");
    let bare_eq = format!("{dep} =");
    let quoted_eq = format!("\"{dep}\" =");

    cargo_toml.lines().any(|line| {
        let line = line.trim();
        !line.starts_with('#')
            && (line.starts_with(&bare_eq)
                || line.starts_with(&quoted_eq)
                || line.starts_with(&quoted))
    })
}
