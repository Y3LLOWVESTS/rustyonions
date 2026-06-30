//! RO:WHAT — Internal ROC Beta Phase 3 Round 2 approved-payout accounting observation boundary tests.
//! RO:WHY — Proves ron-accounting observes executed wallet/ledger payout receipts as derivative snapshot material only, without payout execution, wallet mutation, ledger mutation, balance truth, receipt truth, finality, bridge, staking, liquidity, or external settlement.
//! RO:INTERACTS — UsageEvent, EventIngestPolicy, Recorder, SealedSlice, RewardSnapshotExport.
//! RO:INVARIANTS — accounting observes after wallet/ledger truth only; accounting snapshots are not balances or receipts; raw/planned payout material cannot execute.
//! RO:METRICS — none.
//! RO:CONFIG — EventIngestPolicy::default, fixed test windows.
//! RO:SECURITY — rejects authority-smuggling fields.
//! RO:TEST — cargo test -p ron-accounting --test internal_roc_beta_phase3_approved_payout_observation_boundary.

use ron_accounting::{
    record_usage_events, Dimension, EventIngestPolicy, MetricKind, Recorder,
    RewardContributionExport, RewardSnapshotExport, SliceId, UsageEvent, Window,
};
use serde_json::{json, Value};

const TENANT: u128 = 313;
const CREATOR_A: &str = "acct_phase3_round2_creator_a";
const CREATOR_B: &str = "acct_phase3_round2_creator_b";

const FORBIDDEN_AUTHORITY_KEYS: &[&str] = &[
    "balance",
    "balance_minor",
    "available_balance",
    "wallet_balance",
    "ledger_balance",
    "wallet_mutation",
    "ledger_mutation",
    "wallet_side_effect",
    "ledger_side_effect",
    "wallet_receipt",
    "ledger_receipt",
    "receipt_truth",
    "balance_truth",
    "payout_execution",
    "payout_side_effect",
    "payout_receipt_txid",
    "reward_plan_id",
    "reward_plan_root",
    "accounting_snapshot_root",
    "policy_receipt",
    "rewarder_receipt",
    "accounting_receipt",
    "client_receipt",
    "issue",
    "transfer",
    "burn",
    "hold",
    "capture",
    "release",
    "finality",
    "client_finality_claim",
    "bridge_txid",
    "solana_signature",
    "rox_settlement_id",
    "staking_position_id",
    "staking_yield_bps",
    "liquidity_pool_id",
    "exchange_order_id",
    "outside_settlement_claim",
];

fn approved_payout_receipt_event(
    timestamp_ms: u64,
    account: &str,
    amount_minor: u64,
) -> UsageEvent {
    UsageEvent::new(
        timestamp_ms,
        TENANT,
        account,
        MetricKind::Custom("payout_ok".to_owned()),
        amount_minor,
    )
    .with_source_service("svc-wallet")
    .with_region("internal")
    .with_route("/internal-roc/approved-payout/wallet-receipt")
}

fn planned_payout_candidate_event(timestamp_ms: u64, account: &str, amount: u64) -> UsageEvent {
    UsageEvent::new(
        timestamp_ms,
        TENANT,
        account,
        MetricKind::Custom("plan".to_owned()),
        amount,
    )
    .with_source_service("svc-rewarder")
    .with_region("internal")
    .with_route("/internal-roc/reward-plan/payout-candidate")
}

fn assert_no_authority_keys(value: &Value) {
    match value {
        Value::Object(object) => {
            for key in object.keys() {
                assert!(
                    !FORBIDDEN_AUTHORITY_KEYS.contains(&key.as_str()),
                    "accounting payout observation artifact must not expose authority key `{key}`"
                );
            }

            for nested in object.values() {
                assert_no_authority_keys(nested);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_authority_keys(nested);
            }
        }
        _ => {}
    }
}

#[test]
fn executed_payout_receipts_become_accounting_snapshot_input_only() {
    let recorder = Recorder::default();
    let event = approved_payout_receipt_event(1_900_300_000_000, CREATOR_A, 77);

    let report = record_usage_events(&recorder, &[event], &EventIngestPolicy::default())
        .expect("approved payout receipt observation should record");

    assert_eq!(report.inspected, 1);
    assert_eq!(report.recorded, 1);
    assert_eq!(report.skipped_zero, 0);

    let rows = recorder.snapshot();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key.labels.tenant, TENANT);
    assert_eq!(rows[0].key.labels.service, CREATOR_A);
    assert_eq!(rows[0].key.labels.region, "internal");
    assert_eq!(rows[0].key.labels.method, "PAYOUT_OK");
    assert!(
        rows[0].key.labels.route.starts_with("/internal-roc/"),
        "payout observation must retain internal ROC route context"
    );
    assert_eq!(rows[0].key.dimension, Dimension::Requests);
    assert_eq!(rows[0].value, 77);

    let slice = recorder
        .seal_slice(
            SliceId {
                tenant: TENANT,
                dimension: Dimension::Requests,
                seq: 1,
            },
            Window::for_timestamp_ms(1_900_300_000_000, 300).expect("valid window"),
            None,
            true,
        )
        .expect("approved payout observation slice should seal");

    assert_eq!(slice.rows.len(), 1);
    assert!(slice.digest().starts_with("b3:"));

    let json = serde_json::to_value(&slice).expect("sealed slice should serialize");
    assert_no_authority_keys(&json);
}

