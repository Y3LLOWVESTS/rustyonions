//! RO:WHAT — QuickChain Phase-0 funding-source boundary tests for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/SEC. Reward plans need explicit funding provenance.
//! RO:INTERACTS — inputs::RewardPolicy, http::dto::ComputeEpochRequest, outputs::RewardManifest.
//! RO:INVARIANTS — funding source is policy input; rewarder still does not mutate ledger or mint receipts.
//! RO:METRICS — none; pure DTO and output-boundary test.
//! RO:CONFIG — uses dev policy defaults and deterministic test salt.
//! RO:SECURITY — prevents raw engagement or implicit protocol minting from becoming reward authority.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_preflight_funding_source.

use serde_json::{json, Value};
use svc_rewarder::core::{compute_manifest, AmountMinor, ComputeInput};
use svc_rewarder::http::dto::ComputeEpochRequest;
use svc_rewarder::inputs::{
    canonical_snapshot_cid, validate_reward_policy, AccountContribution, AccountingSnapshot,
    ContentCid, RewardFundingSource, RewardPolicy,
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
                account: "acct_storage_a".into(),
                bytes_stored: 100,
                bytes_served: 40,
                uptime_seconds: 10,
            },
            AccountContribution {
                account: "acct_storage_b".into(),
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

fn valid_compute_body_with_policy(policy: Value) -> Value {
    let snapshot = serde_json::to_value(snapshot()).expect("snapshot serializes");
    let parsed_snapshot =
        serde_json::from_value::<AccountingSnapshot>(snapshot.clone()).expect("snapshot parses");
    let inputs_cid = canonical_snapshot_cid(parsed_snapshot).expect("snapshot cid");

    json!({
        "inputs_cid": inputs_cid,
        "policy_id": POLICY_ID,
        "policy_hash": POLICY_HASH,
        "dry_run": true,
        "snapshot": snapshot,
        "policy": policy
    })
}

fn base_policy_json() -> Value {
    json!({
        "id": POLICY_ID,
        "hash": POLICY_HASH,
        "signed": true,
        "funding_source": "protocol_pool",
        "max_payout_minor_units": "1000",
        "min_payout_minor_units": "1",
        "weight_bps": 10000,
        "rounding": "floor"
    })
}

fn manifest() -> svc_rewarder::outputs::RewardManifest {
    let snapshot = snapshot();
    compute_manifest(
        ComputeInput {
            epoch_id: "epoch-qc-funding-source".into(),
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
fn policy_requires_explicit_funding_source_on_wire() {
    let mut policy = base_policy_json();
    policy.as_object_mut().unwrap().remove("funding_source");

    let err = serde_json::from_value::<RewardPolicy>(policy)
        .expect_err("funding_source must be explicit on policy wire DTOs");

    assert!(
        err.to_string().contains("missing field") && err.to_string().contains("funding_source"),
        "unexpected missing funding_source error: {err}"
    );
}

#[test]
fn current_policy_accepts_explicit_protocol_pool_and_rejects_smuggled_authority_fields() {
    let parsed = serde_json::from_value::<RewardPolicy>(base_policy_json())
        .expect("explicit funding_source policy parses");

    assert_eq!(parsed.funding_source, RewardFundingSource::ProtocolPool);

    for forbidden_field in [
        "implicit_protocol_mint",
        "mint_from_engagement",
        "protocol_reward_authority",
        "settlement_funding_source",
        "external_anchor_source",
        "bridge_source",
    ] {
        let mut policy = base_policy_json();
        policy[forbidden_field] = json!("protocol_pool");

        let err = serde_json::from_value::<RewardPolicy>(policy)
            .expect_err("unknown funding authority field must be rejected");

        assert!(
            err.to_string().contains("unknown field"),
            "{forbidden_field} should fail as unknown, got {err}"
        );
    }
}

#[test]
fn unsigned_protocol_pool_policy_is_rejected_by_validator() {
    let mut policy = RewardPolicy::dev_default(POLICY_ID, POLICY_HASH);
    policy.signed = false;
    policy.funding_source = RewardFundingSource::ProtocolPool;

    let err = validate_reward_policy(&policy, POLICY_ID, POLICY_HASH)
        .expect_err("protocol_pool funding requires signed policy");

    assert_eq!(err.reason(), "bad_request");
    assert!(err.to_string().contains("requires signed policy"));
}

#[test]
fn compute_request_rejects_top_level_funding_authority_smuggling() {
    for forbidden_field in [
        "funding_source",
        "settlement_funding_source",
        "protocol_pool_authorized",
        "mint_authorized",
        "bridge_authorized",
        "anchor_authorized",
    ] {
        let mut body = valid_compute_body_with_policy(base_policy_json());
        body[forbidden_field] = json!("protocol_pool");

        let err = serde_json::from_value::<ComputeEpochRequest>(body)
            .expect_err("top-level funding authority field must be rejected");

        assert!(
            err.to_string().contains("unknown field"),
            "{forbidden_field} should fail as unknown, got {err}"
        );
    }
}

#[test]
fn manifest_carries_funding_provenance_but_not_funding_finality() {
    let manifest = manifest();
    let encoded = serde_json::to_string(&manifest).expect("manifest serializes");

    assert_eq!(
        manifest.policy.funding_source,
        RewardFundingSource::ProtocolPool
    );
    assert!(encoded.contains(r#""funding_source":"protocol_pool""#));
    assert!(encoded.contains(r#""ledger":{"emitted":false,"result":"dry_run"}"#));

    for forbidden in [
        "funding_receipt",
        "funding_finalized",
        "protocol_minted",
        "mint_authorized",
        "settlement_funding_source",
        "bridge_authorized",
        "anchor_authorized",
        "external_settlement",
        "state_root",
        "receipt_root",
        "checkpoint_hash",
        "validator_signature",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "manifest must not claim forbidden funding/finality field {forbidden}"
        );
    }
}

#[test]
fn wallet_preview_carries_batch_provenance_but_requests_remain_wallet_issue_shape() {
    let manifest = manifest();
    let settlement = SettlementBatch::from_manifest(&manifest).expect("settlement plans");
    let wallet_batch = settlement.to_wallet_issue_batch();
    let encoded_batch = serde_json::to_string(&wallet_batch).expect("wallet batch serializes");

    assert_eq!(settlement.funding_source, RewardFundingSource::ProtocolPool);
    assert_eq!(
        wallet_batch.funding_source,
        RewardFundingSource::ProtocolPool
    );
    assert!(encoded_batch.contains(r#""funding_source":"protocol_pool""#));
    assert_eq!(wallet_batch.wallet_path, "/v1/issue");
    assert_eq!(wallet_batch.requests.len(), manifest.payouts.len());

    for request in &wallet_batch.requests {
        assert_eq!(request.asset, "roc");
        assert!(request.amount_minor.parse::<u128>().expect("amount") > 0);
        assert!(request
            .idempotency_key
            .as_deref()
            .expect("idempotency key")
            .starts_with("b3:"));

        let encoded_request = serde_json::to_string(request).expect("request serializes");
        assert!(
            !encoded_request.contains("funding_source"),
            "wallet issue request must not receive rewarder funding metadata"
        );
    }

    for forbidden in [
        "funding_receipt",
        "funding_finalized",
        "protocol_minted",
        "mint_authorized",
        "ledger_receipt",
        "wallet_receipt",
        "balance_minor",
        "state_root",
        "receipt_root",
        "checkpoint_hash",
    ] {
        assert!(
            !encoded_batch.contains(forbidden),
            "wallet preview must not claim forbidden funding/finality field {forbidden}"
        );
    }
}
