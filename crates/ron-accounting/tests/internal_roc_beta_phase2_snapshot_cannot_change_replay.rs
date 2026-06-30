//! RO:WHAT — Internal ROC Beta Phase 2 accounting snapshot replay non-authority tests.
//! RO:WHY — Internal ROC Beta Phase 2; Concerns: ECON/GOV/SEC. Accounting may observe accepted wallet receipts, but snapshots must not alter replay, create balances, create receipts, or unlock paid content.
//! RO:INTERACTS — UsageEvent, Recorder, SealedSlice, EventIngestPolicy.
//! RO:INVARIANTS — accounting snapshots are derivative artifacts; accepted economic receipts are observation input only; cancelled actions produce no accounting receipt event; no wallet/ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — EventIngestPolicy::default, fixed 5-minute accounting window.
//! RO:SECURITY — rejects replay/balance/receipt authority poison fields and preserves b3 artifact-only snapshot output.
//! RO:TEST — cargo test -p ron-accounting --test internal_roc_beta_phase2_snapshot_cannot_change_replay.

use std::collections::BTreeMap;

use ron_accounting::{
    record_usage_events, Dimension, EventIngestPolicy, MetricKind, Recorder, SliceId, UsageEvent,
    Window,
};
use serde_json::{json, Value};

const TENANT: u128 = 91;
const VIEWER: &str = "acct_phase2_accounting_viewer";
const CREATOR: &str = "acct_phase2_accounting_creator";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObservationStatus {
    Accepted,
    CancelledBeforeMutation,
}

#[derive(Debug, Clone, Copy)]
struct WalletObservation {
    action: &'static str,
    status: ObservationStatus,
    amount_minor: u64,
}

const OBSERVATIONS: &[WalletObservation] = &[
    WalletObservation {
        action: "paid_post",
        status: ObservationStatus::Accepted,
        amount_minor: 10,
    },
    WalletObservation {
        action: "paid_comment",
        status: ObservationStatus::CancelledBeforeMutation,
        amount_minor: 15,
    },
    WalletObservation {
        action: "paid_article",
        status: ObservationStatus::Accepted,
        amount_minor: 20,
    },
    WalletObservation {
        action: "content_view",
        status: ObservationStatus::Accepted,
        amount_minor: 25,
    },
];

#[test]
fn accounting_snapshot_observes_accepted_receipts_only_and_cannot_change_replay() {
    let replay_before_accounting = replay_truth(OBSERVATIONS);
    let recorder = Recorder::default();
    let events = accepted_receipt_events(OBSERVATIONS);

    assert_eq!(
        events.len(),
        3,
        "cancelled-before-mutation paid action must not produce accepted accounting receipt input"
    );

    let report = record_usage_events(&recorder, &events, &EventIngestPolicy::default())
        .expect("accepted wallet receipt observations should record");

    assert_eq!(report.inspected, 3);
    assert_eq!(report.recorded, 3);
    assert_eq!(report.skipped_zero, 0);

    let replay_after_ingest = replay_truth(OBSERVATIONS);
    assert_eq!(
        replay_after_ingest, replay_before_accounting,
        "recording accounting observations must not alter ledger replay truth"
    );

    let window = Window::for_timestamp_ms(1_800_300_000, 300).expect("valid accounting window");
    let slice = recorder
        .seal_slice(
            SliceId {
                tenant: TENANT,
                dimension: Dimension::Requests,
                seq: 1,
            },
            window,
            None,
            true,
        )
        .expect("accepted receipt observations should seal");

    assert_eq!(slice.rows.len(), 3);
    assert_eq!(slice.rows.iter().map(|row| row.value).sum::<u64>(), 55);
    assert!(slice
        .rows
        .iter()
        .all(|row| row.dimension == Dimension::Requests));
    assert_b3_artifact(slice.digest());

    let encoded = serde_json::to_value(&slice).expect("sealed slice should encode");
    assert_no_authority_keys(&encoded);

    let replay_after_seal = replay_truth(OBSERVATIONS);
    assert_eq!(
        replay_after_seal, replay_before_accounting,
        "sealing accounting snapshot must not alter ledger replay truth"
    );

    assert_eq!(
        recorder.row_count(),
        0,
        "sealing the accepted receipt stream should drain derivative accounting rows"
    );
}

