//! RO:WHAT — Source/dependency boundary tests for ron-policy QuickChain preflight.
//! RO:WHY — Proves policy does not import mutation authority crates or root/checkpoint runtime.
//! RO:INTERACTS — Cargo.toml and production Rust source under `src/`.
//! RO:INVARIANTS — policy is declarative; no direct wallet/ledger mutation; no chain runtime.

use std::path::{Path, PathBuf};

const FORBIDDEN_DEPENDENCIES: &[&str] = &[
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

const FORBIDDEN_IMPORT_TOKENS: &[&str] = &[
    "svc_wallet",
    "ron_ledger",
    "ron_accounting",
    "svc_rewarder",
    "quickchain_runtime",
    "quickchain_validator",
    "quickchain_consensus",
    "solana_sdk",
    "solana_client",
];

const FORBIDDEN_MUTATION_CALL_SHAPES: &[&str] = &[
    ".issue(",
    "::issue(",
    ".transfer(",
    "::transfer(",
    ".burn(",
    "::burn(",
    ".hold(",
    "::hold(",
    ".capture(",
    "::capture(",
    ".release(",
    "::release(",
    ".mint(",
    "::mint(",
    "put_receipt(",
    "insert_receipt(",
    "create_receipt(",
    "accept_receipt(",
    "commit_receipt(",
    "mutate_balance(",
    "set_balance(",
    "credit_account(",
    "debit_account(",
];

const FORBIDDEN_QUICKCHAIN_RUNTIME_TOKENS: &[&str] = &[
    "state_root",
    "receipt_root",
    "accounting_root",
    "reward_root",
    "checkpoint_root",
    "checkpoint_hash",
    "checkpoint_header",
    "validator_set",
    "validator_signature",
    "committee_quorum",
    "fork_choice",
    "settlement_status",
    "external_anchor",
    "bridge_settlement",
    "root_producer",
    "produce_root",
    "produce_checkpoint",
    "finalize_checkpoint",
    "quickchain_receipt",
    "quickchain_checkpoint",
    "quickchain_validator",
];

#[test]
fn cargo_toml_does_not_depend_on_mutation_authority_crates() {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", manifest_path.display()));
    let lower_manifest = manifest.to_ascii_lowercase();

    for &dependency in FORBIDDEN_DEPENDENCIES {
        assert!(
            !lower_manifest.contains(dependency),
            "ron-policy Cargo.toml must not depend on mutation/runtime authority crate token: {dependency}"
        );
    }
}

#[test]
fn production_source_does_not_import_mutation_authority_crates() {
    for file in production_rust_files() {
        let source = std::fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("read {}: {err}", file.display()));
        let lower_source = source.to_ascii_lowercase();

        for &token in FORBIDDEN_IMPORT_TOKENS {
            assert!(
                !lower_source.contains(token),
                "{} imports forbidden mutation/runtime authority token: {token}",
                file.display()
            );
        }
    }
}

#[test]
fn production_source_does_not_call_wallet_or_ledger_mutation_verbs() {
    for file in production_rust_files() {
        let source = std::fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("read {}: {err}", file.display()));

        for &token in FORBIDDEN_MUTATION_CALL_SHAPES {
            assert!(
                !source.contains(token),
                "{} contains forbidden mutation call shape: {token}",
                file.display()
            );
        }
    }
}

#[test]
fn production_source_does_not_define_quickchain_runtime_surface() {
    for file in production_rust_files() {
        let source = std::fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("read {}: {err}", file.display()));
        let lower_source = source.to_ascii_lowercase();

        for &token in FORBIDDEN_QUICKCHAIN_RUNTIME_TOKENS {
            assert!(
                !lower_source.contains(token),
                "{} contains forbidden QuickChain runtime/root/checkpoint token: {token}",
                file.display()
            );
        }
    }
}

fn production_rust_files() -> Vec<PathBuf> {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rust_files(&source_root, &mut files);
    files.sort();
    files
}

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("read_dir {}: {err}", dir.display()))
    {
        let entry = entry.unwrap_or_else(|err| panic!("read_dir entry {}: {err}", dir.display()));
        let path = entry.path();

        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}
