#![allow(clippy::missing_panics_doc)]

//! RO:WHAT — Internal ROC Beta Phase 5 Round 2 policy-gate interlock tests for svc-rewarder.
//! RO:WHY — Proves rewarder anti-farming/cap checks cannot bypass ron-policy; capped candidates need an explicit policy-gate marker before planning.
//! RO:INTERACTS — anti_farming input gates, reward manifest dry-run path, Phase 3 reward-plan/payout intent boundaries.
//! RO:INVARIANTS — rewarder plans only; policy gate is required before planning; no wallet/ledger mutation, fake receipt, fake balance, bridge, staking, liquidity, or external settlement.
//! RO:METRICS — none.
//! RO:CONFIG — caps are explicit test fixtures standing in for validated economics config.
//! RO:SECURITY — rejects attempts to treat verification/caps as sufficient without policy validation.
//! RO:TEST — `cargo test -p svc-rewarder --test internal_roc_beta_phase5_policy_gate_interlock`.

use svc_rewarder::{
    core::{compute_manifest, AmountMinor, ComputeInput},
    inputs::{
        capped_contributions_from_candidates, AccountingSnapshot, AntiFarmingCapPolicy,
        CappedRewardInputCandidate, ContentCid, RewardFundingSource, RewardInputEventClass,
        RewardPolicy,
    },
    outputs::IntentResult,
};

fn caps() -> AntiFarmingCapPolicy {
    AntiFarmingCapPolicy {
        max_bytes_stored: 1_000,
        max_bytes_served: 2_000,
        max_uptime_seconds: 60,
        max_score_per_account: 10_000,
    }
}

fn candidate(
    account: &str,
    event_class: RewardInputEventClass,
    verified: bool,
    explicit_budget_authorized: bool,
    policy_gate_passed: bool,
) -> CappedRewardInputCandidate {
    CappedRewardInputCandidate {
        account: account.to_owned(),
        event_class,
        verified,
        explicit_budget_authorized,
        policy_gate_passed,
        bytes_stored: 9_999_999,
        bytes_served: 9_999_999,
        uptime_seconds: 9_999_999,
    }
}

fn policy() -> RewardPolicy {
    RewardPolicy {
        id: "internal-roc-phase5-round2-policy-interlock".to_owned(),
        hash: format!("b3:{}", "c".repeat(64)),
        signed: true,
        funding_source: RewardFundingSource::ProtocolPool,
        max_payout_minor_units: AmountMinor(1_000),
        min_payout_minor_units: AmountMinor(1),
        weight_bps: 10_000,
        rounding: "floor".to_owned(),
    }
}

#[test]
fn rewarder_rejects_verified_capped_candidate_without_policy_gate() {
    let result = capped_contributions_from_candidates(
        vec![candidate(
            "acct_policy_missing",
            RewardInputEventClass::ProofEligible,
            true,
            false,
            false,
        )],
        &caps(),
        RewardFundingSource::ProtocolPool,
    );

    assert!(
        result.is_err(),
        "verification/caps alone must not bypass ron-policy"
    );
}

#[test]
fn rewarder_accepts_verified_capped_candidate_after_policy_gate() {
    let contributions = capped_contributions_from_candidates(
        vec![candidate(
            "acct_policy_passed",
            RewardInputEventClass::ProofEligible,
            true,
            false,
            true,
        )],
        &caps(),
        RewardFundingSource::ProtocolPool,
    )
    .expect("verified/capped/policy-gated proof candidate may enter planning");

    assert_eq!(contributions.len(), 1);
    assert_eq!(contributions[0].account, "acct_policy_passed");
    assert!(
        contributions[0].score().expect("score should fit") <= caps().max_score_per_account,
        "policy-gated candidates must still remain capped"
    );
}

#[test]
fn ad_budgeted_candidate_requires_policy_gate_and_explicit_non_protocol_budget() {
    let missing_policy = capped_contributions_from_candidates(
        vec![candidate(
            "acct_ad_policy_missing",
            RewardInputEventClass::AdBudgeted,
            true,
            true,
            false,
        )],
        &caps(),
        RewardFundingSource::AdvertiserBudget,
    );

    assert!(
        missing_policy.is_err(),
        "ad-budgeted material must pass policy before planning"
    );

    let protocol_pool_attempt = capped_contributions_from_candidates(
        vec![candidate(
            "acct_ad_protocol_pool",
            RewardInputEventClass::AdBudgeted,
            true,
            true,
            true,
        )],
        &caps(),
        RewardFundingSource::ProtocolPool,
    );

    assert!(
        protocol_pool_attempt.is_err(),
        "ad-budgeted material must not use protocol-pool emission even after policy gate"
    );

    let advertiser_budget = capped_contributions_from_candidates(
        vec![candidate(
            "acct_ad_budget",
            RewardInputEventClass::AdBudgeted,
            true,
            true,
            true,
        )],
        &caps(),
        RewardFundingSource::AdvertiserBudget,
    )
    .expect("ad-budgeted material with explicit budget and policy gate may enter planning");

    assert_eq!(advertiser_budget.len(), 1);
    assert_eq!(advertiser_budget[0].account, "acct_ad_budget");
}

#[test]
fn policy_gated_capped_inputs_still_produce_non_mutating_manifest_only() {
    let contributions = capped_contributions_from_candidates(
        vec![
            candidate(
                "acct_b",
                RewardInputEventClass::ProofEligible,
                true,
                false,
                true,
            ),
            candidate(
                "acct_a",
                RewardInputEventClass::ProofEligible,
                true,
                false,
                true,
            ),
        ],
        &caps(),
        RewardFundingSource::ProtocolPool,
    )
    .expect("policy-gated candidates should cap into planning input");

    assert_eq!(
        contributions
            .iter()
            .map(|contribution| contribution.account.as_str())
            .collect::<Vec<_>>(),
        vec!["acct_a", "acct_b"],
        "policy-gated capped contributions must remain deterministic and sorted"
    );

    let input = ComputeInput {
        epoch_id: "phase5-round2-policy-gate".to_owned(),
        inputs_cid: ContentCid::parse(format!("b3:{}", "d".repeat(64)))
            .expect("test CID should be valid"),
        policy: policy(),
        snapshot: AccountingSnapshot {
            produced_at_millis: 1,
            pool_minor_units: AmountMinor(1_000),
            contributions,
        },
        dry_run: true,
        idempotency_salt: "phase5-round2-policy-gate".to_owned(),
    };

    let manifest = compute_manifest(input, IntentResult::DryRun)
        .expect("policy-gated dry-run reward manifest should compute");

    assert!(!manifest.ledger.emitted);
    assert_eq!(manifest.ledger.result, "dry_run");
    assert!(manifest
        .payouts
        .iter()
        .all(|payout| payout.amount_minor_units.get() > 0));
}

#[test]
fn policy_gate_interlock_source_has_no_wallet_or_ledger_authority_shortcuts() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("inputs")
            .join("anti_farming.rs"),
    )
    .expect("anti_farming source should be readable")
    .to_ascii_lowercase();

    for forbidden in [
        "wallet_side_effect: true",
        "ledger_side_effect: true",
        "receipt_truth: true",
        "balance_truth: true",
        "direct_protocol_roc_allocation: true",
        "issue_from",
        "transfer_from",
        "burn_from",
        "capture_from",
        "release_from",
        "unlock_from",
        "bypass_policy",
        "skip_policy",
    ] {
        assert!(
            !source.contains(forbidden),
            "policy gate interlock must not construct authority via `{forbidden}`"
        );
    }
}
