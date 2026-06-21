#![allow(clippy::missing_panics_doc)]

//! RO:WHAT — Transport/header authority tests for omnigate QuickChain Phase-0.
//! RO:WHY — Caller-supplied headers must not become paid/receipt/finality/root authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, routes/v1/header_policy.rs, and v1 paid/product route sources.
//! RO:INVARIANTS — transport metadata is not economic truth; backend wallet/ledger receipts remain authoritative.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — blocks fake client receipt/unlock/paid/finality/root/validator/bridge header creep while preserving normal product metadata.
//! RO:TEST — cargo test -p omnigate --test quickchain_preflight_transport_authority.

use std::{
    fs,
    path::{Path, PathBuf},
};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(path: &str) -> String {
    let full = crate_root().join(path);
    fs::read_to_string(&full).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", full.display());
    })
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|err| {
        panic!("failed to read directory {}: {err}", root.display());
    });

    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read directory entry: {err}"))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "rs")
        {
            out.push(path);
        }
    }
}

fn production_sources() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);

    files
        .into_iter()
        .filter(|path| !path.ends_with("routes/v1/header_policy.rs"))
        .map(|path| {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            (path, text)
        })
        .collect()
}

fn route_sources() -> Vec<(&'static str, String)> {
    [
        "src/routes/v1/content_view.rs",
        "src/routes/v1/site_visit.rs",
        "src/routes/v1/streams.rs",
        "src/routes/v1/chat.rs",
        "src/routes/v1/paid.rs",
        "src/routes/v1/wallet.rs",
    ]
    .into_iter()
    .map(|rel| (rel, read_rel(rel)))
    .collect()
}

fn proxy_route_sources() -> &'static [&'static str] {
    &[
        "src/routes/v1/assets.rs",
        "src/routes/v1/chat.rs",
        "src/routes/v1/content_view.rs",
        "src/routes/v1/paid.rs",
        "src/routes/v1/profile.rs",
        "src/routes/v1/site_visit.rs",
        "src/routes/v1/sites.rs",
        "src/routes/v1/text_assets.rs",
    ]
}

#[test]
fn docs_state_transport_headers_are_not_economic_authority() {
    let docs = read_rel("docs/quickchain-preflight.md");

    for required in [
        "Transport headers are not economic authority",
        "Client-supplied receipt, paid, unlocked, entitlement, finality, balance, or cache headers must never prove paid access inside omnigate",
        "`x-ron-wallet-account` and `x-ron-passport` may identify payer intent or passport context",
        "cannot prove payment",
        "cannot unlock paid content",
        "cannot fabricate a receipt",
        "cannot replace backend wallet/ledger truth",
        "`x-ron-receipt`",
        "`x-ron-paid`",
        "`x-ron-unlocked`",
        "`x-ron-finalized`",
        "backend wallet receipt truth",
        "wallet receipt lookup",
        "QuickChain authority-like `x-ron-*` headers are stripped before downstream forwarding",
        "`x-ron-operation-id`",
        "`x-ron-account-sequence`",
        "`x-ron-state-root`",
        "`x-ron-receipt-root`",
        "`x-ron-checkpoint-*`",
        "`x-ron-validator-*`",
        "`x-ron-bridge-*`",
        "`x-ron-quickchain-*`",
    ] {
        assert!(
            docs.contains(required),
            "quickchain-preflight.md must preserve transport/header authority boundary `{required}`"
        );
    }
}

#[test]
fn route_sources_do_not_accept_fake_x_ron_authority_headers() {
    let forbidden_exact_header_tokens = forbidden_quickchain_authority_header_tokens();

    for (rel, source) in route_sources() {
        for header in x_ron_header_tokens(&source) {
            assert!(
                !forbidden_exact_header_tokens.contains(&header.as_str()),
                "{rel} must not parse or trust fake caller authority header `{header}`"
            );
        }
    }
}

#[test]
fn production_x_ron_headers_do_not_define_quickchain_authority_tokens_outside_policy() {
    let forbidden_exact_header_tokens = forbidden_quickchain_authority_header_tokens();

    for (path, source) in production_sources() {
        for header in x_ron_header_tokens(&source) {
            assert!(
                !forbidden_exact_header_tokens.contains(&header.as_str()),
                "{} contains forbidden QuickChain/economic authority header token `{header}` outside the shared header policy",
                path.display()
            );
        }
    }
}

