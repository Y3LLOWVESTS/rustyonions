//! RO:WHAT — Internal ROC Stabilization receipt DTO boundary tests for ron-proto.
//! RO:WHY — Product beta readiness needs receipt DTOs to remain strict display/transport contracts, not authority.
//! RO:INTERACTS — src/quickchain/receipt.rs, src/quickchain/ids.rs, src/quickchain/money.rs, quickchain receipt/money tests.
//! RO:INVARIANTS — DTO-only; operation_id is durable backend op identity; idempotency_key is retry metadata; money is integer string.
//! RO:SECURITY — no fake receipts, fake balances, fake finality, cache-only unlock, bridge, ROX/Solana, staking, liquidity, or external settlement.
//! RO:TEST — cargo test -p ron-proto --test internal_roc_stabilization_receipt_dto_boundary.

#![allow(clippy::missing_panics_doc)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptDtoTruthContract {
    schema: String,
    source_crate: String,
    dto_role: String,
    receipt_authority: String,
    balance_authority: String,
    operation_id_meaning: String,
    idempotency_key_meaning: String,
    amount_encoding: String,
    accepted_status_meaning: String,
}

#[test]
fn receipt_dto_source_keeps_wallet_ledger_authority_and_strict_identity_fields() {
    let receipt = read_rel("src/quickchain/receipt.rs");

    assert_all(
        "quickchain receipt DTO",
        &receipt,
        &[
            "pub struct QuickChainReceiptV1",
            "pub operation_id: String",
            "pub idempotency_key: String",
            "pub amount_minor: String",
            "pub account_sequence: Option<u64>",
            "pub ledger_seq_start: Option<u64>",
            "pub ledger_seq_end: Option<u64>",
            "validate_operation_id_v1",
            "validate_idempotency_key_v1",
            "validate_quickchain_minor_units",
            "a parsed receipt DTO is not authority",
            "Receipt authority remains with svc-wallet/ron-ledger",
            "does not issue receipts",
            "compute roots",
            "unlock paid content",
            "or mutate ledger",
        ],
    );

    assert_all(
        "receipt status vocabulary",
        &receipt,
        &[
            "pub enum QuickChainReceiptStatusV1",
            "Accepted",
            "EpochIncluded",
            "Finalized",
            "Anchored",
            "is_backend_accepted",
            "is_epoch_included_or_stronger",
            "is_finalized_or_stronger",
        ],
    );

    assert_none(
        "quickchain receipt DTO forbidden active runtime markers",
        &receipt.to_lowercase(),
        &[
            "fake_receipt",
            "fake_balance",
            "cache_only_unlock",
            "unlock_from_cache",
            "wallet_mutate",
            "ledger_mutate",
            "bridge_runtime",
            "rox_runtime",
            "solana_runtime",
            "staking_runtime",
            "liquidity_runtime",
            "external_settlement",
            "mint_rox",
            "burn_rox",
        ],
    );
}

#[test]
fn money_and_identity_regressions_keep_receipts_integer_and_retry_only() {
    let ids_money = read_rel("tests/quickchain_ids_and_money.rs");
    let paid_content = read_rel("tests/internal_roc_beta_paid_content_dto.rs");
    let receipt_test = read_rel("tests/quickchain_receipt_dto.rs");

    assert_all(
        "quickchain ids and money regression",
        &ids_money,
        &[
            "validate_quickchain_minor_units",
            "minor_unit_strings_accept_only_canonical_unsigned_integers",
            "json_numeric_money_rejects_before_validation",
            "\"1.0\"",
            "\"1_000\"",
            "\"1 ROC\"",
            "validate_operation_id_v1",
            "validate_idempotency_key_v1",
        ],
    );

    assert_all(
        "paid content receipt DTO regression",
        &paid_content,
        &[
            "QuickChainReceiptV1",
            "backend-derived receipt labels only",
            "operation_id",
            "idempotency_key",
            "amount_minor",
            "QuickChainReceiptStatusV1::Accepted",
            "QuickChainOperationClassV1::Transfer",
        ],
    );

    assert_all(
        "quickchain receipt DTO regression",
        &receipt_test,
        &[
            "QuickChainReceiptV1",
            "operation_id",
            "idempotency_key",
            "amount_minor",
        ],
    );
}

#[test]
fn receipt_dto_contract_rejects_authority_poison_fields() {
    for field in [
        "fake_receipt",
        "fake_balance",
        "fake_finality",
        "paid_entitlement_authority",
        "cache_unlock_authority",
        "wallet_mutation_authority",
        "ledger_mutation_authority",
        "bridge_authority",
        "external_settlement_authority",
        "client_truth",
    ] {
        let mut value = json!({
            "schema": "ron-proto.internal-roc-stabilization-receipt-dto.v1",
            "source_crate": "ron-proto",
            "dto_role": "strict_transport_display_contract",
            "receipt_authority": "svc-wallet_and_ron-ledger_only",
            "balance_authority": "ron-ledger_only",
            "operation_id_meaning": "backend_durable_ledger_operation_identity",
            "idempotency_key_meaning": "retry_metadata_only_not_authority",
            "amount_encoding": "integer_minor_unit_string_only",
            "accepted_status_meaning": "backend_wallet_ledger_accepted_not_external_finality"
        });

        value
            .as_object_mut()
            .expect("contract object")
            .insert(field.to_owned(), json!(true));

        let err = serde_json::from_value::<ReceiptDtoTruthContract>(value)
            .expect_err("authority poison fields must reject");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn stabilization_receipt_doc_states_dto_only_truth_boundary() {
    let doc = read_rel("docs/internal-roc-stabilization-receipt-dto.md");

    assert_all(
        "receipt DTO stabilization doc",
        &doc,
        &[
            "ron-proto is DTO-only",
            "operation_id = backend durable ledger-operation identity",
            "idempotency_key = retry/dedupe metadata only",
            "amount_minor = integer minor-unit string only",
            "accepted = backend wallet/ledger accepted mutation; not external finality",
            "receipt truth",
            "balance truth",
            "paid entitlement",
        ],
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
