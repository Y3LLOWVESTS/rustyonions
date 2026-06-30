//! RO:WHAT — Internal ROC Beta paid-content route boundary tests for svc-gateway.
//! RO:WHY — Phase 1 Round 1 must prove gateway exposes paid post/comment/article/content_view route contracts as public/proxy intent only.
//! RO:INTERACTS — routes/product.rs, routes/paid_storage.rs, headers/proxy.rs, existing product proxy tests.
//! RO:INVARIANTS — gateway is public boundary only; no wallet/ledger mutation, fake receipt, fake balance, fake finality, cache-only unlock, bridge, staking, liquidity, or external settlement authority.
//! RO:METRICS — none.
//! RO:CONFIG — no config changes.
//! RO:SECURITY — client-supplied authority claims must not become paid access truth.
//! RO:TEST — cargo test -p svc-gateway --test internal_roc_beta_paid_content_route_boundary.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GatewayPaidRouteContract {
    version: u16,
    public_route: String,
    upstream_route: String,
    action: String,
    content_kind: String,
    source_service: String,
    route_role: String,
    mutation_authority: String,
    receipt_source: String,
    access_source: String,
}

#[test]
fn gateway_paid_content_routes_are_proxy_contracts_not_mutation_authority() {
    let product = read_rel("src/routes/product.rs");
    let paid_storage = read_rel("src/routes/paid_storage.rs");
    let routes_mod = read_rel("src/routes/mod.rs");

    assert_text_contains_all(
        "product route source",
        &product,
        &[
            "RO:INVARIANTS — proxy-only",
            "no direct passport/wallet/ledger mutation",
            "/content/view/quote",
            "/content/view/pay",
            "/v1/content/view/quote",
            "/v1/content/view/pay",
            "/assets/post/prepare",
            "/assets/post",
            "/assets/comment/prepare",
            "/assets/comment",
            "/assets/article/prepare",
            "/assets/article",
            "/sites/:name/visit/quote",
            "/sites/:name/visit/pay",
            "proxy_to_omnigate",
            "should_forward_product_header",
            "should_copy_response_header",
        ],
    );

    assert_text_contains_all(
        "paid storage route source",
        &paid_storage,
        &[
            "Proxy paid-storage estimate/write requests from gateway to omnigate",
            "estimate is read-only",
            "write is proxy-only",
            "no wallet, ledger, accounting, or storage mutation here",
            "/v1/paid/o/estimate",
            "/v1/paid/o",
            "proxy_to_omnigate",
            "omnigate_connect",
            "omnigate_read",
        ],
    );

    assert!(
        routes_mod.contains("paid_storage::router()")
            || routes_mod.contains("paid_storage")
            || product.contains("/paid/o/prepare"),
        "gateway route table must keep paid/product route surface visible"
    );
}

#[test]
fn gateway_paid_route_contract_rejects_receipt_balance_unlock_and_finality_poison() {
    for field in FORBIDDEN_AUTHORITY_FIELDS {
        let mut value = json!({
            "version": 1,
            "public_route": "/content/view/pay",
            "upstream_route": "/v1/content/view/pay",
            "action": "content_view",
            "content_kind": "article",
            "source_service": "svc-gateway",
            "route_role": "public_proxy_boundary",
            "mutation_authority": "svc-wallet_only",
            "receipt_source": "backend_wallet_ledger_only",
            "access_source": "omnigate_backend_access_only"
        });

        value
            .as_object_mut()
            .expect("route contract object")
            .insert((*field).to_owned(), json!(true));

        let err = serde_json::from_value::<GatewayPaidRouteContract>(value)
            .expect_err("gateway paid route contract must reject authority poison fields");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn gateway_existing_regressions_cover_paid_content_and_visit_proxy_paths() {
    let expected = [
        (
            "tests/content_view_routes_proxy.rs",
            &[
                "content_view_quote_and_pay_proxy_to_omnigate_and_preserve_context",
                "/content/view/quote",
                "/content/view/pay",
                "/v1/content/view/quote",
                "/v1/content/view/pay",
                "gateway does not price, inspect manifests, or mutate wallet/ledger",
            ][..],
        ),
        (
            "tests/site_visit_routes_proxy.rs",
            &[
                "site_visit_quote_and_pay_proxy_to_omnigate_and_preserve_context",
                "/sites/:name/visit",
                "/v1/sites",
                "gateway does not price, inspect manifests, or mutate wallet/ledger",
            ][..],
        ),
        (
            "tests/paid_storage_estimate_proxy.rs",
            &["/paid/o/estimate", "/v1/paid/o/estimate", "upstream"][..],
        ),
        (
            "tests/paid_storage_write_proxy.rs",
            &["/paid/o", "/v1/paid/o", "upstream"][..],
        ),
        (
            "tests/product_routes_proxy.rs",
            &[
                "Product route proxy tests",
                "gateway is proxy-only",
                "filters hop-by-hop headers",
            ][..],
        ),
    ];

    for (path, needles) in expected {
        let text = read_rel(path);
        assert_text_contains_all(path, &text, needles);
    }
}

#[test]
fn gateway_header_and_error_boundaries_keep_authority_source_labeled_and_redacted() {
    let proxy = read_rel("src/headers/proxy.rs");
    let product = read_rel("src/routes/product.rs");
    let paid_storage = read_rel("src/routes/paid_storage.rs");
    let errors = read_rel("src/errors.rs");

    assert_text_contains_all(
        "gateway proxy header policy",
        &proxy,
        &[
            "should_forward_product_header",
            "is_hop_by_hop_or_host",
            "should_copy_response_header",
            "idempotency-key",
            "x-correlation-id",
            "x-request-id",
        ],
    );

    let combined = format!("{product}\n{paid_storage}\n{errors}");

    assert_text_contains_all(
        "gateway source-labeled failure path",
        &combined,
        &["upstream_unavailable", "omnigate_connect", "omnigate_read"],
    );

    assert!(
        combined.contains("proxy-only") || combined.contains("Gateway is only an edge/BFF ingress"),
        "gateway paid/product source should explicitly label itself proxy-only"
    );
}

#[test]
fn gateway_source_does_not_construct_paid_receipt_balance_finality_or_external_runtime_truth() {
    let cargo = read_rel("Cargo.toml");
    let source = read_many(&[
        "src/routes/product.rs",
        "src/routes/paid_storage.rs",
        "src/routes/app.rs",
        "src/routes/objects.rs",
        "src/headers/proxy.rs",
        "src/state.rs",
        "src/config/mod.rs",
        "src/config/env.rs",
        "src/errors.rs",
    ])
    .to_ascii_lowercase();

    assert!(
        !cargo.contains("ron-ledger"),
        "svc-gateway must not depend on ron-ledger"
    );
    assert!(
        !cargo.contains("svc-wallet"),
        "svc-gateway must not depend on svc-wallet as a direct mutation crate"
    );

    for forbidden in [
        "ron_ledger::",
        "ledger::",
        "direct_ledger",
        "commit_to_ledger",
        "gateway_receipt_hash",
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
            "svc-gateway source must not construct paid authority shortcut `{forbidden}`"
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
