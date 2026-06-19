#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Boundary tests proving ron-ledger QuickChain projection stays pre-canonical and pre-root.
//! RO:WHY — ECON/GOV: ledger may execute/replay/project economic truth, but canonical bytes, hash preimages, roots, validators, and settlement remain separate gates.
//! RO:INTERACTS — crates/ron-ledger/src/quickchain source files and ron-proto QuickChain DTO contracts.
//! RO:INVARIANTS — no serde/JSON canonicalization, BLAKE3, preimage construction, wall clocks, runtime modules, checkpoints, validators, anchors, bridges, or settlement.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — regression tripwire only; does not create roots, proofs, receipts, validators, anchors, or spend authority.
//! RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_canonical_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

const BANNED_PRE_CANONICAL_TOKENS: &[(&str, &str)] = &[
    (
        "serde_json",
        "ron-ledger QuickChain pre-root code must not serialize canonical JSON",
    ),
    (
        "serde::serialize",
        "ron-ledger QuickChain pre-root code must not become a wire-format crate",
    ),
    (
        "serde::deserialize",
        "ron-ledger QuickChain pre-root code must not become a wire-format crate",
    ),
    (
        "#[derive(serialize",
        "ron-ledger QuickChain pre-root structs must not become canonical wire DTOs",
    ),
    (
        "#[derive(deserialize",
        "ron-ledger QuickChain pre-root structs must not become canonical wire DTOs",
    ),
    (
        "to_canonical_json",
        "canonical JSON belongs in ron-proto/vector gates, not ledger execution/projection",
    ),
    (
        "from_canonical_json",
        "canonical JSON belongs in ron-proto/vector gates, not ledger execution/projection",
    ),
    (
        "quickchaincanonical",
        "canonical JSON belongs in ron-proto/vector gates, not ledger execution/projection",
    ),
    (
        "domain_separator",
        "domain-separated preimage construction is not authorized in ledger pre-root code",
    ),
    (
        "preimage",
        "hash preimage construction is not authorized in ledger pre-root code",
    ),
    (
        "blake3::",
        "ledger pre-root code must not compute hashes over ledger state",
    ),
    ("sha2::", "alternate hash algorithms are not authorized"),
    ("sha3::", "alternate hash algorithms are not authorized"),
    (
        "std::time::systemtime",
        "ledger pre-root code must not read wall-clock time",
    ),
    (
        "systemtime",
        "ledger pre-root code must not read wall-clock time",
    ),
    (
        "unix_epoch",
        "ledger pre-root code must not read wall-clock time",
    ),
    (
        "tokio::",
        "ledger pre-root code must not depend on async runtime behavior",
    ),
    (
        ".await",
        "ledger pre-root code must not depend on async runtime behavior",
    ),
    (".spawn(", "ledger pre-root code must not spawn tasks"),
    (
        "reqwest::",
        "ledger pre-root code must not call HTTP clients",
    ),
    (
        "axum::",
        "ledger pre-root code must not expose service endpoints",
    ),
    (
        "hyper::",
        "ledger pre-root code must not expose or call service endpoints",
    ),
    (
        "std::fs",
        "ledger pre-root source must not perform filesystem IO",
    ),
    (
        "std::net",
        "ledger pre-root source must not perform network IO",
    ),
    (
        "sled::",
        "ledger pre-root source must not depend on database iteration order",
    ),
    (
        "rusqlite::",
        "ledger pre-root source must not depend on database iteration order",
    ),
    (
        "sqlx::",
        "ledger pre-root source must not depend on database iteration order",
    ),
    (
        "rocksdb::",
        "ledger pre-root source must not depend on database iteration order",
    ),
    (
        "redb::",
        "ledger pre-root source must not depend on database iteration order",
    ),
    (
        "rand::",
        "ledger pre-root source must not depend on randomness",
    ),
    (
        "thread_rng",
        "ledger pre-root source must not depend on randomness",
    ),
    ("unsafe", "ledger pre-root source must remain safe Rust"),
];

