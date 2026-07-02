//! RO:WHAT — Internal ROC Stabilization boundary tests for svc-index pointer and lookup non-authority.
//! RO:WHY — Product beta readiness requires asset/site/provider pointers to hydrate UX without becoming economic truth.
//! RO:INTERACTS — types.rs, store/keys.rs, index_manifests routes, resolve/providers routes, pointer regressions.
//! RO:INVARIANTS — index is lookup/pointer only; owner wallet/passport metadata is reference-only.
//! RO:SECURITY — no receipt/balance/finality/paid-unlock/wallet/ledger/bridge/staking/liquidity/external-settlement authority.
//! RO:TEST — cargo test -p svc-index --test internal_roc_stabilization_pointer_non_authority_boundary.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexPointerNonAuthorityContract {
    schema: String,
    source_crate: String,
    role: String,
    pointer_role: String,
    owner_metadata_role: String,
    paid_access_truth: String,
    wallet_truth: String,
    ledger_truth: String,
}

#[test]
fn index_sources_keep_pointer_lookup_and_reference_metadata_boundaries() {
    let types = read_rel("src/types.rs");
    let keys = read_rel("src/store/keys.rs");
    let index_manifests = read_rel("src/http/routes/index_manifests.rs");
    let resolve_route = read_rel("src/http/routes/resolve.rs");
    let providers_route = read_rel("src/http/routes/providers.rs");
    let resolve_pipeline = read_rel("src/pipeline/resolve.rs");

    assert_all(
        "index pointer DTO source",
        &types,
        &[
            "PutAssetManifestPointer",
            "PutSiteManifestPointer",
            "AssetManifestPointer",
            "SiteManifestPointer",
            "asset_cid",
            "manifest_cid",
            "owner_passport_subject",
            "owner_wallet_account",
            "#[serde(deny_unknown_fields)]",
        ],
    );

    assert_all(
        "index manifest keyspace source",
        &keys,
        &[
            "ASSET_MANIFEST_PREFIX",
            "SITE_MANIFEST_PREFIX",
            "asset_manifest_key",
            "site_manifest_key",
        ],
    );

    assert_all(
        "index manifest route source",
        &index_manifests,
        &[
            "put_asset_manifest",
            "get_asset_manifest",
            "put_site_manifest",
            "get_site_manifest",
            "AssetManifestPointer",
            "SiteManifestPointer",
        ],
    );

    assert_any_lower(
        "resolve route remains lookup response",
        &resolve_route,
        &["resolve", "manifest", "providers", "not_found", "not found"],
    );

    assert_any_lower(
        "providers route remains provider lookup",
        &providers_route,
        &["providers", "cid", "not_found", "not found"],
    );

    assert_any_lower(
        "resolve pipeline remains lookup/pointer path",
        &resolve_pipeline,
        &["resolve", "manifest", "providers", "cache"],
    );

    assert_none_lower(
        "svc-index production source forbidden authority markers",
        &format!("{types}\n{keys}\n{index_manifests}\n{resolve_route}\n{providers_route}\n{resolve_pipeline}"),
        &[
            "svc_wallet::",
            "ron_ledger::",
            "wallet_mutate(",
            "ledger_mutate(",
            "mutate_wallet(",
            "mutate_ledger(",
            "create_receipt(",
            "insert_receipt(",
            "commit_receipt(",
            "mutate_balance(",
            "set_balance(",
            "grant_paid_access(",
            "unlock_paid_content(",
            "cache_only_unlock(",
            "fake_receipt(",
            "fake_balance(",
            "fake_finality(",
            "bridge_runtime",
            "rox_runtime",
            "solana_runtime",
            "staking_runtime",
            "liquidity_runtime",
            "external_settlement_runtime",
            "mint_rox(",
            "burn_rox(",
        ],
    );
}