#[test]
fn shared_header_policy_blocks_quickchain_authority_headers() {
    let policy = read_rel("src/routes/v1/header_policy.rs");

    for required in [
        "is_allowed_ron_context_header",
        "fn is_quickchain_authority_header",
        "x-ron-operation-id",
        "x-ron-account-sequence",
        "x-ron-receipt",
        "x-ron-paid",
        "x-ron-unlocked",
        "x-ron-balance",
        "x-ron-root",
        "x-ron-state-root",
        "x-ron-receipt-root",
        "x-ron-checkpoint",
        "x-ron-finalized",
        "x-ron-finality",
        "x-ron-epoch-included",
        "x-ron-anchored",
        "x-ron-external-settlement",
        "x-ron-validator",
        "x-ron-bridge",
        "x-ron-spend-authority",
        "raw.starts_with(\"x-ron-quickchain-\")",
        "raw.starts_with(\"x-ron-validator-\")",
        "raw.starts_with(\"x-ron-bridge-\")",
        "raw.starts_with(\"x-ron-checkpoint-\")",
        "raw.starts_with(\"x-ron-proof-\")",
    ] {
        assert!(
            policy.contains(required),
            "shared header policy must preserve forbidden authority marker `{required}`"
        );
    }
}

#[test]
fn proxy_like_routes_use_shared_header_policy_for_x_ron_forwarding() {
    for rel in proxy_route_sources() {
        let source = read_rel(rel);

        assert!(
            source.contains("super::header_policy::is_allowed_ron_context_header(name)"),
            "{rel} must use the shared QuickChain-aware header policy for x-ron-* forwarding"
        );

        assert!(
            !source.contains(r#"starts_with("x-ron-")"#),
            "{rel} must not directly wildcard-forward x-ron-* headers"
        );
    }
}

#[test]
fn receipt_or_paid_authority_must_come_from_backend_paths_not_request_headers() {
    for (rel, source) in route_sources() {
        let lower = source.to_ascii_lowercase();

        let has_backend_receipt_truth = lower.contains("wallet_receipt")
            || lower.contains("wallet receipt lookup")
            || lower.contains("receipt validation")
            || lower.contains("/v1/transfer")
            || lower.contains("/v1/hold");

        let has_header_authority_shape = lower.contains("header_receipt")
            || lower.contains("receipt_header")
            || lower.contains("header_paid")
            || lower.contains("paid_header")
            || lower.contains("unlock_header")
            || lower.contains("header_unlock")
            || lower.contains("entitlement_header")
            || lower.contains("header_entitlement");

        assert!(
            !has_header_authority_shape,
            "{rel} must not derive receipt/paid/unlock entitlement from request-header authority variables"
        );

        if rel.ends_with("content_view.rs")
            || rel.ends_with("site_visit.rs")
            || rel.ends_with("streams.rs")
            || rel.ends_with("chat.rs")
            || rel.ends_with("wallet.rs")
        {
            assert!(
                has_backend_receipt_truth,
                "{rel} must preserve backend wallet/receipt truth wording or path references"
            );
        }
    }
}

fn forbidden_quickchain_authority_header_tokens() -> &'static [&'static str] {
    &[
        // Direct fake payment/receipt claims.
        "x-ron-receipt",
        "x-ron-receipt-id",
        "x-ron-receipt-hash",
        "x-ron-paid",
        "x-ron-unlocked",
        "x-ron-unlock",
        "x-ron-entitlement",
        // Fake finality / balance / ledger truth.
        "x-ron-finalized",
        "x-ron-finality",
        "x-ron-epoch-included",
        "x-ron-anchored",
        "x-ron-balance",
        "x-ron-ledger",
        "x-ron-operation-id",
        "x-ron-account-sequence",
        // Forbidden QuickChain/root/checkpoint/validator authority surface.
        "x-ron-root",
        "x-ron-state-root",
        "x-ron-receipt-root",
        "x-ron-hold-root",
        "x-ron-epoch-root",
        "x-ron-checkpoint",
        "x-ron-validator",
        "x-ron-bridge",
        // Fake spend/capture authority.
        "x-ron-spend-authority",
        "x-ron-capture-authority",
    ]
}

fn x_ron_header_tokens(source: &str) -> Vec<String> {
    let mut out = Vec::new();

    for literal in string_literals(source) {
        let lower = literal.to_ascii_lowercase();
        let bytes = lower.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            let Some(pos) = lower[i..].find("x-ron-") else {
                break;
            };

            let start = i + pos;
            let mut end = start;

            while end < bytes.len() && is_header_token_byte(bytes[end]) {
                end += 1;
            }

            if end > start {
                out.push(lower[start..end].to_string());
            }

            i = end.saturating_add(1);
        }
    }

    out.sort();
    out.dedup();
    out
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

fn is_header_token_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
}
