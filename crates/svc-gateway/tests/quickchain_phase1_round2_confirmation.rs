#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Phase 1 Round 2 downstream-confirmation tests for svc-gateway.
//! RO:WHY — Confirms gateway remains public route/admission/proxy only while core QuickChain roots/proofs stay elsewhere.
//! RO:INTERACTS — docs/quickchain-preflight.md, scripts/dev-quickchain-preflight.sh, gateway route/header sources.
//! RO:INVARIANTS — gateway does not become wallet, ledger, root, checkpoint, validator, bridge, or finality authority.
//! RO:METRICS — none; source/docs boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents accepted/backend-derived paid access from being mislabeled as future finality.
//! RO:TEST — cargo test -p svc-gateway --test quickchain_phase1_round2_confirmation.

use std::{
    fs,
    path::{Path, PathBuf},
};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    let path = crate_dir().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
}

fn normalized(text: &str) -> String {
    text.to_ascii_lowercase().replace('`', "")
}

fn strip_line_comments(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with("///"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_rs_sources(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(root).unwrap_or_else(|err| panic!("read dir {}: {err}", root.display()))
    {
        let path = entry
            .unwrap_or_else(|err| panic!("read dir entry in {}: {err}", root.display()))
            .path();

        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("target") {
                continue;
            }
            collect_rs_sources(&path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn read_all_src() -> String {
    let src_root = crate_dir().join("src");
    let mut files = Vec::new();
    collect_rs_sources(&src_root, &mut files);
    files.sort();

    files
        .iter()
        .map(|path| {
            fs::read_to_string(path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn string_literals(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            let mut escaped = false;

            while j < bytes.len() {
                let b = bytes[j];

                if escaped {
                    escaped = false;
                } else if b == b'\\' {
                    escaped = true;
                } else if b == b'"' {
                    break;
                }

                j += 1;
            }

            if j < bytes.len() {
                out.push(source[start..j].to_string());
                i = j + 1;
                continue;
            }
        }

        i += 1;
    }

    out
}

fn assert_contains(haystack: &str, needle: &str, context: &str) {
    assert!(
        haystack.contains(needle),
        "{context} must preserve required marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, forbidden: &str, context: &str) {
    assert!(
        !haystack.contains(forbidden),
        "{context} must not contain forbidden authority marker: {forbidden}"
    );
}

#[test]
fn docs_name_phase1_round2_gateway_downstream_confirmation_boundary() {
    let doc = normalized(&read("docs/quickchain-preflight.md"));

    for required in [
        "phase 1 round 2 downstream confirmation",
        "gateway paid enforcement remains backend-derived",
        "gateway is not wallet truth",
        "gateway is not ledger truth",
        "gateway is not quickchain root authority",
        "gateway is not finality authority",
        "gateway cannot unlock paid content from cache alone",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "quickchain_phase1_round2_confirmation",
    ] {
        assert_contains(&doc, required, "svc-gateway quickchain-preflight.md");
    }
}

#[test]
fn docs_separate_accepted_receipt_from_future_finality_labels() {
    let doc = normalized(&read("docs/quickchain-preflight.md"));

    for required in [
        "accepted backend receipt can unlock local paid content",
        "accepted is not finalized",
        "accepted is not epoch_included",
        "accepted is not anchored",
        "future statuses remain parked: accepted, epoch_included, finalized, anchored",
        "current paid unlock is backend-derived local access, not future quickchain epoch inclusion",
        "gateway never converts accepted wallet/ledger receipt into quickchain finality",
        "gateway must label status honestly and must not fabricate status",
    ] {
        assert_contains(&doc, required, "svc-gateway accepted/finality docs");
    }
}

#[test]
fn source_has_no_direct_ledger_or_quickchain_authority_imports() {
    let source = strip_line_comments(&read_all_src());

    for forbidden in [
        "ron_ledger::",
        "LedgerClient",
        "ledger_commit",
        "direct_ledger",
        "mutate_ledger",
        "quickchain_root",
        "checkpoint_root",
        "validator_signature",
        "validator_set",
        "settlement_finality",
        "external_anchor",
        "bridge_txid",
    ] {
        assert_not_contains(&source, forbidden, "svc-gateway src");
    }
}

#[test]
fn route_literals_do_not_expose_quickchain_settlement_or_external_runtime_paths() {
    let source = strip_line_comments(&read_all_src());

    let forbidden_route_prefixes = [
        "/quickchain",
        "/checkpoint",
        "/checkpoints",
        "/state-root",
        "/receipt-root",
        "/hold-root",
        "/epoch-root",
        "/epoch-checkpoint",
        "/validators",
        "/validator",
        "/bridge",
        "/anchors",
        "/anchor",
        "/external-settlement",
        "/settlement",
        "/staking",
        "/liquidity",
        "/solana",
        "/rox",
    ];

    for literal in string_literals(&source) {
        let lower = literal.to_ascii_lowercase();
        let trimmed = lower.trim();

        for forbidden in forbidden_route_prefixes {
            assert!(
                !trimmed.starts_with(forbidden),
                "svc-gateway must not expose forbidden QuickChain/external route literal `{literal}`"
            );
        }
    }
}

#[test]
fn preflight_runner_dynamically_discovers_this_round2_confirmation_suite() {
    let script = read("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "basename \"$test_path\" .rs",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "svc-gateway dev-quickchain-preflight.sh");
    }
}
