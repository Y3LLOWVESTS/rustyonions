use svc_rewarder::core::{compute_manifest, AmountMinor, ComputeInput};
use svc_rewarder::inputs::{AccountContribution, AccountingSnapshot, ContentCid, RewardPolicy};
use svc_rewarder::outputs::{IntentResult, SettlementBatch};

fn cid() -> ContentCid {
    ContentCid::parse(format!("b3:{}", "a".repeat(64))).unwrap()
}

fn policy(min_payout: AmountMinor, max_payout: AmountMinor) -> RewardPolicy {
    RewardPolicy {
        id: "policy:v1".into(),
        hash: format!("b3:{}", "b".repeat(64)),
        signed: true,
        max_payout_minor_units: max_payout,
        min_payout_minor_units: min_payout,
        weight_bps: 10_000,
        rounding: "floor".into(),
    }
}

fn input_with(
    epoch_id: &str,
    pool: AmountMinor,
    min_payout: AmountMinor,
    max_payout: AmountMinor,
    contributions: Vec<AccountContribution>,
) -> ComputeInput {
    ComputeInput {
        epoch_id: epoch_id.into(),
        inputs_cid: cid(),
        policy: policy(min_payout, max_payout),
        snapshot: AccountingSnapshot {
            produced_at_millis: 1,
            pool_minor_units: pool,
            contributions,
        },
        dry_run: true,
        idempotency_salt: "quarantine-test".into(),
    }
}

#[test]
fn dust_below_min_payout_becomes_residual_not_zero_payouts() {
    let input = input_with(
        "epoch-dust-1",
        AmountMinor(10),
        AmountMinor(100),
        AmountMinor(10),
        vec![
            AccountContribution {
                account: "acct_a".into(),
                bytes_stored: 1,
                bytes_served: 0,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: "acct_b".into(),
                bytes_stored: 1,
                bytes_served: 0,
                uptime_seconds: 0,
            },
        ],
    );

    let manifest = compute_manifest(input, IntentResult::DryRun).unwrap();

    assert!(manifest.payouts.is_empty());
    assert_eq!(manifest.totals.pool_minor_units, AmountMinor(10));
    assert_eq!(manifest.totals.payout_minor_units, AmountMinor::ZERO);
    assert_eq!(manifest.totals.residual_minor_units, AmountMinor(10));

    let settlement = SettlementBatch::from_manifest(&manifest).unwrap();
    assert!(settlement.intents.is_empty());
    assert_eq!(settlement.total_minor_units, AmountMinor::ZERO);
}

#[test]
fn zero_activity_snapshot_yields_all_residual() {
    let input = input_with(
        "epoch-zero-activity",
        AmountMinor(500),
        AmountMinor(1),
        AmountMinor(500),
        vec![
            AccountContribution {
                account: "acct_a".into(),
                bytes_stored: 0,
                bytes_served: 0,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: "acct_b".into(),
                bytes_stored: 0,
                bytes_served: 0,
                uptime_seconds: 0,
            },
        ],
    );

    let manifest = compute_manifest(input, IntentResult::DryRun).unwrap();

    assert!(manifest.payouts.is_empty());
    assert_eq!(manifest.totals.payout_minor_units, AmountMinor::ZERO);
    assert_eq!(manifest.totals.residual_minor_units, AmountMinor(500));
}

#[test]
fn arithmetic_overflow_quarantines_before_any_settlement_plan() {
    let input = input_with(
        "epoch-overflow-1",
        AmountMinor(u128::MAX),
        AmountMinor(1),
        AmountMinor(u128::MAX),
        vec![AccountContribution {
            account: "acct_overflow".into(),
            bytes_stored: 2,
            bytes_served: 0,
            uptime_seconds: 0,
        }],
    );

    let err = compute_manifest(input, IntentResult::DryRun).unwrap_err();

    assert_eq!(err.reason(), "invariant");
}
