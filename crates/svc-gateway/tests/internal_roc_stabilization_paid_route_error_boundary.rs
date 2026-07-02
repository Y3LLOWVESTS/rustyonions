//! RO:WHAT — Internal ROC Stabilization paid route error-boundary tests for svc-gateway.
//! RO:WHY — Product beta readiness requires public paid routes to stay proxy-only while failures are source-labeled and non-authoritative.
//! RO:INTERACTS — src/routes/product.rs, src/routes/paid_storage.rs, src/errors.rs, content_view/site_visit proxy tests.
//! RO:INVARIANTS — gateway is public boundary only; no wallet/ledger mutation, fake receipt, fake balance, fake finality, cache-only unlock, or protected body leakage.
//! RO:METRICS — none.
//! RO:CONFIG — no config changes.
//! RO:SECURITY — gateway transport failures are source-labeled/redacted; client headers/cache cannot become paid access truth.
//! RO:TEST — cargo test -p svc-gateway --test internal_roc_stabilization_paid_route_error_boundary.

#![allow(clippy::missing_panics_doc)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GatewayPaidRouteErrorBoundary {
    schema: String,
    source_service: String,
    route_role: String,
    error_source_label: String,
    receipt_source: String,
    access_source: String,
    mutation_authority: String,
    finality_authority: String,
}

#[test]
fn gateway_paid_route_errors_are_source_labeled_redacted_and_retry_safe() {
    let errors = read_rel("src/errors.rs");
    let docs = read_rel("docs/internal-roc-stabilization-paid-route.md");

    assert_all(
        "svc-gateway errors.rs",
        &errors,
        &[
            "pub source_label: &'a str",
            "svc-gateway.public_edge_error.v1",
            "code: \"upstream_unavailable\"",
            "retryable: true",
            "upstream transport error",
            "reason: reason_field",
            "StatusCode::BAD_GATEWAY",
        ],
    );

    assert_all(
        "svc-gateway paid-route stabilization docs",
        &docs,
        &[
            "source_label = svc-gateway.public_edge_error.v1",
            "Gateway transport failures",
            "must not leak secrets",
            "protected payload truth, receipt truth, balance truth, and paid access truth remain backend-owned",
        ],
    );

    assert_none(
        "svc-gateway error boundary",
        &errors.to_lowercase(),
        &[
            "bearer dev",
            "private_key",
            "seed_phrase",
            "raw_capability",
            "protected_body",
            "protected_content",
        ],
    );
}

#[test]
fn gateway_paid_routes_remain_proxy_only_and_do_not_unlock_or_mutate() {
    let product = read_rel("src/routes/product.rs");
    let paid_storage = read_rel("src/routes/paid_storage.rs");
    let header_policy = read_rel("src/headers/proxy.rs");

    assert_all(
        "svc-gateway product paid routes",
        &product,
        &[
            "RO:INVARIANTS — proxy-only",
            "no direct passport/wallet/ledger mutation",
            "/content/view/quote",
            "/content/view/pay",
            "/v1/content/view/quote",
            "/v1/content/view/pay",
            "/sites/:name/visit/quote",
            "/sites/:name/visit/pay",
            "proxy_to_omnigate",
            "should_forward_product_header",
            "should_copy_response_header",
            "omnigate_connect",
            "omnigate_read",
        ],
    );

    assert_all(
        "svc-gateway paid storage route",
        &paid_storage,
        &[
            "estimate is read-only",
            "write is proxy-only",
            "no wallet, ledger, accounting, or storage mutation here",
            "proxy_to_omnigate",
        ],
    );

    assert_all(
        "svc-gateway header proxy policy",
        &header_policy,
        &[
            "should_forward_product_header",
            "idempotency-key",
            "x-correlation-id",
            "x-request-id",
            "is_hop_by_hop_or_host",
        ],
    );

    assert_none(
        "svc-gateway product/paid sources",
        &format!("{product}\n{paid_storage}").to_lowercase(),
        &[
            "force_unlock",
            "cache_only_unlock",
            "unlock_from_cache",
            "fake_receipt",
            "fake_balance",
            "fake_finality",
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

#[test]
fn gateway_paid_route_contract_rejects_stabilization_authority_poison() {
    for field in [
        "fake_receipt",
        "fake_balance",
        "fake_finality",
        "cache_unlock_authority",
        "protected_body_on_denial",
        "wallet_mutation_authority",
        "ledger_mutation_authority",
        "bridge_authority",
        "external_settlement_authority",
    ] {
        let mut value = json!({
            "schema": "svc-gateway.internal-roc-stabilization-paid-route-error.v1",
            "source_service": "svc-gateway",
            "route_role": "public_proxy_boundary",
            "error_source_label": "svc-gateway.public_edge_error.v1",
            "receipt_source": "backend_wallet_ledger_only",
            "access_source": "omnigate_backend_access_only",
            "mutation_authority": "svc-wallet_only",
            "finality_authority": "none_current_runtime"
        });

        value
            .as_object_mut()
            .expect("contract object")
            .insert(field.to_owned(), json!(true));

        let err = serde_json::from_value::<GatewayPaidRouteErrorBoundary>(value)
            .expect_err("authority poison fields must be rejected");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn gateway_existing_proxy_regressions_cover_paid_content_and_site_visit() {
    for (rel, snippets) in [
        (
            "tests/content_view_routes_proxy.rs",
            &[
                "/content/view/quote",
                "/content/view/pay",
                "/v1/content/view/quote",
                "/v1/content/view/pay",
                "idempotency-key",
                "connection",
                "body_len",
            ][..],
        ),
        (
            "tests/site_visit_routes_proxy.rs",
            &[
                "/sites/ron7/visit/quote",
                "/sites/ron7/visit/pay",
                "/v1/sites/ron7/visit/quote",
                "/v1/sites/ron7/visit/pay",
                "idempotency-key",
                "connection",
                "body_len",
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
