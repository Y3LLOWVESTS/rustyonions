//! RO:WHAT — Internal ROC Beta paid-content access boundary tests for omnigate.
//! RO:WHY — Phase 1 Round 1 must prove omnigate coordinates paid post/comment/article/content_view access without becoming wallet/ledger/receipt/finality authority.
//! RO:INTERACTS — routes/v1/content_view.rs, routes/v1/site_visit.rs, routes/v1/paid.rs, wallet/storage/index coordination tests.
//! RO:INVARIANTS — Omnigate hydrates/coordinates; svc-wallet remains mutation front-door; ron-ledger remains durable truth; no fake receipts, balances, finality, or cache-only unlock.
//! RO:METRICS — none.
//! RO:CONFIG — no config changes.
//! RO:SECURITY — client/cache/policy/manifest/b3 evidence alone cannot unlock paid content.
//! RO:TEST — cargo test -p omnigate --test internal_roc_beta_paid_content_access_boundary.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OmnigatePaidAccessDecision {
    version: u16,
    schema: String,
    action: String,
    content_kind: String,
    source_service: String,
    coordinator_role: String,
    receipt_source: String,
    access_source: String,
    mutation_authority: String,
    render_rule: String,
    amount_minor: String,
}

#[test]
fn omnigate_paid_content_routes_coordinate_quote_pay_and_access_without_ledger_authority() {
    let content_view = read_rel("src/routes/v1/content_view.rs");
    let site_visit = read_rel("src/routes/v1/site_visit.rs");
    let paid = read_rel("src/routes/v1/paid.rs");
    let v1_mod = read_rel("src/routes/v1/mod.rs");

    assert_text_contains_all(
        "content_view route source",
        &content_view,
        &[
            "quote is read-only",
            "pay uses svc-wallet only",
            "no direct ledger mutation",
            "integer minor units only",
            "wallet_receipt",
            "CONTENT_VIEW_QUOTE_SCHEMA",
            "CONTENT_VIEW_PAYMENT_SCHEMA",
            "omnigate.content-view-quote.v1",
            "omnigate.content-view-payment.v1",
            "DEFAULT_CONTENT_VIEW_PRICE_MINOR",
            "DEFAULT_WALLET_BASE_URL",
            "DEFAULT_WALLET_BEARER",
            "is_canonical_b3_cid",
        ],
    );

    assert_text_contains_all(
        "site_visit route source",
        &site_visit,
        &["site_visit", "quote", "pay", "wallet", "receipt"],
    );

    assert_text_contains_all(
        "paid object route source",
        &paid,
        &[
            "prepare/estimate are read-only",
            "write is proxy-only",
            "no wallet, ledger, accounting, or storage mutation here",
            "PREPARE_SCHEMA",
            "omnigate.paid-object-prepare.v1",
            "storage_base_url",
            "should_forward_header",
            "DEFAULT_ACTION",
            "DEFAULT_ASSET",
            "DEFAULT_CURRENCY",
        ],
    );

    assert!(
        v1_mod.contains("content_view") && v1_mod.contains("paid"),
        "omnigate v1 route table must mount paid and content-view surfaces"
    );
}