const BANNED_RUNTIME_MODULE_STEMS: &[(&str, &str)] = &[
    (
        "root",
        "root-producing modules remain blocked until canonical bytes and locked hash vectors authorize them",
    ),
    (
        "checkpoint",
        "checkpoint/finality modules are not part of the pre-root ledger slice",
    ),
    (
        "validator",
        "validator modules are not part of the pre-root ledger slice",
    ),
    (
        "consensus",
        "consensus modules are not part of the pre-root ledger slice",
    ),
    (
        "settlement",
        "settlement modules are not part of the pre-root ledger slice",
    ),
    (
        "anchor",
        "anchor modules are not part of the pre-root ledger slice",
    ),
    (
        "bridge",
        "bridge modules are not part of the pre-root ledger slice",
    ),
    (
        "staking",
        "staking modules are not part of the pre-root ledger slice",
    ),
    (
        "pruning",
        "pruning modules are not part of the pre-root ledger slice",
    ),
    (
        "finality",
        "finality modules are not part of the pre-root ledger slice",
    ),
    (
        "signer",
        "signing modules are not part of the pre-root ledger slice",
    ),
    (
        "signature",
        "signature modules are not part of the pre-root ledger slice",
    ),
];

#[test]
fn quickchain_pre_root_ledger_sources_do_not_serialize_hash_or_build_preimages() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/quickchain");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    files.sort();

    assert!(
        files.len() >= 10,
        "expected the current ron-ledger QuickChain preflight module set to be present"
    );

    for path in files {
        let rel = relative_key(Path::new(env!("CARGO_MANIFEST_DIR")), &path);
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {rel}: {error}"));
        let code_only = strip_comments_and_literals(&raw).to_ascii_lowercase();

        for (token, reason) in BANNED_PRE_CANONICAL_TOKENS {
            assert!(
                !code_only.contains(token),
                "{rel}: found forbidden pre-canonical token `{token}` after stripping comments/literals: {reason}"
            );
        }
    }
}

#[test]
fn quickchain_pre_root_module_tree_does_not_claim_runtime_phase_authority() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/quickchain");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    files.sort();

    for path in files {
        let rel = relative_key(Path::new(env!("CARGO_MANIFEST_DIR")), &path);
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        for (banned_stem, reason) in BANNED_RUNTIME_MODULE_STEMS {
            assert_ne!(
                stem, *banned_stem,
                "{rel}: forbidden QuickChain runtime-phase module stem `{banned_stem}`: {reason}"
            );
        }
    }
}

fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("failed to read directory entry: {error}"))
                .path()
        })
        .collect();

    entries.sort();

    for path in entries {
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn relative_key(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Remove comments and ordinary string/char literal bodies before scanning.
///
/// This is deliberately a conservative regression tripwire, not a Rust parser.
fn strip_comments_and_literals(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                if i < bytes.len() {
                    out.push('\n');
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < bytes.len() {
                    if bytes[i] == b'\n' {
                        out.push('\n');
                    }

                    if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                        i += 2;
                        break;
                    }

                    i += 1;
                }
            }
            b'"' => {
                out.push('"');
                i += 1;

                while i < bytes.len() {
                    match bytes[i] {
                        b'\\' if i + 1 < bytes.len() => {
                            out.push(' ');
                            out.push(' ');
                            i += 2;
                        }
                        b'"' => {
                            out.push('"');
                            i += 1;
                            break;
                        }
                        b'\n' => {
                            out.push('\n');
                            i += 1;
                        }
                        _ => {
                            out.push(' ');
                            i += 1;
                        }
                    }
                }
            }
            b'\'' => {
                out.push('\'');
                i += 1;

                while i < bytes.len() {
                    match bytes[i] {
                        b'\\' if i + 1 < bytes.len() => {
                            out.push(' ');
                            out.push(' ');
                            i += 2;
                        }
                        b'\'' => {
                            out.push('\'');
                            i += 1;
                            break;
                        }
                        b'\n' => {
                            out.push('\n');
                            i += 1;
                            break;
                        }
                        _ => {
                            out.push(' ');
                            i += 1;
                        }
                    }
                }
            }
            byte => {
                out.push(byte as char);
                i += 1;
            }
        }
    }

    out
}
