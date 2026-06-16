//! RO:WHAT — QuickChain Phase-0 no-direct-mutation boundary tests for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/SEC. Rewarder plans payouts and targets wallet egress; it must not expose ledger/root/bridge authority.
//! RO:INTERACTS — config DTOs, HTTP router, reward manifest, settlement intent planning, wallet preview DTOs.
//! RO:INVARIANTS — no direct ledger mutation routes; no bridge/anchor/validator config; no fake receipts/balances/finality.
//! RO:METRICS — none; pure Phase-0 boundary tests.
//! RO:CONFIG — proves deny_unknown_fields rejects external settlement authority knobs.
//! RO:SECURITY — prevents accidental settlement/chain authority creep into rewarder.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation.

use axum::body::Body;
use axum::http::Request;
use serde_json::json;
use svc_rewarder::config::Config;
use svc_rewarder::core::{compute_manifest, AmountMinor, ComputeInput};
use svc_rewarder::http::routes::router;
use svc_rewarder::http::RewarderState;
use svc_rewarder::inputs::{
    canonical_snapshot_cid, AccountContribution, AccountingSnapshot, ContentCid, RewardPolicy,
};
use svc_rewarder::outputs::{IntentResult, SettlementBatch};
use tower::ServiceExt;

const POLICY_ID: &str = "policy:v1";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const IDEMPOTENCY_SALT: &str = "svc-rewarder|quickchain-preflight";

fn snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: "acct_a".into(),
                bytes_stored: 100,
                bytes_served: 50,
                uptime_seconds: 10,
            },
            AccountContribution {
                account: "acct_b".into(),
                bytes_stored: 200,
                bytes_served: 0,
                uptime_seconds: 20,
            },
        ],
    }
}

fn inputs_cid_for(snapshot: AccountingSnapshot) -> ContentCid {
    let cid = canonical_snapshot_cid(snapshot).expect("canonical snapshot cid");
    ContentCid::parse(cid).expect("canonical cid parses")
}

fn manifest() -> svc_rewarder::outputs::RewardManifest {
    let snapshot = snapshot();
    compute_manifest(
        ComputeInput {
            epoch_id: "epoch-qc-no-direct-mutation".into(),
            inputs_cid: inputs_cid_for(snapshot.clone()),
            policy: RewardPolicy::dev_default(POLICY_ID, POLICY_HASH),
            snapshot,
            dry_run: false,
            idempotency_salt: IDEMPOTENCY_SALT.into(),
        },
        IntentResult::Accepted,
    )
    .expect("manifest computes")
}

#[test]
fn config_rejects_external_settlement_bridge_anchor_validator_and_root_knobs() {
    for raw in [
        r#"external_settlement = true"#,
        r#"bridge_enabled = true"#,
        r#"anchor_base_url = "http://127.0.0.1:9999""#,
        r#"validator_set = "local-dev""#,
        r#"quickchain = { roots_enabled = true }"#,
        r#"[rewarder]
root_production_enabled = true
"#,
        r#"[rewarder]
checkpoint_writer_enabled = true
"#,
        r#"[ingress]
bridge_base_url = "http://127.0.0.1:9999"
"#,
        r#"[ingress]
anchor_base_url = "http://127.0.0.1:9999"
"#,
        r#"[ingress]
validator_rpc_url = "http://127.0.0.1:9999"
"#,
        r#"[pq]
validator_set = "qc-dev"
"#,
        r#"[shard]
validators = 4
"#,
    ] {
        let err = toml::from_str::<Config>(raw)
            .expect_err("external settlement/chain authority config must be rejected");

        assert!(
            err.to_string().contains("unknown field"),
            "expected unknown-field rejection for config snippet:\n{raw}\nerr={err}"
        );
    }
}

