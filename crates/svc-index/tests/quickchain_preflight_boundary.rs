//! RO:WHAT — Source/dependency boundary tests for svc-index QuickChain Phase-0.
//! RO:WHY — Prove svc-index does not import wallet/ledger authority or implement root/checkpoint/validator/settlement machinery.
//! RO:INTERACTS — Cargo.toml and production source under src/.
//! RO:INVARIANTS — svc-index may own pointer truth only; it must not mutate ROC or produce QuickChain authority artifacts.
//! RO:TEST — run with `cargo test -p svc-index --test quickchain_preflight_boundary`.

use std::{
    fs,
    path::{Path, PathBuf},
};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn collect_rust_files(root: &Path, files: &mut Vec<PathBuf>) {
    if !root.exists() {
        return;
    }

    let entries = fs::read_dir(root)
        .unwrap_or_else(|err| panic!("failed to read directory {}: {err}", root.display()));

    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read directory entry: {err}"))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "rs")
        {
            files.push(path);
        }
    }
}

fn strip_rust_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut idx = 0;
    let mut in_block = false;

    while idx < bytes.len() {
        if in_block {
            if idx + 1 < bytes.len() && bytes[idx] == b'*' && bytes[idx + 1] == b'/' {
                in_block = false;
                idx += 2;
            } else {
                if bytes[idx] == b'\n' {
                    out.push('\n');
                }
                idx += 1;
            }
            continue;
        }

        if idx + 1 < bytes.len() && bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
            in_block = true;
            idx += 2;
            continue;
        }

        if idx + 1 < bytes.len() && bytes[idx] == b'/' && bytes[idx + 1] == b'/' {
            while idx < bytes.len() && bytes[idx] != b'\n' {
                idx += 1;
            }
            if idx < bytes.len() {
                out.push('\n');
                idx += 1;
            }
            continue;
        }

        out.push(bytes[idx] as char);
        idx += 1;
    }

    out
}

fn production_sources_without_comments() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    collect_rust_files(&manifest_dir().join("src"), &mut files);

    files
        .into_iter()
        .map(|path| {
            let body = strip_rust_comments(&read(&path)).to_lowercase();
            (path, body)
        })
        .collect()
}

fn assert_no_token(label: &str, text: &str, forbidden: &[&str]) {
    for token in forbidden {
        assert!(
            !text.contains(token),
            "{label} must not contain forbidden QuickChain/value-plane authority token {token:?}"
        );
    }
}

#[test]
fn cargo_does_not_depend_on_wallet_or_ledger_authority_crates() {
    let cargo = read(manifest_dir().join("Cargo.toml")).to_lowercase();

    assert_no_token(
        "svc-index Cargo.toml",
        &cargo,
        &[
            "ron-ledger",
            "svc-wallet",
            "ron-accounting",
            "svc-rewarder",
            "quickchain-runtime",
            "quickchain-validator",
            "quickchain-consensus",
            "solana",
        ],
    );
}

#[test]
fn production_source_does_not_import_value_plane_authority_crates() {
    for (path, source) in production_sources_without_comments() {
        assert_no_token(
            &path.display().to_string(),
            &source,
            &[
                "ron_ledger",
                "svc_wallet",
                "ron_accounting",
                "svc_rewarder",
                "quickchain_runtime",
                "quickchain_validator",
                "quickchain_consensus",
            ],
        );
    }
}

#[test]
fn production_source_does_not_call_wallet_mutation_verbs() {
    let forbidden_call_shapes = [
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
    ];

    for (path, source) in production_sources_without_comments() {
        assert_no_token(&path.display().to_string(), &source, &forbidden_call_shapes);
    }
}

#[test]
fn production_source_does_not_define_quickchain_runtime_infrastructure() {
    let forbidden_authority_tokens = [
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

    for (path, source) in production_sources_without_comments() {
        assert_no_token(
            &path.display().to_string(),
            &source,
            &forbidden_authority_tokens,
        );
    }
}
