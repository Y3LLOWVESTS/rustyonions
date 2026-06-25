#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 3 Round 1 passport-gated validator boundary tests for svc-wallet.
//! RO:WHY — svc-wallet may produce backend-derived ROC receipts, but validator identity, passport admission, registry membership, and validator capability authorization must not become wallet authority.
//! RO:INTERACTS — svc_wallet::quickchain projection DTOs, wallet request DTOs, svc-wallet source/Cargo boundary.
//! RO:INVARIANTS — wallet remains mutation front-door only; no validator admission, passport registry authority, staking, slashing, or validator balance authority.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — rejects Phase 3 validator/passport authority fields and prevents wallet validator-economy creep.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase3_validator_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use svc_wallet::{
    dto::{
        requests::{AmountMinor, BurnRequest, IssueRequest, TransferRequest},
        responses::{Receipt, ReceiptSettlementStatus, WalletOp},
    },
    quickchain::{
        project_wallet_receipt_for_quickchain_preflight, QuickChainWalletReceiptProjection,
        QuickChainWalletReceiptProjectionContext,
    },
    util::blake3_receipt::finalize_receipt,
};

const PHASE3_VALIDATOR_AUTHORITY_KEYS: &[&str] = &[
    "validator_passport_subject",
    "validator_capability",
    "validator_capability_scope",
    "validator_capability_id",
    "validator_set_hash",
    "validator_set_version",
    "validator_registry_epoch",
    "validator_lifecycle_status",
    "validator_admission_rule",
    "validator_revocation_rule",
    "validator_rotation_epoch",
    "passport_registry_proof",
    "passport_admission_proof",
    "passport_revocation_proof",
    "registry_membership_proof",
    "registry_admission_proof",
    "capability_not_before_ms",
    "capability_expires_at_ms",
    "capability_rotation_proof",
    "passport_required",
    "bond_required",
    "bonded_economics",
    "validator_bond",
    "staking_power",
    "slash_evidence",
    "slashing",
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

fn strip_line_comments(input: &str) -> String {
    input
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .collect::<Vec<_>>()
        .join("\n")
}

fn valid_wallet_receipt() -> Receipt {
    finalize_receipt(Receipt {
        txid: "tx_qc_phase3_wallet_validator_boundary".to_string(),
        op: WalletOp::Transfer,
        from: Some("acct_phase3_wallet_alice".to_string()),
        to: Some("acct_phase3_wallet_bob".to_string()),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(17),
        nonce: Some(1),
        idem: "idem_qc_phase3_wallet_validator_boundary".to_string(),
        ts: 1_777_309_851_000,
        ledger_seq_start: Some(30),
        ledger_seq_end: Some(31),
        ledger_root: "22".repeat(32),
        settlement_status: ReceiptSettlementStatus::Accepted,
        receipt_hash: String::new(),
    })
    .expect("valid wallet receipt should finalize")
}

fn valid_projection_value() -> Value {
    let receipt = valid_wallet_receipt();
    let context = QuickChainWalletReceiptProjectionContext::accepted(
        "roc-dev",
        "op:wallet:transfer:phase3-validator-boundary",
    )
    .expect("explicit wallet projection context should validate");

    let projection = project_wallet_receipt_for_quickchain_preflight(&receipt, &context)
        .expect("wallet receipt should project");

    serde_json::to_value(projection).expect("projection should serialize")
}

fn clean_issue_request() -> Value {
    json!({
        "to": "acct_phase3_issue",
        "asset": "roc",
        "amount_minor": "1",
        "idempotency_key": "idem_phase3_issue",
        "memo": null
    })
}

fn clean_transfer_request() -> Value {
    json!({
        "from": "acct_phase3_from",
        "to": "acct_phase3_to",
        "asset": "roc",
        "amount_minor": "1",
        "nonce": 1,
        "idempotency_key": "idem_phase3_transfer",
        "memo": null
    })
}

fn clean_burn_request() -> Value {
    json!({
        "from": "acct_phase3_burn",
        "asset": "roc",
        "amount_minor": "1",
        "nonce": 1,
        "idempotency_key": "idem_phase3_burn",
        "memo": null
    })
}

fn assert_no_phase3_authority_key_recursive(value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                for forbidden in [
                    "validator",
                    "passport",
                    "registry",
                    "capability",
                    "bond",
                    "stake",
                    "slash",
                    "staking",
                    "slashing",
                ] {
                    assert!(
                        !key.contains(forbidden),
                        "wallet projection must not expose Phase 3 validator/passport authority key `{key}`"
                    );
                }
                assert_no_phase3_authority_key_recursive(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_phase3_authority_key_recursive(item);
            }
        }
        _ => {}
    }
}

