//! RO:WHAT — Source-boundary regression tests for ron-proto QuickChain DTO/preflight code.
//! RO:WHY — ECON/GOV: ron-proto may define strict DTOs, validation helpers, domains, canonical bytes, and vectors, but not runtime authority.
//! RO:INTERACTS — crates/ron-proto/src/quickchain source files.
//! RO:INVARIANTS — DTO-only; no service IO, async runtime, database, randomness, crypto execution, wallet/ledger mutation, or authority-sensitive defaults.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — regression tripwire only; does not create roots, proofs, receipts, validators, anchors, or spend authority.
//! RO:TEST — cargo test -p ron-proto --test quickchain_source_boundary.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const BANNED_RUNTIME_TOKENS: &[(&str, &str)] = &[
    (
        "blake3::",
        "ron-proto may describe BLAKE3/hash payload contracts, but src/quickchain must not compute hashes",
    ),
    (
        "sha2::",
        "alternate hash algorithms are not part of the QuickChain DTO surface",
    ),
    (
        "sha3::",
        "alternate hash algorithms are not part of the QuickChain DTO surface",
    ),
    (
        "use ring::",
        "signature/validator crypto is not authorized inside ron-proto QuickChain DTO code",
    ),
    (
        "ring::signature",
        "signature/validator crypto is not authorized inside ron-proto QuickChain DTO code",
    ),
    (
        "k256::",
        "public-chain/signature primitives are not authorized inside ron-proto QuickChain DTO code",
    ),
    (
        "secp256k1",
        "public-chain/signature primitives are not authorized inside ron-proto QuickChain DTO code",
    ),
    (
        "tokio::",
        "ron-proto QuickChain must not depend on an async runtime",
    ),
    (
        ".await",
        "ron-proto QuickChain must remain pure DTO/validation code with no async behavior",
    ),
    (
        ".spawn(",
        "ron-proto QuickChain must not spawn tasks",
    ),
    (
        "reqwest::",
        "ron-proto QuickChain must not call HTTP clients",
    ),
    (
        "axum::",
        "ron-proto QuickChain must not expose service endpoints",
    ),
    (
        "hyper::",
        "ron-proto QuickChain must not expose or call service endpoints",
    ),
    (
        "std::fs",
        "ron-proto QuickChain source must not perform filesystem IO",
    ),
    (
        "std::net",
        "ron-proto QuickChain source must not perform network IO",
    ),
    (
        "std::thread",
        "ron-proto QuickChain source must not spawn or depend on threads",
    ),
    (
        "std::time::systemtime",
        "ron-proto QuickChain source must not depend on wall-clock time",
    ),
    (
        "systemtime",
        "ron-proto QuickChain source must not depend on wall-clock time",
    ),
    (
        "unix_epoch",
        "ron-proto QuickChain source must not depend on wall-clock time",
    ),
    (
        "rand::",
        "ron-proto QuickChain source must not depend on randomness",
    ),
    (
        "thread_rng",
        "ron-proto QuickChain source must not depend on randomness",
    ),
    (
        "sled::",
        "ron-proto QuickChain source must not depend on database iteration order",
    ),
    (
        "rusqlite::",
        "ron-proto QuickChain source must not depend on database iteration order",
    ),
    (
        "sqlx::",
        "ron-proto QuickChain source must not depend on database iteration order",
    ),
    (
        "rocksdb::",
        "ron-proto QuickChain source must not depend on database iteration order",
    ),
    (
        "redb::",
        "ron-proto QuickChain source must not depend on database iteration order",
    ),
    (
        "unsafe",
        "ron-proto QuickChain source must remain safe Rust",
    ),
];

const BANNED_DEFAULTING_TOKENS: &[(&str, &str)] = &[
    (
        "derive(default",
        "QuickChain DTOs must not gain implicit default constructors",
    ),
    (
        "impl default",
        "QuickChain DTOs must not gain implicit default constructors",
    ),
    (
        "::default(",
        "QuickChain DTO/validation source must not fill authority-sensitive data by default",
    ),
    (
        "default::default(",
        "QuickChain DTO/validation source must not fill authority-sensitive data by default",
    ),
];

#[test]
fn quickchain_sources_remain_pure_dto_and_vector_prep() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/quickchain");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    files.sort();

    assert!(
        files.len() >= 12,
        "expected the current ron-proto QuickChain DTO module set to be present"
    );

    for path in files {
        let rel = relative_key(Path::new(env!("CARGO_MANIFEST_DIR")), &path);
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {rel}: {error}"));
        let code_only = strip_comments_and_literals(&raw).to_ascii_lowercase();

        for (token, reason) in BANNED_RUNTIME_TOKENS {
            assert!(
                !code_only.contains(token),
                "{rel}: found forbidden runtime token `{token}` after stripping comments/literals: {reason}"
            );
        }
    }
}

