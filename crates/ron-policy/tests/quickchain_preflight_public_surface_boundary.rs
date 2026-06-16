//! RO:WHAT — Public-surface boundary tests for ron-policy QuickChain preflight.
//! RO:WHY — Ensures examples, benches, and fuzz targets do not import wallet/ledger authority or runtime surfaces.
//! RO:INTERACTS — `examples/`, `benches/`, and `fuzz/fuzz_targets/` Rust files.
//! RO:INVARIANTS — non-src surfaces remain policy-only; no mutation verbs; no roots/checkpoints/validators/settlement.

use std::path::{Path, PathBuf};

const SURFACE_ROOTS: &[&str] = &["examples", "benches", "fuzz/fuzz_targets"];

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
fn examples_benches_and_fuzz_targets_remain_policy_only() {
    let files = public_surface_rust_files();
    assert!(
        !files.is_empty(),
        "expected ron-policy examples, benches, or fuzz target files to scan"
    );

    for file in files {
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

        for &token in FORBIDDEN_MUTATION_CALL_SHAPES {
            assert!(
                !source.contains(token),
                "{} contains forbidden mutation call shape: {token}",
                file.display()
            );
        }

        for &token in FORBIDDEN_QUICKCHAIN_RUNTIME_TOKENS {
            assert!(
                !lower_source.contains(token),
                "{} contains forbidden QuickChain runtime/root/checkpoint token: {token}",
                file.display()
            );
        }
    }
}

fn public_surface_rust_files() -> Vec<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();

    for &relative_root in SURFACE_ROOTS {
        let root = manifest_dir.join(relative_root);
        if root.exists() {
            collect_rust_files(&root, &mut files);
        }
    }

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