#[test]
fn wallet_projection_remains_receipt_evidence_not_validator_membership_or_passport_authority() {
    let value = valid_projection_value();

    assert_no_phase3_authority_key_recursive(&value);

    let object = value
        .as_object()
        .expect("wallet projection JSON should be an object");

    for required in [
        "schema",
        "chain_id",
        "operation_id",
        "txid",
        "op",
        "asset",
        "amount_minor",
        "idempotency_key",
        "ledger_seq_start",
        "ledger_seq_end",
        "legacy_ledger_root",
        "receipt_hash",
        "settlement_status",
    ] {
        assert!(
            object.contains_key(required),
            "wallet projection should preserve receipt evidence field: {required}"
        );
    }
}

#[test]
fn wallet_projection_rejects_phase3_validator_and_passport_authority_fields() {
    for field in PHASE3_VALIDATOR_AUTHORITY_KEYS {
        let mut value = valid_projection_value();
        value
            .as_object_mut()
            .expect("projection JSON should be an object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-validator-authority"),
            );

        assert!(
            serde_json::from_value::<QuickChainWalletReceiptProjection>(value).is_err(),
            "wallet projection DTO must reject Phase 3 validator/passport authority field: {field}"
        );
    }
}

#[test]
fn wallet_request_dtos_reject_phase3_validator_and_passport_authority_fields() {
    for field in PHASE3_VALIDATOR_AUTHORITY_KEYS {
        let mut issue = clean_issue_request();
        issue
            .as_object_mut()
            .expect("issue JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-validator-authority"),
            );
        assert!(
            serde_json::from_value::<IssueRequest>(issue).is_err(),
            "IssueRequest must reject Phase 3 validator/passport authority field: {field}"
        );

        let mut transfer = clean_transfer_request();
        transfer
            .as_object_mut()
            .expect("transfer JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-validator-authority"),
            );
        assert!(
            serde_json::from_value::<TransferRequest>(transfer).is_err(),
            "TransferRequest must reject Phase 3 validator/passport authority field: {field}"
        );

        let mut burn = clean_burn_request();
        burn.as_object_mut()
            .expect("burn JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-validator-authority"),
            );
        assert!(
            serde_json::from_value::<BurnRequest>(burn).is_err(),
            "BurnRequest must reject Phase 3 validator/passport authority field: {field}"
        );
    }
}

#[test]
fn wallet_manifest_keeps_passport_registry_and_auth_crates_out_of_authority_path() {
    let manifest = read(crate_dir().join("Cargo.toml"));

    for forbidden in [
        "svc-passport",
        "svc_passport",
        "svc-registry",
        "svc_registry",
        "ron-auth",
        "ron_auth",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "svc-wallet must not link Phase 3 passport/registry/auth authority crate in this round: {forbidden}"
        );
    }
}

#[test]
fn wallet_source_does_not_implement_phase3_validator_or_passport_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-wallet Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path));

        for forbidden in [
            "QuickChainValidator",
            "ValidatorCapability",
            "ValidatorSet",
            "ValidatorAdmission",
            "ValidatorRevocation",
            "validator_set_hash",
            "validator_passport_subject",
            "validator_capability",
            "validator_capability_scope",
            "validator_registry_epoch",
            "passport_admission_proof",
            "passport_revocation_proof",
            "registry_membership_proof",
            "registry_admission_proof",
            "passport_required",
            "bond_required",
            "bonded_economics",
            "validator_bond",
            "staking_power",
            "admit_validator",
            "revoke_validator",
            "rotate_validator",
            "slash_validator",
            "svc_passport",
            "svc_registry",
            "ron_auth::",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-wallet source must not implement Phase 3 validator/passport authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