#[tokio::test]
async fn router_does_not_expose_direct_wallet_ledger_quickchain_or_bridge_mutation_routes() {
    let state = RewarderState::new(Config::default()).expect("state builds");
    let app = router(state);

    for uri in [
        "/v1/issue",
        "/wallet/issue",
        "/wallet/transfer",
        "/wallet/burn",
        "/ledger/issue",
        "/ledger/transfer",
        "/ledger/burn",
        "/ledger/hold",
        "/ledger/capture",
        "/ledger/release",
        "/ledger/append",
        "/quickchain/root",
        "/quickchain/checkpoint",
        "/quickchain/validator",
        "/quickchain/settle",
        "/bridge/anchor",
        "/bridge/settle",
        "/anchors",
        "/validators",
    ] {
        let req = Request::builder()
            .method("POST")
            .uri(uri)
            .body(Body::empty())
            .expect("request builds");

        let res = app
            .clone()
            .oneshot(req)
            .await
            .expect("router should respond");

        assert!(
            !res.status().is_success(),
            "svc-rewarder must not expose direct mutation/chain authority route {uri}; got {}",
            res.status()
        );
    }
}

#[test]
fn planning_outputs_do_not_claim_receipts_balances_operation_truth_roots_or_finality() {
    let manifest = manifest();
    let settlement = SettlementBatch::from_manifest(&manifest).expect("settlement plans");
    let wallet_batch = settlement.to_wallet_issue_batch();

    let encoded_manifest = serde_json::to_string(&manifest).expect("manifest serializes");
    let encoded_settlement = serde_json::to_string(&settlement).expect("settlement serializes");
    let encoded_wallet_batch =
        serde_json::to_string(&wallet_batch).expect("wallet batch serializes");

    assert!(encoded_manifest.contains(r#""funding_source":"protocol_pool""#));
    assert!(encoded_settlement.contains(r#""funding_source":"protocol_pool""#));
    assert!(encoded_wallet_batch.contains(r#""funding_source":"protocol_pool""#));

    for request in &wallet_batch.requests {
        let encoded_request = serde_json::to_string(request).expect("request serializes");
        assert!(
            !encoded_request.contains("funding_source"),
            "wallet issue request shape must not receive rewarder funding metadata"
        );
    }

    let combined = format!("{encoded_manifest}\n{encoded_settlement}\n{encoded_wallet_batch}");

    for forbidden in [
        "operation_id",
        "account_sequence",
        "hold_id",
        "state_root",
        "receipt_root",
        "checkpoint_hash",
        "validator_signature",
        "validator_set",
        "bridge_authorized",
        "anchor_authorized",
        "external_settlement",
        "funding_receipt",
        "funding_finalized",
        "ledger_receipt",
        "wallet_receipt",
        "receipt_hash",
        "txid",
        "balance_minor",
        "available_minor",
        "held_minor",
        "finalized",
        "anchored",
        "protocol_minted",
        "mint_authorized",
    ] {
        assert!(
            !combined.contains(forbidden),
            "rewarder planning outputs must not claim forbidden authority/finality field {forbidden}"
        );
    }
}

#[test]
fn compute_request_still_rejects_direct_mutation_authority_smuggling() {
    let snapshot = serde_json::to_value(snapshot()).expect("snapshot serializes");
    let parsed_snapshot =
        serde_json::from_value::<AccountingSnapshot>(snapshot.clone()).expect("snapshot parses");
    let inputs_cid = canonical_snapshot_cid(parsed_snapshot).expect("snapshot cid");

    for forbidden_field in [
        "ledger_mutation",
        "wallet_mutation",
        "direct_issue",
        "direct_transfer",
        "direct_burn",
        "direct_hold",
        "direct_capture",
        "direct_release",
        "operation_id",
        "account_sequence",
        "state_root",
        "receipt_root",
        "checkpoint_hash",
        "validator_signature",
        "bridge_anchor",
        "external_settlement",
    ] {
        let body = json!({
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
            },
            forbidden_field: true
        });

        let err = serde_json::from_value::<svc_rewarder::http::dto::ComputeEpochRequest>(body)
            .expect_err("direct mutation authority field must be rejected");

        assert!(
            err.to_string().contains("unknown field"),
            "{forbidden_field} should fail as unknown, got {err}"
        );
    }
}
