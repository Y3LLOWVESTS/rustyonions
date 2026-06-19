//! RO:WHAT — Value-loop boundary tests for ron-policy QuickChain Phase-0.
//! RO:WHY — Proves policy decisions/configuration cannot become wallet/ledger mutation, payout allocation, paid proof, or finality.
//! RO:INTERACTS — `src/`, `Cargo.toml`, `docs/quickchain-preflight.md`.
//! RO:INVARIANTS — policy declares/evaluates only; svc-wallet/ron-ledger remain economic truth.
//! RO:TEST — `cargo test -p ron-policy --test quickchain_preflight_value_loop_boundary`.

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

const FORBIDDEN_POLICY_AUTHORITY_ENTRYPOINTS: &[&str] = &[
    "receipt_from_policy",
    "balance_from_policy",
    "unlock_from_policy",
    "finality_from_policy",
    "settle_from_policy",
    "root_from_policy",
    "checkpoint_from_policy",
    "validator_from_policy",
    "mint_from_policy",
    "transfer_from_policy",
    "capture_from_policy",
    "release_from_policy",
    "reward_from_policy",
    "issue_receipt_from_policy",
    "grant_unlock_from_policy",
    "claim_finality_from_policy",
    "allocate_protocol_roc_from_policy",
    "execute_payout_from_policy",
];

#[test]
fn ron_policy_has_no_direct_value_plane_or_quickchain_runtime_dependencies() {
    let cargo = read(crate_dir().join("Cargo.toml"));

    for dep in FORBIDDEN_DIRECT_DEPENDENCIES {
        assert!(
            !dependency_declared(&cargo, dep),
            "ron-policy must not depend directly on value-plane mutation or QuickChain runtime authority crate/dependency: {dep}"
        );
    }
}

#[test]
fn ron_policy_source_has_no_authority_shaped_value_loop_entrypoints() {
    let source = production_source().to_ascii_lowercase();

    for token in FORBIDDEN_POLICY_AUTHORITY_ENTRYPOINTS {
        assert!(
            !source.contains(token),
            "ron-policy production source must not define authority-shaped value-loop entrypoint: {token}"
        );
    }
}

#[test]
fn docs_record_policy_as_declarative_without_delegating_backend_truth() {
    let docs = read(crate_dir().join("docs").join("quickchain-preflight.md"));

    for phrase in [
        "ron-policy is declarative policy infrastructure",
        "policy decision is not economic truth",
        "policy allow is not paid proof",
        "policy obligation is not receipt proof",
        "policy explanation is not finality proof",
        "economics policy config is not ledger mutation",
        "feature flag is not settlement authority",
        "Policy must not manufacture paid proof",
        "Policy must not manufacture receipt proof",
        "Policy must not manufacture finality proof",
        "Policy must not manufacture balance proof",
    ] {
        assert!(
            docs.contains(phrase),
            "ron-policy QuickChain preflight docs must preserve value-loop boundary phrase: {phrase}"
        );
    }
}

#[test]
fn docs_reject_transport_header_claims_as_authority() {
    let docs = read(crate_dir().join("docs").join("quickchain-preflight.md")).to_ascii_lowercase();

    for phrase in [
        "x-ron-paid: true as authority",
        "x-ron-receipt: fake as authority",
        "x-ron-balance: fake as authority",
        "x-ron-finalized: true as authority",
        "x-quickchain-root as authority",
        "x-quickchain-checkpoint as authority",
        "x-quickchain-validator as authority",
        "policy is transport-agnostic",
    ] {
        assert!(
            docs.contains(phrase),
            "ron-policy docs must preserve transport/header non-authority phrase: {phrase}"
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
