//! RO:WHAT — Internal ROC Stabilization boundary tests for svc-storage b3 artifacts and paid-write admission.
//! RO:WHY — Product beta readiness requires storage to serve bytes and paid-write evidence without becoming economic truth.
//! RO:INTERACTS — storage modules, paid object routes, paid write policy, settlement, accounting exporter.
//! RO:INVARIANTS — b3 proves bytes only; storage admission is not finality; metering is not balance truth.
//! RO:SECURITY — no direct wallet/ledger mutation, fake receipt/balance/finality, cache-only unlock, bridge, staking, liquidity, or external settlement.
//! RO:TEST — cargo test -p svc-storage --test internal_roc_stabilization_b3_artifact_non_authority_boundary.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageArtifactNonAuthorityContract {
    schema: String,
    source_crate: String,
    role: String,
    b3_role: String,
    paid_admission_role: String,
    wallet_role: String,
    ledger_truth: String,
    accounting_role: String,
}

#[test]
fn storage_sources_keep_b3_paid_admission_settlement_and_accounting_boundaries() {
    let storage_mod = read_rel("src/storage/mod.rs");
    let cas = read_rel("src/storage/cas.rs");
    let paid_object = read_rel("src/http/routes/paid_object.rs");
    let paid_estimate = read_rel("src/http/routes/paid_estimate.rs");
    let paid_write = read_rel("src/policy/paid_write.rs");
    let settlement = read_rel("src/policy/settlement.rs");
    let economics = read_rel("src/policy/economics.rs");
    let accounting = read_rel("src/accounting/exporter.rs");

    assert_any_lower(
        "storage module keeps b3/content-addressed role",
        &format!("{storage_mod}\n{cas}"),
        &["b3", "content-addressed", "content addressed", "cid"],
    );

    assert_any_lower(
        "paid object route hashes body to b3/cid",
        &paid_object,
        &["blake3", "b3:", "cid"],
    );

    assert_any_lower(
        "paid object route checks admission proof",
        &paid_object,
        &["paid", "wallet", "receipt", "verifier", "proof"],
    );

    assert_any_lower(
        "paid write policy binds request context",
        &paid_write,
        &[
            "paid_storage_context_idem",
            "context idem",
            "context-idem",
            "idempotency",
        ],
    );

    assert_any_lower(
        "paid estimate remains read-only pricing",
        &paid_estimate,
        &["estimate", "bytes", "price", "amount"],
    );

    assert_any_lower(
        "storage economics uses integer policy pricing",
        &economics,
        &["ron_policy", "roc-economics", "paid_storage_put", "price"],
    );

    assert_any_lower(
        "storage settlement uses wallet front-door language",
        &settlement,
        &["wallet", "capture", "release", "settlement"],
    );

    assert_any_lower(
        "storage accounting exporter emits metering only",
        &accounting,
        &["usage", "accounting", "export", "idempotency"],
    );

    assert_none_lower(
        "svc-storage production source forbidden direct authority markers",
        &format!(
            "{storage_mod}\n{cas}\n{paid_object}\n{paid_estimate}\n{paid_write}\n{settlement}\n{economics}\n{accounting}"
        ),
        &[
            "ron_ledger::",
            "svc_wallet::",
            "ledger_mutate(",
            "wallet_mutate(",
            "mutate_ledger(",
            "mutate_wallet(",
            "create_receipt(",
            "insert_receipt(",
            "fake_receipt(",
            "fake_balance(",
            "fake_finality(",
            "cache_only_unlock(",
            "unlock_from_cache(",
            "grant_paid_access(",
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
fn existing_storage_regressions_are_wired_into_stabilization_surface() {
    let paid_artifact = read_rel("tests/internal_roc_beta_paid_content_artifact_boundary.rs");
    let paid_economics = read_rel("tests/paid_write_economics.rs");
    let paid_estimate = read_rel("tests/paid_write_estimate.rs");
    let paid_verifier = read_rel("tests/paid_write_verifier.rs");
    let paid_settlement = read_rel("tests/paid_write_settlement.rs");
    let wallet_mode = read_rel("tests/paid_write_wallet_mode.rs");
    let accounting_export = read_rel("tests/paid_write_accounting_export.rs");
    let web3_loop = read_rel("tests/web3_paid_storage_loop.rs");
    let b3_integrity = read_rel("tests/quickchain_preflight_b3_integrity.rs");
    let paid_cache = read_rel("tests/quickchain_preflight_paid_cache.rs");
    let no_direct_mutation = read_rel("tests/quickchain_preflight_no_direct_mutation.rs");
    let value_loop = read_rel("tests/quickchain_preflight_value_loop_boundary.rs");

    assert_loaded_boundary_regression(
        "paid-content artifact boundary regression",
        &paid_artifact,
        "paid",
    );
    assert_loaded_boundary_regression(
        "paid-write economics regression",
        &paid_economics,
        "economics",
    );
    assert_loaded_boundary_regression("paid-write estimate regression", &paid_estimate, "estimate");
    assert_loaded_boundary_regression("paid-write verifier regression", &paid_verifier, "receipt");
    assert_loaded_boundary_regression(
        "paid-write settlement regression",
        &paid_settlement,
        "settlement",
    );
    assert_loaded_boundary_regression("paid-write wallet-mode regression", &wallet_mode, "wallet");
    assert_loaded_boundary_regression(
        "paid-write accounting export regression",
        &accounting_export,
        "accounting",
    );
    assert_loaded_boundary_regression("web3 paid storage loop regression", &web3_loop, "wallet");
    assert_loaded_boundary_regression("b3 integrity regression", &b3_integrity, "b3");
    assert_loaded_boundary_regression("paid cache regression", &paid_cache, "cache");
    assert_loaded_boundary_regression(
        "no direct mutation regression",
        &no_direct_mutation,
        "mutation",
    );
    assert_loaded_boundary_regression("value loop boundary regression", &value_loop, "wallet");
}

#[test]
fn storage_artifact_contract_rejects_authority_poison_fields() {
    for field in [
        "wallet_mutation_authority",
        "ledger_mutation_authority",
        "receipt_truth",
        "balance_truth",
        "entitlement_truth",
        "paid_unlock_truth",
        "finality_truth",
        "cache_unlock_authority",
        "b3_unlock_authority",
        "manifest_unlock_authority",
        "policy_unlock_authority",
        "accounting_balance_truth",
        "metering_payout_truth",
        "bridge_authority",
        "external_settlement_authority",
        "rox_solana_authority",
        "staking_authority",
        "liquidity_authority",
        "raw_engagement_mint_authority",
    ] {
        let mut value = json!({
            "schema": "svc-storage.internal-roc-stabilization-b3-artifact-non-authority.v1",
            "source_crate": "svc-storage",
            "role": "b3_bytes_artifact_storage_and_paid_admission_participant",
            "b3_role": "content_identity_only_not_receipt_or_unlock",
            "paid_admission_role": "backend_wallet_receipt_evidence_checked_not_created",
            "wallet_role": "svc-wallet_only_for_capture_release",
            "ledger_truth": "ron-ledger_only_after_svc-wallet_accepts",
            "accounting_role": "metering_export_only_not_balance_or_payout_truth"
        });

        value
            .as_object_mut()
            .expect("contract object")
            .insert(field.to_owned(), json!(true));

        let err = serde_json::from_value::<StorageArtifactNonAuthorityContract>(value)
            .expect_err("authority poison fields must reject");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn stabilization_doc_states_storage_product_beta_truth_boundary() {
    let doc = read_rel("docs/internal-roc-stabilization-b3-artifact-non-authority.md");

    assert_all(
        "svc-storage stabilization doc",
        &doc,
        &[
            "b3 proves bytes only",
            "ETag proves content only",
            "storage admission is not finality",
            "metering is not balance truth",
            "svc-wallet remains mutation front-door",
            "ron-ledger remains durable truth",
            "stored bytes as paid proof",
            "b3 hash as receipt proof",
            "cache hit as entitlement proof",
            "usage event as balance proof",
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
