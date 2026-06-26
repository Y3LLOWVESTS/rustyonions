#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Source-boundary audit for ron-ledger QuickChain pre-root code.
//! RO:WHY — ECON/GOV: ron-ledger may execute and project deterministic state, but must not sneak in roots, validators, clocks, IO, or hashing.
//! RO:INTERACTS — crates/ron-ledger/src/quickchain source files.
//! RO:INVARIANTS — no crypto hash calls, Merkle implementation, network, storage, wall-clock, async runtime, randomness, or validator machinery.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — this is a regression tripwire, not a proof of consensus safety.
//! RO:TEST — this file.

use std::{
    fs,
    path::{Path, PathBuf},
};

const BANNED_CODE_TOKENS: &[(&str, &str)] = &[
    (
        "blake3::",
        "ron-ledger pre-root QuickChain code must not compute hashes over ledger state",
    ),
    (
        "sha2::",
        "ron-ledger pre-root QuickChain code must not introduce alternate hash algorithms",
    ),
    (
        "sha3::",
        "ron-ledger pre-root QuickChain code must not introduce alternate hash algorithms",
    ),
    (
        "ring::",
        "validator/signature/crypto work is not authorized in this pre-root slice",
    ),
    (
        "ed25519",
        "validator/signature work is not authorized in this pre-root slice",
    ),
    (
        "merkle",
        "Merkle/root implementation is blocked until root vectors are independently reproducible",
    ),
    (
        "validator",
        "validator machinery is blocked until roots/proofs are proven",
    ),
    (
        "std::fs",
        "QuickChain preflight helpers must not perform filesystem IO",
    ),
    (
        "std::net",
        "QuickChain preflight helpers must not perform network IO",
    ),
    (
        "tokio::",
        "QuickChain preflight helpers must not spawn or depend on async runtime behavior",
    ),
    (
        "reqwest::",
        "QuickChain preflight helpers must not call HTTP clients",
    ),
    (
        "axum::",
        "QuickChain preflight helpers must not expose service endpoints",
    ),
    (
        "sled::",
        "QuickChain preflight helpers must not depend on database iteration order",
    ),
    (
        "rusqlite::",
        "QuickChain preflight helpers must not depend on database iteration order",
    ),
    (
        "systemtime",
        "QuickChain preflight helpers must not depend on wall-clock time",
    ),
    (
        "unix_epoch",
        "QuickChain preflight helpers must not depend on wall-clock time",
    ),
    (
        "rand::",
        "QuickChain preflight helpers must not depend on randomness during replay/projection",
    ),
    (
        "thread_rng",
        "QuickChain preflight helpers must not depend on randomness during replay/projection",
    ),
    (
        ".spawn(",
        "QuickChain preflight helpers must not spawn tasks",
    ),
];

#[test]
fn quickchain_preflight_sources_have_no_runtime_authority_or_root_production() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/quickchain");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    files.sort();

    assert!(
        files.len() >= 10,
        "expected the current QuickChain preflight source module set to be present"
    );

    for path in files {
        let rel = path
            .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")))
            .unwrap_or(path.as_path())
            .display()
            .to_string();

        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {rel}: {error}"));
        let code_only = strip_comments_and_literals(&raw).to_ascii_lowercase();

        for (token, reason) in BANNED_CODE_TOKENS {
            if is_phase1_round2_root_projection_exception(&rel, token)
                || is_phase3_round1_validator_gate_exception(&rel, token)
            {
                continue;
            }

            assert!(
                !code_only.contains(token),
                "{rel}: found forbidden token `{token}` after stripping comments/literals: {reason}"
            );
        }
    }
}

fn is_phase1_round2_root_projection_exception(rel: &str, token: &str) -> bool {
    rel == "src/quickchain/tree_material_projection.rs" && matches!(token, "blake3::")
}

fn is_phase3_round1_validator_gate_exception(rel: &str, token: &str) -> bool {
    matches!(
        rel,
        "src/quickchain/passport_gate.rs"
            | "src/quickchain/validator_lifecycle.rs"
            | "src/quickchain/bond_accounting.rs"
            | "src/quickchain/bond_dispute.rs"
            | "src/quickchain/mod.rs"
    ) && matches!(token, "validator")
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

/// Remove comments and ordinary string/char literal bodies before scanning.
///
/// The scanner is intentionally conservative: it is a regression tripwire that
/// catches accidental imports/calls in source code. It is not a Rust parser and
/// is not a substitute for security review.
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
