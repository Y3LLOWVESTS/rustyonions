//! RO:WHAT — Internal ROC Stabilization paid access error-boundary tests for omnigate.
//! RO:WHY — Product beta readiness requires paid content_view/site_visit failures to stay source-labeled, denial-safe, and non-authoritative.
//! RO:INTERACTS — routes/v1/content_view.rs, routes/v1/site_visit.rs, routes/v1/paid.rs, errors/http_map.rs.
//! RO:INVARIANTS — omnigate coordinates/hydrates only; no direct ledger mutation, fake receipt, fake balance, fake finality, cache-only unlock, or protected body leakage.
//! RO:METRICS — none.
//! RO:CONFIG — no config changes.
//! RO:SECURITY — manifest/index/b3/header/cache evidence cannot become economic proof; paid failures are source-labeled and redacted.
//! RO:TEST — cargo test -p omnigate --test internal_roc_stabilization_paid_access_error_boundary.

#![allow(clippy::missing_panics_doc)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OmnigatePaidAccessErrorBoundary {
    schema: String,
    source_service: String,
    coordinator_role: String,
    error_source_label: String,
    receipt_source: String,
    access_source: String,
    mutation_authority: String,
    render_rule: String,
}

#[test]
fn omnigate_paid_access_errors_are_source_labeled_and_denial_safe() {
    for rel in [
        "src/routes/v1/content_view.rs",
        "src/routes/v1/site_visit.rs",
    ] {
        let source = read_rel(rel);

        assert_all(
            rel,
            &source,
            &[
                "source_label: &'a str",
                "source_label: \"omnigate.paid_access_error.v1\"",
                "prepare/quote responses include display-safe detail",
                "errors are redacted/source-labeled; denial never leaks protected body",
                "quote is read-only",
                "pay uses svc-wallet only",
                "no direct ledger mutation",
                "integer minor units only",
                "wallet_receipt",
                "should_forward_header",
            ],
        );

        assert_none(
            rel,
            &source.to_lowercase(),
            &[
                "force_unlock",
                "cache_only_unlock",
                "unlock_from_cache",
                "fake_receipt",
                "fake_balance",
                "fake_finality",
                "protected_body_on_denial",
                "bridge_runtime",
                "staking_runtime",
                "liquidity_runtime",
                "solana_runtime",
                "rox_runtime",
                "external_settlement",
                "/v1/ledger",
                "ledger_base_url",
            ],
        );
    }
}

#[test]
fn omnigate_paid_routes_preserve_backend_wallet_ledger_truth_without_becoming_authority() {
    let content_view = read_rel("src/routes/v1/content_view.rs");
    let site_visit = read_rel("src/routes/v1/site_visit.rs");
    let paid = read_rel("src/routes/v1/paid.rs");
    let docs = read_rel("docs/internal-roc-stabilization-paid-access.md");

    assert_all(
        "content_view",
        &content_view,
        &[
            "CONTENT_VIEW_QUOTE_SCHEMA",
            "CONTENT_VIEW_PAYMENT_SCHEMA",
            "omnigate.content-view-quote.v1",
            "omnigate.content-view-payment.v1",
            "DEFAULT_WALLET_BASE_URL",
            "DEFAULT_WALLET_BEARER",
            "is_canonical_b3_cid",
            "recipient_account",
            "client_idempotency_key",
        ],
    );

    assert_all(
        "site_visit",
        &site_visit,
        &[
            "SITE_VISIT_QUOTE_SCHEMA",
            "SITE_VISIT_PAYMENT_SCHEMA",
            "site_visit",
            "recipient_account",
            "client_idempotency_key",
            "wallet",
            "receipt",
        ],
    );

    assert_all(
        "paid",
        &paid,
        &[
            "prepare/estimate are read-only",
            "write is proxy-only",
            "no wallet, ledger, accounting, or storage mutation here",
            "omnigate.paid-object-prepare.v1",
        ],
    );

    assert_all(
        "omnigate paid access docs",
        &docs,
        &[
            "source_label = omnigate.paid_access_error.v1",
            "Denial/failure response bodies must not include protected content",
            "quote is read-only; pay uses svc-wallet only",
            "ron-ledger remains durable truth",
        ],
    );
}

#[test]
fn omnigate_paid_access_contract_rejects_stabilization_authority_poison() {
    for field in [
        "fake_receipt",
        "fake_balance",
        "fake_finality",
        "cache_unlock_authority",
        "protected_body_on_denial",
        "wallet_direct_mutation",
        "ledger_direct_mutation",
        "bridge_authority",
        "external_settlement_authority",
    ] {
        let mut value = json!({
            "schema": "omnigate.internal-roc-stabilization-paid-access-error.v1",
            "source_service": "omnigate",
            "coordinator_role": "hydration_access_coordinator",
            "error_source_label": "omnigate.paid_access_error.v1",
            "receipt_source": "backend_wallet_ledger_only",
            "access_source": "backend_derived_access_only",
            "mutation_authority": "svc-wallet_only",
            "render_rule": "deny_until_backend_receipt_access_truth"
        });

        value
            .as_object_mut()
            .expect("contract object")
            .insert(field.to_owned(), json!(true));

        let err = serde_json::from_value::<OmnigatePaidAccessErrorBoundary>(value)
            .expect_err("authority poison fields must be rejected");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn omnigate_existing_regressions_cover_paid_backend_paths() {
    for (rel, snippets) in [
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
            &[
                "quote_returns_10_roc_and_manifest_payout_recipient",
                "pay_recovers_wallet_nonce_and_returns_wallet_receipt",
                "quote_rejects_recipient_mismatch",
            ][..],
        ),
        (
            "tests/internal_roc_beta_paid_content_access_boundary.rs",
            &[
                "no fake receipts, balances, finality, or cache-only unlock",
                "content_view",
                "site_visit",
                "paid",
            ][..],
        ),
    ] {
        assert_all(rel, &read_rel(rel), snippets);
    }
}

fn assert_all(label: &str, haystack: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            haystack.contains(needle),
            "{label} must contain required marker {needle:?}"
        );
    }
}

fn assert_none(label: &str, haystack: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            !haystack.contains(needle),
            "{label} must not contain forbidden marker {needle:?}"
        );
    }
}

fn read_rel(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}