#[test]
fn omnigate_paid_access_decision_shape_rejects_authority_poison_fields() {
    for field in FORBIDDEN_AUTHORITY_FIELDS {
        let mut value = json!({
            "version": 1,
            "schema": "omnigate.content-view-payment.v1",
            "action": "content_view",
            "content_kind": "article",
            "source_service": "omnigate",
            "coordinator_role": "hydration_access_coordinator",
            "receipt_source": "backend_wallet_ledger_only",
            "access_source": "backend_wallet_ledger_receipt",
            "mutation_authority": "svc-wallet_only",
            "render_rule": "unlock_only_after_backend_access_truth",
            "amount_minor": "5"
        });

        value
            .as_object_mut()
            .expect("paid access object")
            .insert((*field).to_owned(), json!(true));

        let err = serde_json::from_value::<OmnigatePaidAccessDecision>(value)
            .expect_err("omnigate paid access DTO must reject authority poison fields");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn omnigate_existing_regressions_cover_content_view_site_visit_and_paid_storage_access_paths() {
    let expected = [
        (
            "tests/content_view.rs",
            &[
                "quote_returns_manifest_recipient_and_does_not_call_wallet",
                "pay_recovers_wallet_nonce_and_returns_wallet_receipt",
                "quote_rejects_recipient_mismatch",
            ][..],
        ),
        (
            "tests/site_visit.rs",
            &["quote", "pay", "wallet", "receipt"][..],
        ),
        (
            "tests/paid_storage_prepare.rs",
            &["prepare", "estimate"][..],
        ),
        (
            "tests/paid_storage_estimate_proxy.rs",
            &["estimate", "storage"][..],
        ),
        (
            "tests/paid_storage_write_proxy.rs",
            &["paid", "storage", "upstream"][..],
        ),
    ];

    for (path, needles) in expected {
        let text = read_rel(path);
        let lower = text.to_ascii_lowercase();

        for needle in needles {
            assert!(
                lower.contains(&needle.to_ascii_lowercase()),
                "{path} missing required phrase `{needle}`"
            );
        }
    }
}

#[test]
fn omnigate_source_preserves_backend_derived_receipt_access_and_header_filtering_boundaries() {
    let content_view = read_rel("src/routes/v1/content_view.rs");
    let paid = read_rel("src/routes/v1/paid.rs");
    let wallet = read_rel("src/routes/v1/wallet.rs");
    let header_policy = read_rel("src/routes/v1/header_policy.rs");

    assert_text_contains_all(
        "content-view source",
        &content_view,
        &[
            "svc-wallet",
            "wallet_receipt",
            "recipient",
            "payer",
            "nonce",
        ],
    );

    assert_text_contains_all(
        "paid route header/source policy",
        &paid,
        &[
            "should_forward_header",
            "is_hop_by_hop_or_host",
            "header::AUTHORIZATION",
            "idempotency-key",
        ],
    );

    assert!(
        wallet.to_ascii_lowercase().contains("wallet"),
        "wallet route source must remain visibly wallet-scoped"
    );

    assert!(
        header_policy.contains("is_allowed_ron_context_header") || header_policy.contains("x-ron"),
        "omnigate header policy must keep ron context header allowlist explicit"
    );
}

#[test]
fn omnigate_source_does_not_construct_ledger_finality_bridge_staking_or_cache_unlock_authority() {
    let cargo = read_rel("Cargo.toml");
    let source = read_many(&[
        "src/routes/v1/content_view.rs",
        "src/routes/v1/site_visit.rs",
        "src/routes/v1/paid.rs",
        "src/routes/v1/wallet.rs",
        "src/routes/v1/assets.rs",
        "src/routes/v1/sites.rs",
        "src/hydration/compose.rs",
        "src/hydration/planner.rs",
        "src/downstream/storage_client.rs",
        "src/downstream/index_client.rs",
        "src/routes/v1/header_policy.rs",
    ])
    .to_ascii_lowercase();

    assert!(
        !cargo.contains("ron-ledger"),
        "omnigate must not depend on ron-ledger directly"
    );

    for forbidden in [
        "ron_ledger::",
        "ledger::",
        "direct_ledger",
        "commit_to_ledger",
        "omnigate_receipt_hash",
        "receipt_hash: format!",
        "receipt_hash: \"b3:",
        "balance_minor:",
        "available_minor:",
        "held_minor:",
        "state_root:",
        "receipt_root:",
        "checkpoint_root:",
        "validator_signature:",
        "bridge_txid:",
        "staking_position_id:",
        "liquidity_pool_id:",
        "external_settlement_id:",
        "unlock_from_cache",
        "cache_unlock_authority",
        "raw_engagement_mints_roc",
    ] {
        assert!(
            !source.contains(forbidden),
            "omnigate source must not construct paid authority shortcut `{forbidden}`"
        );
    }
}

const FORBIDDEN_AUTHORITY_FIELDS: &[&str] = &[
    "wallet_mutation",
    "ledger_mutation",
    "balance_truth",
    "receipt_truth",
    "paid_unlock_authority",
    "entitlement_truth",
    "client_finality_claim",
    "cache_unlock_authority",
    "gateway_receipt_truth",
    "omnigate_receipt_truth",
    "receipt_id",
    "receipt_hash",
    "receipt_root",
    "receipt_proof",
    "balance_minor",
    "wallet_balance",
    "ledger_balance",
    "paid_proof",
    "unlock_granted",
    "finality",
    "finalized",
    "settlement_status",
    "state_root",
    "checkpoint_root",
    "checkpoint_hash",
    "validator_signature",
    "bridge_txid",
    "bridge_proof",
    "staking_position_id",
    "liquidity_pool_id",
    "external_settlement_id",
    "raw_engagement_mints_roc",
];

fn assert_text_contains_all(label: &str, text: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            text.contains(needle),
            "{label} missing required phrase `{needle}`"
        );
    }
}

fn read_many(paths: &[&str]) -> String {
    let mut out = String::new();

    for path in paths {
        out.push_str(&read_rel(path));
        out.push('\n');
    }

    out
}

fn read_rel(path: &str) -> String {
    let full = crate_root().join(path);
    fs::read_to_string(&full)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", full.display()))
}

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}