#[test]
fn payout_receipt_observation_rows_are_deterministic_regardless_input_order() {
    let forward = vec![
        approved_payout_receipt_event(1_900_300_000_001, CREATOR_A, 13),
        approved_payout_receipt_event(1_900_300_000_002, CREATOR_B, 29),
    ];

    let mut reverse = forward.clone();
    reverse.reverse();

    let forward_recorder = Recorder::default();
    let reverse_recorder = Recorder::default();

    record_usage_events(&forward_recorder, &forward, &EventIngestPolicy::default())
        .expect("forward payout observations should record");
    record_usage_events(&reverse_recorder, &reverse, &EventIngestPolicy::default())
        .expect("reverse payout observations should record");

    let forward_slice = forward_recorder
        .seal_slice(
            SliceId {
                tenant: TENANT,
                dimension: Dimension::Requests,
                seq: 2,
            },
            Window::for_timestamp_ms(1_900_300_000_000, 300).expect("valid window"),
            None,
            true,
        )
        .expect("forward slice should seal");

    let reverse_slice = reverse_recorder
        .seal_slice(
            SliceId {
                tenant: TENANT,
                dimension: Dimension::Requests,
                seq: 2,
            },
            Window::for_timestamp_ms(1_900_300_000_000, 300).expect("valid window"),
            None,
            true,
        )
        .expect("reverse slice should seal");

    assert_eq!(
        forward_slice.digest(),
        reverse_slice.digest(),
        "same accepted payout observations should seal deterministically regardless input order"
    );
    assert_eq!(forward_slice.rows, reverse_slice.rows);
}

#[test]
fn planned_payout_candidates_are_not_executed_wallet_receipt_observations() {
    let recorder = Recorder::default();
    let events = vec![
        planned_payout_candidate_event(1_900_300_000_010, CREATOR_A, 999),
        approved_payout_receipt_event(1_900_300_000_011, CREATOR_A, 37),
    ];

    let report = record_usage_events(&recorder, &events, &EventIngestPolicy::default())
        .expect("events should record as distinct derivative accounting lanes");

    assert_eq!(report.inspected, 2);
    assert_eq!(report.recorded, 2);

    let rows = recorder.snapshot();
    assert_eq!(rows.len(), 2);

    let mut saw_plan = false;
    let mut saw_receipt_observation = false;

    for row in rows {
        match row.key.labels.method.as_str() {
            "PLAN" => {
                saw_plan = true;
                assert_eq!(row.key.labels.service, CREATOR_A);
                assert!(
                    row.key.labels.route.contains("reward-plan"),
                    "planned payout candidate should retain reward-plan route context"
                );
                assert_eq!(row.value, 999);
            }
            "PAYOUT_OK" => {
                saw_receipt_observation = true;
                assert_eq!(row.key.labels.service, CREATOR_A);
                assert!(
                    row.key.labels.route.contains("wallet-receipt"),
                    "executed payout observation should retain wallet-receipt route context"
                );
                assert_eq!(row.value, 37);
            }
            other => panic!("unexpected payout observation method label: {other}"),
        }
    }

    assert!(saw_plan);
    assert!(saw_receipt_observation);
}

#[test]
fn usage_event_rejects_client_supplied_payout_execution_authority() {
    let clean = serde_json::to_value(approved_payout_receipt_event(
        1_900_300_000_020,
        CREATOR_A,
        17,
    ))
    .expect("usage event should serialize");

    for forbidden in FORBIDDEN_AUTHORITY_KEYS.iter().copied().chain([
        "event_class",
        "source_event_class",
        "economic_receipt",
        "reward_material",
        "wallet_accepted",
        "ledger_accepted",
        "operation_id",
        "account_sequence",
    ]) {
        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("usage event JSON should be object")
            .insert(forbidden.to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<UsageEvent>(poisoned).is_err(),
            "UsageEvent must reject payout authority-smuggling field `{forbidden}`"
        );
    }
}

#[test]
fn reward_snapshot_export_for_payout_observations_stays_planning_material() {
    let snapshot = RewardSnapshotExport::new(
        1_900_300_000_030,
        "106",
        vec![
            RewardContributionExport::new(CREATOR_A, 77, 0, 0),
            RewardContributionExport::new(CREATOR_B, 29, 0, 0),
        ],
    )
    .expect("valid reward snapshot export should construct");

    let canonical = snapshot
        .canonicalized()
        .expect("reward snapshot should canonicalize");

    assert_eq!(canonical.contribution_count(), 2);
    assert_eq!(
        canonical
            .pool_minor_units_as_u128()
            .expect("pool must parse as integer minor units"),
        106
    );

    let cid = canonical
        .canonical_cid()
        .expect("snapshot CID should compute");
    assert!(cid.starts_with("b3:"));
    assert_eq!(cid.len(), 67);

    let json = serde_json::to_value(&canonical).expect("snapshot should serialize");
    assert_no_authority_keys(&json);

    for forbidden in FORBIDDEN_AUTHORITY_KEYS {
        let mut poisoned = json.clone();
        poisoned
            .as_object_mut()
            .expect("snapshot JSON should be object")
            .insert((*forbidden).to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<RewardSnapshotExport>(poisoned).is_err(),
            "RewardSnapshotExport must reject payout authority field `{forbidden}`"
        );
    }
}
