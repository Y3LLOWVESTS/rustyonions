//! RO:WHAT — Source-authority scan for svc-rewarder QuickChain preflight.
//! RO:WHY — ECON/GOV/SEC: rewarder may plan payouts, but must not grow chain, ledger, validator, or external-settlement authority.
//! RO:INTERACTS — crates/svc-rewarder/src/**/*.rs.
//! RO:INVARIANTS — no src/quickchain runtime module; no direct ledger crate calls; no validator/checkpoint/bridge/staking/liquidity code.
//! RO:METRICS — none; source-only regression tripwire.
//! RO:CONFIG — none.
//! RO:SECURITY — scans code after removing comments and string literals so policy words in docs/comments do not mask runtime authority creep.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_preflight_source_authority_scan.

use std::{
    fs,
    path::{Path, PathBuf},
};

const BANNED_CODE_TOKENS: &[(&str, &str)] = &[
    (
        "ron_ledger::",
        "svc-rewarder must not call ron-ledger directly; wallet remains the mutation front-door",
    ),
    (
        "svc_wallet::",
        "svc-rewarder should hand off through explicit wallet-shaped DTO/HTTP boundaries, not link wallet internals",
    ),
    (
        "ron_proto::quickchain",
        "service runtime must not consume QuickChain DTOs as authority in this preflight slice",
    ),
    (
        "quickchain::",
        "svc-rewarder must not grow a runtime QuickChain module or authority path",
    ),
    (
        "checkpoint",
        "checkpoint/root production is blocked until canonical bytes and locked vectors exist",
    ),
    (
        "merkle",
        "Merkle/root implementation is not authorized in svc-rewarder",
    ),
    (
        "validator",
        "validator machinery is not authorized in svc-rewarder",
    ),
    (
        "consensus",
        "consensus logic is not authorized in svc-rewarder",
    ),
    (
        "bridge",
        "bridge/external-settlement logic is forbidden during internal ROC proving",
    ),
    (
        "staking",
        "staking logic is forbidden during internal ROC proving",
    ),
    (
        "liquidity",
        "liquidity/exchange-facing logic is forbidden during internal ROC proving",
    ),
    (
        "solana",
        "Solana integration is deferred and must not enter svc-rewarder now",
    ),
    (
        "anchor_lang",
        "Solana Anchor integration is deferred and must not enter svc-rewarder now",
    ),
    (
        "spl_token",
        "external token program integration is deferred and must not enter svc-rewarder now",
    ),
    (
        "ethers::",
        "external-chain client logic is forbidden in svc-rewarder",
    ),
    (
        "web3::",
        "external-chain client logic is forbidden in svc-rewarder",
    ),
    (
        "rox",
        "ROX/external token logic is deferred until internal ROC is proven",
    ),
    (
        "operation_id",
        "operation_id is durable ledger-assigned identity; rewarder must not assign it",
    ),
    (
        "account_sequence",
        "account_sequence is ledger-assigned; rewarder must not assign it",
    ),
    (
        "ledger_root",
        "ledger roots are not produced or assigned by rewarder",
    ),
    (
        "receipt_hash",
        "wallet/ledger receipts are backend truth; rewarder must not invent receipt hashes",
    ),
    (
        "finalized",
        "rewarder output must not claim chain or settlement finality",
    ),
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(root)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", root.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|err| panic!("failed to read directory entry: {err}"))
                .path()
        })
        .collect::<Vec<_>>();

    entries.sort();

    for path in entries {
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn raw_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut cursor = start;

    if bytes.get(cursor) == Some(&b'b') {
        cursor += 1;
    }

    if bytes.get(cursor) != Some(&b'r') {
        return None;
    }

    cursor += 1;

    let mut hashes = 0usize;
    while bytes.get(cursor) == Some(&b'#') {
        hashes += 1;
        cursor += 1;
    }

    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }

    cursor += 1;

    while cursor < bytes.len() {
        if bytes[cursor] == b'"' {
            let closing_hashes_match =
                (0..hashes).all(|idx| bytes.get(cursor + 1 + idx) == Some(&b'#'));

            if closing_hashes_match {
                return Some(cursor + 1 + hashes);
            }
        }

        cursor += 1;
    }

    Some(bytes.len())
}

fn quoted_string_end(bytes: &[u8], quote_at: usize) -> usize {
    let mut cursor = quote_at + 1;

    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => cursor += 2,
            b'"' => return cursor + 1,
            _ => cursor += 1,
        }
    }

    bytes.len()
}

fn strip_comments_and_string_literals(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        match bytes[cursor] {
            b'/' if bytes.get(cursor + 1) == Some(&b'/') => {
                cursor += 2;
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
                if cursor < bytes.len() {
                    out.push('\n');
                    cursor += 1;
                }
            }
            b'/' if bytes.get(cursor + 1) == Some(&b'*') => {
                cursor += 2;
                while cursor + 1 < bytes.len() {
                    if bytes[cursor] == b'\n' {
                        out.push('\n');
                    }
                    if bytes[cursor] == b'*' && bytes[cursor + 1] == b'/' {
                        cursor += 2;
                        break;
                    }
                    cursor += 1;
                }
            }
            b'r' | b'b' => {
                if let Some(end) = raw_string_end(bytes, cursor) {
                    out.push(' ');
                    cursor = end;
                } else if bytes[cursor] == b'b' && bytes.get(cursor + 1) == Some(&b'"') {
                    out.push(' ');
                    cursor = quoted_string_end(bytes, cursor + 1);
                } else {
                    out.push(bytes[cursor] as char);
                    cursor += 1;
                }
            }
            b'"' => {
                out.push(' ');
                cursor = quoted_string_end(bytes, cursor);
            }
            byte => {
                out.push(byte as char);
                cursor += 1;
            }
        }
    }

    out
}

#[test]
fn rewarder_source_tree_does_not_define_quickchain_runtime_modules() {
    let src_root = crate_dir().join("src");
    let mut files = Vec::new();
    collect_rs_files(&src_root, &mut files);

    assert!(
        files.len() >= 20,
        "expected the svc-rewarder source tree to be present; found only {} files",
        files.len()
    );

    for path in files {
        let rel = path
            .strip_prefix(crate_dir())
            .unwrap_or(path.as_path())
            .display()
            .to_string()
            .replace('\\', "/");

        assert!(
            !rel.contains("/quickchain"),
            "{rel}: svc-rewarder must not grow a src/quickchain runtime module"
        );
    }
}

#[test]
fn rewarder_source_has_no_direct_chain_or_ledger_authority_tokens() {
    let src_root = crate_dir().join("src");
    let mut files = Vec::new();
    collect_rs_files(&src_root, &mut files);
    files.sort();

    for path in files {
        let rel = path
            .strip_prefix(crate_dir())
            .unwrap_or(path.as_path())
            .display()
            .to_string();

        let raw =
            fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {rel}: {err}"));
        let code_only = strip_comments_and_string_literals(&raw).to_ascii_lowercase();

        for (token, reason) in BANNED_CODE_TOKENS {
            assert!(
                !code_only.contains(token),
                "{rel}: found forbidden runtime token `{token}` after stripping comments/literals: {reason}"
            );
        }
    }
}
