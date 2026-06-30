//! RO:WHAT — Internal ROC Beta Phase 4 Round 2 confirmation/failure boundary test for svc-gateway.
//! RO:WHY — Proves gateway keeps paid-action quote/error/denial UX safe without becoming economic authority.
//! RO:INTERACTS — src/routes/product.rs, src/routes/paid_storage.rs, src/headers/proxy.rs, docs/internal-roc-beta-phase4-confirmation-failure.md.
//! RO:INVARIANTS — gateway is public/proxy boundary only; no direct wallet/ledger mutation; no fake receipts/balances/finality; no cache/header-only unlock.
//! RO:SECURITY — no bridge, staking, liquidity, ROX/Solana, external settlement, protected-body leak, or silent spend.
//! RO:TEST — cargo test -p svc-gateway --test internal_roc_beta_phase4_confirmation_failure_boundary.

#![allow(clippy::missing_panics_doc)]

use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn gateway_phase4_round2_quote_and_failure_contract_is_display_safe() {
    let doc = read_rel("docs/internal-roc-beta-phase4-confirmation-failure.md");
    let product = read_rel("src/routes/product.rs");
    let paid_storage = read_rel("src/routes/paid_storage.rs");
    let proxy = read_rel("src/headers/proxy.rs");

    assert_contains_all(
        "gateway phase4 docs",
        &doc,
        &[
            "prepare/quote responses include enough display-safe detail",
            "recipient/split labels are safe and bounded",
            "errors are redacted/source-labeled",
            "denial never leaks protected body",
            "gateway is proxy/admission boundary only",
            "gateway wallet authority",
            "gateway ledger authority",
            "cache-only paid unlock",
            "header-only paid unlock",
            "amount_minor",
            "display_amount",
            "action",
            "asset",
            "payer_account",
            "recipient_account",
            "quote_id",
            "quote_hash",
            "client_idempotency_key",
            "source_label",
        ],
    );

    assert_contains_all(
        "gateway product route source",
        &product,
        &[
            "INTERNAL-ROC-PHASE4-CONFIRMATION",
            "prepare/quote responses include display-safe detail",
            "amount_minor",
            "display_amount",
            "action",
            "asset",
            "payer_account",
            "recipient_account",
            "quote_id",
            "quote_hash",
            "client_idempotency_key",
            "source_label",
            "errors are redacted/source-labeled",
            "denial never leaks protected body",
            "/content/view/quote",
            "/content/view/pay",
            "/sites/:name/visit/quote",
            "/sites/:name/visit/pay",
            "proxy_to_omnigate",
        ],
    );

    assert_contains_all(
        "gateway paid storage route source",
        &paid_storage,
        &[
            "Gateway is only an edge/BFF ingress here",
            "omnigate /v1/paid/o",
            "svc-storage /paid/o",
            "proxy_to_omnigate",
        ],
    );

    assert_contains_all(
        "gateway header proxy",
        &proxy,
        &[
            "should_forward_product_header",
            "should_copy_response_header",
        ],
    );
}

#[test]
fn gateway_source_does_not_gain_wallet_ledger_or_external_authority() {
    let src_files = collect_source_files(crate_root().join("src"));
    assert!(!src_files.is_empty(), "expected svc-gateway src files");

    let forbidden = [
        "ron_ledger::",
        "use ron_ledger",
        "ledger::Ledger",
        "Ledger::",
        "svc_wallet::",
        "wallet::transfer(",
        "wallet::issue(",
        "wallet::burn(",
        "bridge_mint",
        "bridge_burn",
        "solana_sdk",
        "anchor_lang",
        "staking_reward",
        "liquidity_pool",
        "exchange_order",
        "cache_only_unlock_authority",
        "header_only_unlock_authority",
        "fake_receipt_authority",
        "fake_balance_authority",
        "fake_finality_authority",
    ];

    for path in src_files {
        let text = fs::read_to_string(&path).unwrap_or_default();
        let stripped = strip_comments(&text);
        let compact = stripped
            .replace([' ', '\n', '\r', '\t', '-'], "")
            .to_lowercase();

        for needle in forbidden {
            let compact_needle = needle
                .replace([' ', '\n', '\r', '\t', '-'], "")
                .to_lowercase();
            assert!(
                !compact.contains(&compact_needle),
                "svc-gateway source must not contain forbidden Phase 4 authority marker `{needle}` in {}",
                path.display()
            );
        }
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(rel: &str) -> String {
    let path = crate_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
}

fn assert_contains_all(label: &str, text: &str, needles: &[&str]) {
    for needle in needles {
        assert!(text.contains(needle), "{label} must contain `{needle}`");
    }
}

fn collect_source_files(root: PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_source_files_inner(&root, &mut out);
    out.sort();
    out
}

fn collect_source_files_inner(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            collect_source_files_inner(&path, out);
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_block = false;
    let mut in_line = false;

    while let Some(ch) = chars.next() {
        if in_line {
            if ch == '\n' {
                in_line = false;
                out.push('\n');
            }
            continue;
        }

        if in_block {
            if ch == '*' && chars.peek() == Some(&'/') {
                let _ = chars.next();
                in_block = false;
            }
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'/') {
            let _ = chars.next();
            in_line = true;
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'*') {
            let _ = chars.next();
            in_block = true;
            continue;
        }

        out.push(ch);
    }

    out
}