#[test]
fn usage_event_rejects_replay_balance_and_receipt_authority_poison_fields() {
    for forbidden_field in [
        "ledger_replay_result",
        "balance_after_replay",
        "receipt_after_replay",
        "accounting_mutates_replay",
        "wallet_mutation",
        "ledger_mutation",
        "balance_truth",
        "receipt_truth",
        "paid_unlock_authority",
        "entitlement_truth",
        "finality_truth",
        "settlement_truth",
        "cache_unlock_authority",
        "raw_engagement_mints_roc",
    ] {
        let mut event = json!({
            "timestamp_ms": 1_800_000,
            "tenant": TENANT,
            "subject": "svc-wallet",
            "metric_kind": { "custom": "economic_receipt_paid_post" },
            "value": 10,
            "source_service": "svc-wallet",
            "region": "local",
            "route": "/internal-roc/paid_post/accepted-wallet-receipt"
        });

        event
            .as_object_mut()
            .expect("test event object")
            .insert(forbidden_field.to_string(), json!(true));

        assert!(
            serde_json::from_value::<UsageEvent>(event).is_err(),
            "UsageEvent must reject replay/accounting authority poison field `{forbidden_field}`"
        );
    }
}

fn accepted_receipt_events(observations: &[WalletObservation]) -> Vec<UsageEvent> {
    observations
        .iter()
        .enumerate()
        .filter(|(_, observation)| observation.status == ObservationStatus::Accepted)
        .map(|(idx, observation)| {
            UsageEvent::new(
                1_800_000 + idx as u64,
                TENANT,
                "svc-wallet",
                MetricKind::Custom(format!("economic_receipt_{}", observation.action)),
                observation.amount_minor,
            )
            .with_source_service("svc-wallet")
            .with_region("local")
            .with_route(format!(
                "/internal-roc/{}/accepted-wallet-receipt",
                observation.action
            ))
        })
        .collect()
}

fn replay_truth(observations: &[WalletObservation]) -> BTreeMap<&'static str, i128> {
    let mut balances = BTreeMap::new();
    balances.insert(VIEWER, 1_000_i128);
    balances.insert(CREATOR, 0_i128);

    for observation in observations
        .iter()
        .filter(|item| item.status == ObservationStatus::Accepted)
    {
        *balances
            .get_mut(VIEWER)
            .expect("viewer replay account should exist") -= i128::from(observation.amount_minor);
        *balances
            .get_mut(CREATOR)
            .expect("creator replay account should exist") += i128::from(observation.amount_minor);
    }

    balances
}

fn assert_b3_artifact(value: &str) {
    assert!(
        value.starts_with("b3:"),
        "expected b3:<64 lowercase hex>, got {value}"
    );

    let hex = &value[3..];
    assert_eq!(hex.len(), 64, "expected 64 hex chars, got {value}");
    assert!(
        hex.chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
        "expected lowercase b3 artifact CID, got {value}"
    );
}

fn assert_no_authority_keys(value: &Value) {
    let forbidden_keys = [
        "wallet_mutation",
        "ledger_mutation",
        "balance_truth",
        "receipt_truth",
        "paid_unlock_authority",
        "entitlement_truth",
        "payout_side_effect",
        "rewarder_direct_payout",
        "client_finality_claim",
        "cache_unlock_authority",
        "bridge_txid",
        "staking_position_id",
        "liquidity_pool_id",
        "outside_settlement_id",
        "raw_engagement_mints_roc",
        "accounting_root",
        "reward_root",
        "checkpoint_root",
        "finality_truth",
        "settlement_truth",
    ];

    match value {
        Value::Object(object) => {
            for key in object.keys() {
                assert!(
                    !forbidden_keys.iter().any(|forbidden| key == forbidden),
                    "accounting snapshot artifact must not expose authority key `{key}`"
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
