use criterion::{criterion_group, criterion_main, Criterion};
use svc_rewarder::core::AmountMinor;
use svc_rewarder::core::{compute_manifest, ComputeInput};
use svc_rewarder::inputs::{AccountContribution, AccountingSnapshot, ContentCid, RewardPolicy};
use svc_rewarder::outputs::IntentResult;

fn bench_reward_calc(c: &mut Criterion) {
    c.bench_function("reward_calc_100", |b| {
        b.iter(|| {
            let snapshot = AccountingSnapshot {
                produced_at_millis: 1,
                pool_minor_units: AmountMinor(1_000_000),
                contributions: (0..100)
                    .map(|i| AccountContribution {
                        account: format!("acct_{i:03}"),
                        bytes_stored: 1000 + i,
                        bytes_served: 500 + i,
                        uptime_seconds: 60,
                    })
                    .collect(),
            };
            let policy = RewardPolicy {
                id: "policy:v1".into(),
                hash: format!("b3:{}", "b".repeat(64)),
                signed: true,
                max_payout_minor_units: AmountMinor(1_000_000),
                min_payout_minor_units: AmountMinor(1),
                weight_bps: 10_000,
                rounding: "floor".into(),
            };
            let input = ComputeInput {
                epoch_id: "bench".into(),
                inputs_cid: ContentCid::parse(format!("b3:{}", "a".repeat(64))).unwrap(),
                policy,
                snapshot,
                dry_run: true,
                idempotency_salt: "svc-rewarder|v1".into(),
            };
            compute_manifest(input, IntentResult::DryRun).unwrap()
        })
    });
}

criterion_group!(benches, bench_reward_calc);
criterion_main!(benches);