#[test]
fn quickchain_sources_do_not_gain_implicit_default_constructors() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/quickchain");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    files.sort();

    for path in files {
        let rel = relative_key(Path::new(env!("CARGO_MANIFEST_DIR")), &path);
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {rel}: {error}"));
        let code_only = strip_comments_and_literals(&raw).to_ascii_lowercase();

        for (token, reason) in BANNED_DEFAULTING_TOKENS {
            assert!(
                !code_only.contains(token),
                "{rel}: found forbidden default-constructor token `{token}` after stripping comments/literals: {reason}"
            );
        }
    }
}

#[test]
fn quickchain_serde_defaults_stay_structural_not_authority_sensitive() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/quickchain");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    files.sort();

    let denied_field_names = authority_sensitive_default_denied_field_names();
    let mut observed_total = 0usize;

    for path in files {
        let rel = relative_key(Path::new(env!("CARGO_MANIFEST_DIR")), &path);
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {rel}: {error}"));
        let code_without_comments_and_literals =
            strip_comments_and_literals(&raw).to_ascii_lowercase();

        for target in serde_default_targets(&code_without_comments_and_literals) {
            observed_total += 1;

            assert!(
                is_structural_optional_or_growth_target(&target),
                "{rel}: #[serde(default)] may only target Option<T>, Vec<T>, or BTreeMap<K,V> optional/growth fields; found `{target}`"
            );

            let field_name = parse_public_field_name(&target).unwrap_or_else(|| {
                panic!("{rel}: #[serde(default)] target is not a simple public field: `{target}`")
            });

            assert!(
                !denied_field_names.contains(field_name.as_str()),
                "{rel}: #[serde(default)] on authority-sensitive field `{field_name}` is forbidden"
            );
        }
    }

    assert!(
        observed_total >= 10,
        "expected existing reviewed serde(default) optional DTO fields to be observed"
    );
}

fn authority_sensitive_default_denied_field_names() -> BTreeSet<&'static str> {
    BTreeSet::from([
        // Schema/version identity must be explicit.
        "version",
        "schema",
        "domain",
        "domain_separator",
        "canonicalization",
        "encoding",
        // Chain/checkpoint identity and ordering must be explicit.
        "chain_id",
        "checkpoint_id",
        "checkpoint_height",
        "checkpoint_seq",
        "checkpoint_sequence",
        "epoch",
        "epoch_id",
        "epoch_start_ms",
        "epoch_end_ms",
        "created_at_ms",
        "updated_at_ms",
        "expires_at_ms",
        // Root/hash fields must never appear by default.
        "state_root",
        "accounting_root",
        "reward_root",
        "holds_root",
        "empty_tree_hash",
        // Economic operation identity and amount fields must be explicit.
        "operation_id",
        "idempotency_key",
        "account_sequence_actor",
        "actor_account_id",
        "balance_minor",
        "held_minor",
        "current_supply_minor",
        "asset",
        // Receipt identity/status/direction must be explicit.
        "receipt_id",
        "receipt_txid",
        "receipt_status",
        "receipt_direction",
        "receipt_action",
        // Validator/signature identity must be explicit; only the signature list
        // itself may be an empty Vec growth field when absent.
        "validator_id",
        "validator_public_key",
        "signature",
        "signature_algorithm",
        "signed_payload_hash",
    ])
}

fn is_structural_optional_or_growth_target(target: &str) -> bool {
    target.starts_with("pub ")
        && (target.contains(": option<")
            || target.contains(": vec<")
            || target.contains(": btreemap<"))
}

fn parse_public_field_name(target: &str) -> Option<String> {
    let rest = target.strip_prefix("pub ")?;
    let (name, _) = rest.split_once(':')?;
    let name = name.trim().trim_start_matches("r#");

    if name.is_empty() {
        return None;
    }

    Some(name.to_string())
}

fn serde_default_targets(code_without_comments_and_literals: &str) -> Vec<String> {
    let lines: Vec<&str> = code_without_comments_and_literals.lines().collect();
    let mut targets = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        if !line.contains("#[serde(default") {
            continue;
        }

        let mut target = None;
        for candidate in lines.iter().skip(index + 1) {
            let normalized = normalize_code_line(candidate);
            if normalized.is_empty() || normalized.starts_with("#[") {
                continue;
            }

            target = Some(normalized);
            break;
        }

        targets.push(target.unwrap_or_else(|| {
            panic!("#[serde(default)] was not followed by a field declaration")
        }));
    }

    targets
}

fn normalize_code_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
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
