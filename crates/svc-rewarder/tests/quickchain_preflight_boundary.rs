//! RO:WHAT — QuickChain Phase-0 boundary tests for svc-rewarder DTO/output authority.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/SEC. Rewarder must stay payout-planning-only.
//! RO:INTERACTS — http::dto, inputs, core, outputs.
//! RO:INVARIANTS — no roots; no fake receipts; no balances; no finality; integer money strings only.
//! RO:METRICS — none; pure DTO/output boundary tests.
//! RO:CONFIG — uses dev policy defaults and deterministic test salt.
//! RO:SECURITY — rejects smuggled authority fields before they can influence reward plans.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_preflight_boundary.

use serde_json::{json, Value};
use svc_rewarder::core::{compute_manifest, AmountMinor, ComputeInput};
use svc_rewarder::http::dto::ComputeEpochRequest;
use svc_rewarder::inputs::{
    canonical_snapshot_cid, AccountContribution, AccountingSnapshot, ContentCid, RewardPolicy,
};
use svc_rewarder::outputs::{IntentResult, SettlementBatch};

const POLICY_ID: &str = "policy:v1";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const IDEMPOTENCY_SALT: &str = "svc-rewarder|quickchain-preflight";

fn snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: "acct_b".into(),
                bytes_stored: 200,
                bytes_served: 0,
                uptime_seconds: 20,
            },
            AccountContribution {
                account: "acct_a".into(),
                bytes_stored: 100,
                bytes_served: 50,
                uptime_seconds: 10,
            },
        ],
    }
}

fn snapshot_value() -> Value {
    serde_json::to_value(snapshot()).expect("snapshot serializes")
}

fn inputs_cid_for(snapshot: AccountingSnapshot) -> ContentCid {
    let cid = canonical_snapshot_cid(snapshot).expect("canonical snapshot cid");
    ContentCid::parse(cid).expect("canonical cid parses")
}

fn valid_compute_request_json() -> Value {
    let snapshot = snapshot_value();
    let parsed_snapshot =
        serde_json::from_value::<AccountingSnapshot>(snapshot.clone()).expect("snapshot parses");
    let inputs_cid = canonical_snapshot_cid(parsed_snapshot).expect("snapshot cid");

    json!({
        "inputs_cid": inputs_cid,
        "policy_id": POLICY_ID,
        "policy_hash": POLICY_HASH,
        "dry_run": true,
        "snapshot": snapshot,
        "policy": {
            "id": POLICY_ID,
            "hash": POLICY_HASH,
            "signed": true,
            "funding_source": "protocol_pool",
            "max_payout_minor_units": "1000",
            "min_payout_minor_units": "1",
            "weight_bps": 10000,
            "rounding": "floor"
        }
    })
}

fn manifest() -> svc_rewarder::outputs::RewardManifest {
    let snapshot = snapshot();
    compute_manifest(
        ComputeInput {
            epoch_id: "epoch-qc-boundary".into(),
            inputs_cid: inputs_cid_for(snapshot.clone()),
            policy: RewardPolicy::dev_default(POLICY_ID, POLICY_HASH),
            snapshot,
            dry_run: true,
            idempotency_salt: IDEMPOTENCY_SALT.into(),
        },
        IntentResult::DryRun,
    )
    .expect("manifest computes")
}

#[test]
fn compute_request_rejects_smuggled_quickchain_authority_fields() {
    let forbidden_fields = [
        (
            "state_root",
            json!("b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ),
        (
            "receipt_root",
            json!("b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ),
        (
            "checkpoint_hash",
            json!("b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ),
        ("validator_signature", json!("sig")),
        ("settlement_status", json!("finalized")),
        ("finalized", json!(true)),
        ("ledger_receipt", json!({"txid":"tx_fake"})),
        ("wallet_balance", json!("1000")),
        ("payout_authorized", json!(true)),
    ];

    for (field, value) in forbidden_fields {
        let mut body = valid_compute_request_json();
        body[field] = value;

        let err = serde_json::from_value::<ComputeEpochRequest>(body)
            .expect_err("unknown authority field must be rejected");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field} should fail as unknown, got {err}"
        );
    }
}

#[test]
fn reward_manifest_does_not_expose_roots_receipts_balances_or_finality() {
    let manifest = manifest();
    let encoded = serde_json::to_string(&manifest).expect("manifest serializes");

    assert!(encoded.contains(r#""run_key":"b3:"#));
    assert!(encoded.contains(r#""commitment":"b3:"#));

    for forbidden in [
        "state_root",
        "receipt_root",
        "accounting_root",
        "reward_root",
        "checkpoint_hash",
        "validator",
        "signature",
        "receipt_hash",
        "txid",
        "balance_minor",
        "available_minor",
        "held_minor",
        "finalized",
        "anchored",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "manifest must not expose forbidden authority field {forbidden}"
        );
    }
}

#[test]
fn wallet_preview_is_issue_request_shape_not_receipt_or_balance_truth() {
    let manifest = manifest();
    let settlement = SettlementBatch::from_manifest(&manifest).expect("settlement plans");
    let wallet_batch = settlement.to_wallet_issue_batch();
    let value = serde_json::to_value(&wallet_batch).expect("wallet batch serializes");
    let encoded = serde_json::to_string(&wallet_batch).expect("wallet batch json");

    assert_eq!(value["run_key"], manifest.run_key);
    assert_eq!(value["manifest_commitment"], manifest.commitment);
    assert_eq!(value["wallet_path"], "/v1/issue");
    assert!(!value["requests"]
        .as_array()
        .expect("requests array")
        .is_empty());

    for request in value["requests"].as_array().expect("requests array") {
        assert!(request["amount_minor"].is_string());
        assert_eq!(request["asset"], "roc");
        assert!(request["idempotency_key"]
            .as_str()
            .expect("idempotency key")
            .starts_with("b3:"));
    }

    for forbidden in [
        "receipt_hash",
        "txid",
        "ledger_receipt",
        "wallet_receipt",
        "balance_minor",
        "available_minor",
        "held_minor",
        "finalized",
        "checkpoint_hash",
        "state_root",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "wallet preview must not expose forbidden authority field {forbidden}"
        );
    }
}

#[test]
fn json_number_money_is_rejected_at_snapshot_wire_boundary() {
    let bad_snapshot = json!({
        "produced_at_millis": 1,
        "pool_minor_units": 1000,
        "contributions": [
            {
                "account": "acct_a",
                "bytes_stored": 100,
                "bytes_served": 0,
                "uptime_seconds": 10
            }
        ]
    });

    let err = serde_json::from_value::<AccountingSnapshot>(bad_snapshot)
        .expect_err("AmountMinor must reject JSON-number money");

    assert!(
        err.to_string().contains("invalid type") || err.to_string().contains("amount"),
        "unexpected error: {err}"
    );
}
