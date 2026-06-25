#![cfg(feature = "quickchain-preflight")]
#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 3 Round 2 validator lifecycle boundary tests for svc-wallet.
//! RO:WHY — Validator rotation, revocation, equivocation, replay challenges, downtime, and governance parameter updates must not become wallet spend, receipt, balance, settlement, staking, slashing, or bridge authority.
//! RO:INTERACTS — wallet request/receipt DTOs, v1 route surface, svc-wallet production source boundary.
//! RO:INVARIANTS — wallet remains the explicit ROC mutation front-door only; lifecycle/evidence metadata cannot authorize issue/transfer/burn/hold/capture/release.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — rejects Phase 3 Round 2 lifecycle authority smuggling and preserves no silent spend / no fake receipt boundaries.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase3_validator_lifecycle_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Map, Value};
use svc_wallet::dto::{
    requests::{AmountMinor, BurnRequest, IssueRequest, TransferRequest},
    responses::{Receipt, ReceiptSettlementStatus, WalletOp},
};

const PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS: &[&str] = &[
    "validator_rotation",
    "validator_rotation_epoch",
    "validator_rotation_decision",
    "validator_revocation",
    "validator_revocation_reason",
    "validator_revocation_decision",
    "validator_lifecycle_decision",
    "validator_lifecycle_status",
    "validator_lifecycle_rejection_code",
    "equivocation_evidence",
    "double_attestation_evidence",
    "split_brain_evidence",
    "replay_challenge_evidence",
    "invalid_attestation_evidence",
    "validator_downtime_status",
    "validator_degraded_status",
    "downtime_report",
    "governance_parameter_update",
    "validator_set_parameter_update",
    "quorum_parameter_update",
    "checkpoint_parameter_update",
    "slash_evidence",
    "slashing",
    "staking_power",
    "validator_bond",
    "bonded_economics",
    "validator_reward",
    "bridge_settlement",
    "external_settlement",
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
}

fn collect_rs_files(root: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|err| {
        panic!("failed to read directory {}: {err}", root.display());
    });

    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();

        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "target")
        {
            continue;
        }

        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "rs")
        {
            files.push(path);
        }
    }
}

fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with("///"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_no_key(value: &Value, forbidden_key: &str) {
    match value {
        Value::Object(map) => {
            assert!(
                !map.contains_key(forbidden_key),
                "wallet JSON value must not contain lifecycle authority key `{forbidden_key}`: {value}"
            );

            for nested in map.values() {
                assert_no_key(nested, forbidden_key);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_key(nested, forbidden_key);
            }
        }
        _ => {}
    }
}

fn issue_body_with(extra_key: &str) -> Value {
    let mut body = Map::new();
    body.insert("to".to_owned(), json!("acct_creator"));
    body.insert("asset".to_owned(), json!("roc"));
    body.insert("amount_minor".to_owned(), json!("10"));
    body.insert(
        "idempotency_key".to_owned(),
        json!("idem_phase3_round2_issue"),
    );
    body.insert(extra_key.to_owned(), json!("client-must-not-supply"));
    Value::Object(body)
}

fn transfer_body_with(extra_key: &str) -> Value {
    let mut body = Map::new();
    body.insert("from".to_owned(), json!("acct_payer"));
    body.insert("to".to_owned(), json!("acct_creator"));
    body.insert("asset".to_owned(), json!("roc"));
    body.insert("amount_minor".to_owned(), json!("10"));
    body.insert("nonce".to_owned(), json!(1_u64));
    body.insert(
        "idempotency_key".to_owned(),
        json!("idem_phase3_round2_transfer"),
    );
    body.insert(extra_key.to_owned(), json!("client-must-not-supply"));
    Value::Object(body)
}

fn burn_body_with(extra_key: &str) -> Value {
    let mut body = Map::new();
    body.insert("from".to_owned(), json!("acct_payer"));
    body.insert("asset".to_owned(), json!("roc"));
    body.insert("amount_minor".to_owned(), json!("10"));
    body.insert("nonce".to_owned(), json!(1_u64));
    body.insert(
        "idempotency_key".to_owned(),
        json!("idem_phase3_round2_burn"),
    );
    body.insert(extra_key.to_owned(), json!("client-must-not-supply"));
    Value::Object(body)
}

fn accepted_wallet_receipt() -> Receipt {
    Receipt {
        txid: "tx_phase3_round2_wallet_receipt".to_owned(),
        op: WalletOp::Transfer,
        from: Some("acct_payer".to_owned()),
        to: Some("acct_creator".to_owned()),
        asset: "roc".to_owned(),
        amount_minor: AmountMinor(10),
        nonce: Some(1),
        idem: "idem_phase3_round2_receipt".to_owned(),
        ts: 1,
        ledger_seq_start: Some(1),
        ledger_seq_end: Some(2),
        ledger_root: "00".repeat(32),
        settlement_status: ReceiptSettlementStatus::Accepted,
        receipt_hash: "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_owned(),
    }
}

#[test]
fn wallet_mutation_dtos_reject_validator_lifecycle_authority_fields() {
    for extra_key in PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS {
        assert!(
            serde_json::from_value::<IssueRequest>(issue_body_with(extra_key)).is_err(),
            "IssueRequest must reject client-supplied Phase 3 Round 2 lifecycle authority field: {extra_key}"
        );

        assert!(
            serde_json::from_value::<TransferRequest>(transfer_body_with(extra_key)).is_err(),
            "TransferRequest must reject client-supplied Phase 3 Round 2 lifecycle authority field: {extra_key}"
        );

        assert!(
            serde_json::from_value::<BurnRequest>(burn_body_with(extra_key)).is_err(),
            "BurnRequest must reject client-supplied Phase 3 Round 2 lifecycle authority field: {extra_key}"
        );
    }
}

#[test]
fn wallet_receipts_remain_accepted_only_not_validator_lifecycle_decisions() {
    let receipt = accepted_wallet_receipt();
    let receipt_value =
        serde_json::to_value(&receipt).expect("wallet receipt should serialize to JSON");

    for forbidden_key in PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS {
        assert_no_key(&receipt_value, forbidden_key);

        let mut poisoned = receipt_value
            .as_object()
            .expect("receipt value should be an object")
            .clone();
        poisoned.insert((*forbidden_key).to_owned(), json!("client-must-not-supply"));

        assert!(
            serde_json::from_value::<Receipt>(Value::Object(poisoned)).is_err(),
            "Receipt must reject lifecycle/finality/staking/slashing authority field: {forbidden_key}"
        );
    }

    assert_eq!(receipt.settlement_status, ReceiptSettlementStatus::Accepted);
}

#[test]
fn wallet_route_surface_stays_explicit_wallet_operations_only() {
    let router_source = read(crate_dir().join("src/routes/v1/mod.rs"));

    for required_route in [
        "\"/balance\"",
        "\"/issue\"",
        "\"/transfer\"",
        "\"/burn\"",
        "\"/hold\"",
        "\"/capture\"",
        "\"/release\"",
        "\"/tx/:txid\"",
    ] {
        assert!(
            router_source.contains(required_route),
            "svc-wallet v1 router must keep explicit wallet route: {required_route}"
        );
    }

    for forbidden_route in [
        "\"/validator",
        "\"/validators",
        "\"/validator-rotation",
        "\"/validator-revocation",
        "\"/equivocation",
        "\"/replay-challenge",
        "\"/downtime",
        "\"/governance-parameter",
        "\"/slash",
        "\"/slashing",
        "\"/staking",
        "\"/bridge",
        "\"/external-settlement",
        "\"/settlement-finality",
    ] {
        assert!(
            !router_source.contains(forbidden_route),
            "svc-wallet route surface must not expose Phase 3 Round 2 validator/economy authority route: {forbidden_route}"
        );
    }
}

#[test]
fn wallet_source_does_not_construct_validator_lifecycle_mutation_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-wallet Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "rotate_validator",
            "revoke_validator",
            "validator_lifecycle_decision",
            "validator_lifecycle_status",
            "equivocation_evidence",
            "double_attestation_evidence",
            "split_brain_evidence",
            "replay_challenge_evidence",
            "downtime_report",
            "validator_degraded_status",
            "governance_parameter_update",
            "slash_validator",
            "stake_validator",
            "validator_reward",
            "mint_from_validator",
            "issue_from_attestation",
            "capture_from_quorum",
            "settle_from_validator",
            "bridge_settlement",
            "external_settlement",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-wallet production source must not construct Phase 3 Round 2 validator lifecycle mutation authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