#[test]
fn existing_index_regressions_are_wired_into_stabilization_surface() {
    let paid_pointer = read_rel("tests/internal_roc_beta_paid_content_pointer_boundary.rs");
    let pointer_authority = read_rel("tests/quickchain_preflight_pointer_authority.rs");
    let routes = read_rel("tests/quickchain_preflight_routes.rs");
    let boundary = read_rel("tests/quickchain_preflight_boundary.rs");
    let value_loop = read_rel("tests/quickchain_preflight_value_loop_boundary.rs");
    let http_contract = read_rel("tests/http_contract.rs");
    let integration = read_rel("tests/integration.rs");
    let prop_index = read_rel("tests/prop_index.rs");

    assert_loaded_boundary_regression(
        "paid-content pointer boundary regression",
        &paid_pointer,
        "paid",
    );
    assert_any_lower(
        "paid-content pointer labels",
        &paid_pointer,
        &["paid_post", "paid_comment", "paid_article", "content_view"],
    );
    assert_loaded_boundary_regression(
        "pointer authority regression",
        &pointer_authority,
        "pointer",
    );
    assert_loaded_boundary_regression("route boundary regression", &routes, "routes");
    assert_loaded_boundary_regression("index boundary regression", &boundary, "authority");
    assert_loaded_boundary_regression("value-loop boundary regression", &value_loop, "wallet");
    assert_loaded_boundary_regression("http contract regression", &http_contract, "manifest");
    assert_any_lower(
        "http contract pointer wire regression",
        &http_contract,
        &[
            "putassetmanifestpointer",
            "assetmanifestpointer",
            "normalize_b3_cid",
            "normalize_asset_kind",
            "normalize_site_name",
            "raw_bytes",
        ],
    );
    assert_loaded_boundary_regression("integration regression", &integration, "manifest");
    assert_loaded_boundary_regression("property index regression", &prop_index, "manifest");
}

#[test]
fn index_pointer_contract_rejects_authority_poison_fields() {
    for field in [
        "wallet_mutation_authority",
        "ledger_mutation_authority",
        "receipt_truth",
        "balance_truth",
        "paid_unlock_authority",
        "entitlement_truth",
        "finality_truth",
        "index_unlock_authority",
        "pointer_unlock_authority",
        "manifest_unlock_authority",
        "cache_unlock_authority",
        "owner_wallet_spend_authority",
        "passport_paid_proof",
        "provider_settlement_truth",
        "bridge_authority",
        "external_settlement_authority",
        "rox_solana_authority",
        "staking_authority",
        "liquidity_authority",
        "raw_engagement_mint_authority",
    ] {
        let mut value = json!({
            "schema": "svc-index.internal-roc-stabilization-pointer-non-authority.v1",
            "source_crate": "svc-index",
            "role": "lookup_pointer_navigation_infrastructure_only",
            "pointer_role": "asset_site_manifest_reference_only",
            "owner_metadata_role": "wallet_and_passport_reference_strings_only",
            "paid_access_truth": "backend_gateway_omnigate_wallet_ledger_only",
            "wallet_truth": "svc-wallet_only",
            "ledger_truth": "ron-ledger_only"
        });

        value
            .as_object_mut()
            .expect("contract object")
            .insert(field.to_owned(), json!(true));

        let err = serde_json::from_value::<IndexPointerNonAuthorityContract>(value)
            .expect_err("authority poison fields must reject");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn stabilization_doc_states_index_product_beta_truth_boundary() {
    let doc = read_rel("docs/internal-roc-stabilization-pointer-non-authority.md");

    assert_all(
        "svc-index stabilization doc",
        &doc,
        &[
            "b3 identifies bytes",
            "crab:// is navigation",
            "names are mutable pointers",
            "manifests describe content",
            "owner wallet/passport fields are reference metadata only",
            "index pointer as paid access proof",
            "manifest pointer as receipt proof",
            "provider lookup as settlement proof",
            "owner wallet reference as spend authority",
            "resolve response as entitlement truth",
            "backend wallet/ledger receipt/access truth",
        ],
    );
}

fn assert_loaded_boundary_regression(label: &str, source: &str, expected_marker: &str) {
    assert!(
        source.len() > 256,
        "{label} should be a non-empty focused regression source"
    );
    assert!(
        source.contains("RO:WHAT")
            || source.contains("Internal ROC")
            || source.contains("#![allow"),
        "{label} must retain boundary/test header markers"
    );
    assert!(
        source.to_lowercase().contains(expected_marker),
        "{label} must contain expected marker {expected_marker:?}"
    );
}

fn assert_all(label: &str, haystack: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            haystack.contains(needle),
            "{label} must contain required marker {needle:?}"
        );
    }
}

fn assert_any_lower(label: &str, haystack: &str, needles: &[&str]) {
    let lower = haystack.to_lowercase();
    assert!(
        needles.iter().any(|needle| lower.contains(needle)),
        "{label} must contain one of {needles:?}"
    );
}

fn assert_none_lower(label: &str, haystack: &str, needles: &[&str]) {
    let lower = haystack.to_lowercase();
    for needle in needles {
        assert!(
            !lower.contains(needle),
            "{label} must not contain forbidden marker {needle:?}"
        );
    }
}

fn read_rel(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}
