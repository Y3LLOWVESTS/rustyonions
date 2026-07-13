#![allow(clippy::missing_panics_doc)]

//! RO:WHAT — Internal ROC Beta Phase 5 Round 2 anti-farming/event-class tests for svc-rewarder.
//! RO:WHY — Proves rewarder consumes only eligible/capped inputs and cannot turn raw engagement or unfunded ad events into protocol ROC.
//! RO:INTERACTS — anti-farming input gates, accounting snapshots, reward manifest dry-run path.
//! RO:INVARIANTS — rewarder remains planning-only; no wallet/ledger mutation, fake receipt, fake balance, bridge, staking, liquidity, or external settlement.
//! RO:METRICS — none.
//! RO:CONFIG — caps are explicit test fixtures standing in for validated economics config.
//! RO:SECURITY — rejects analytics, direct metering, unverified proof candidates, and protocol-funded ad-budgeted material.
//! RO:TEST — `cargo test -p svc-rewarder --test internal_roc_beta_phase5_antifarming_event_gates`.

use svc_rewarder::{
    core::{compute_manifest, AmountMinor, ComputeInput},
    inputs::{
        capped_contributions_from_candidates, capped_contributions_from_candidates_with_economics,
        load_internal_roc_planning_economics_toml, AccountingSnapshot, AntiFarmingCapPolicy,
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
) -> CappedRewardInputCandidate {
    CappedRewardInputCandidate {
        account: account.to_owned(),
        event_class,
        verified,
        explicit_budget_authorized,
        policy_gate_passed: true,
        bytes_stored: 9_999_999,
        bytes_served: 9_999_999,
        uptime_seconds: 9_999_999,
    }
}

fn policy(funding_source: RewardFundingSource) -> RewardPolicy {
    RewardPolicy {
        id: "internal-roc-phase5-round2".to_owned(),
        hash: format!("b3:{}", "a".repeat(64)),
        signed: true,
        funding_source,
        max_payout_minor_units: AmountMinor(1_000),
        min_payout_minor_units: AmountMinor(1),
        weight_bps: 10_000,
        rounding: "floor".to_owned(),
    }
}

#[test]
fn anti_farming_caps_verified_proof_eligible_inputs_before_planning() {
    let contributions = capped_contributions_from_candidates(
        vec![candidate(
            "acct_alice",
            RewardInputEventClass::ProofEligible,
            true,
            false,
        )],
        &caps(),
        RewardFundingSource::ProtocolPool,
    )
    .expect("verified proof-eligible input should cap into planning contribution");

    assert_eq!(contributions.len(), 1);
    let contribution = &contributions[0];
    assert_eq!(contribution.account, "acct_alice");
    assert!(contribution.bytes_stored <= caps().max_bytes_stored);
    assert!(contribution.bytes_served <= caps().max_bytes_served);
    assert!(contribution.uptime_seconds <= caps().max_uptime_seconds);
    assert!(
        contribution.score().expect("score should fit") <= caps().max_score_per_account,
        "anti-farming caps must bound score before reward planning"
    );
}

#[test]
fn rewarder_rejects_analytics_metering_and_unverified_proof_candidates() {
    for rejected in [
        candidate(
            "acct_view_farmer",
            RewardInputEventClass::AnalyticsOnly,
            true,
            false,
        ),
        candidate(
            "acct_metering_only",
            RewardInputEventClass::Metering,
            true,
            false,
        ),
        candidate(
            "acct_unverified",
            RewardInputEventClass::ProofEligible,
            false,
            false,
        ),
    ] {
        assert!(
            capped_contributions_from_candidates(
                vec![rejected],
                &caps(),
                RewardFundingSource::ProtocolPool,
            )
            .is_err(),
            "analytics, direct metering, and unverified proof candidates must not enter planning"
        );
    }
}

#[test]
fn ad_budgeted_material_requires_explicit_non_protocol_budget() {
    let unfunded = candidate("acct_ad", RewardInputEventClass::AdBudgeted, true, false);
    assert!(
        capped_contributions_from_candidates(
            vec![unfunded],
            &caps(),
            RewardFundingSource::AdvertiserBudget,
        )
        .is_err(),
        "ad-budgeted material without explicit budget must reject"
    );

    let protocol_funded = candidate("acct_ad", RewardInputEventClass::AdBudgeted, true, true);
    assert!(
        capped_contributions_from_candidates(
            vec![protocol_funded],
            &caps(),
            RewardFundingSource::ProtocolPool,
        )
        .is_err(),
        "ad-budgeted material must not use protocol-pool emission"
    );

    let advertiser_funded = candidate("acct_ad", RewardInputEventClass::AdBudgeted, true, true);
    let contributions = capped_contributions_from_candidates(
        vec![advertiser_funded],
        &caps(),
        RewardFundingSource::AdvertiserBudget,
    )
    .expect("verified ad-budgeted material with explicit advertiser budget may enter planning");

    assert_eq!(contributions.len(), 1);
    assert_eq!(contributions[0].account, "acct_ad");
}

#[test]
fn capped_inputs_produce_deterministic_non_mutating_reward_manifest() {
    let candidates = vec![
        candidate("acct_b", RewardInputEventClass::ProofEligible, true, false),
        candidate("acct_a", RewardInputEventClass::ProofEligible, true, false),
    ];

    let contributions = capped_contributions_from_candidates(
        candidates,
        &caps(),
        RewardFundingSource::ProtocolPool,
    )
    .expect("verified proof candidates should cap into planning inputs");

    assert_eq!(
        contributions
            .iter()
            .map(|contribution| contribution.account.as_str())
            .collect::<Vec<_>>(),
        vec!["acct_a", "acct_b"],
        "capped contributions must be deterministic and sorted"
    );

    let snapshot = AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(1_000),
        contributions,
    };

    let input = ComputeInput {
        epoch_id: "phase5-round2".to_owned(),
        inputs_cid: ContentCid::parse(format!("b3:{}", "b".repeat(64)))
            .expect("test CID should be valid"),
        policy: policy(RewardFundingSource::ProtocolPool),
        snapshot,
        dry_run: true,
        idempotency_salt: "phase5-round2".to_owned(),
    };

    let first = compute_manifest(input.clone(), IntentResult::DryRun)
        .expect("dry-run reward manifest should compute");
    let second = compute_manifest(input, IntentResult::DryRun)
        .expect("same dry-run reward manifest should compute");

    assert_eq!(first.commitment, second.commitment);
    assert_eq!(first.payouts, second.payouts);
    assert!(!first.ledger.emitted);
    assert_eq!(first.ledger.result, "dry_run");
}

#[test]
fn anti_farming_gate_source_has_no_wallet_or_ledger_authority_shortcuts() {
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
    ] {
        assert!(
            !source.contains(forbidden),
            "anti-farming gate must not construct authority via `{forbidden}`"
        );
    }
}

fn economics_with_event_limit(
    limit: u64,
) -> svc_rewarder::inputs::InternalRocRewardPlanningEconomics {
    let canonical = include_bytes!("../../../configs/roc-economics.toml");

    let canonical_raw =
        std::str::from_utf8(canonical).expect("canonical economics should be UTF-8");

    let changed_raw = canonical_raw.replacen(
        "max_events_per_account_per_epoch = 1000",
        &format!("max_events_per_account_per_epoch = {limit}"),
        1,
    );

    assert_ne!(
        changed_raw, canonical_raw,
        "test must replace the canonical event limit"
    );

    let economics = load_internal_roc_planning_economics_toml(changed_raw.as_bytes())
        .expect("changed complete economics should validate");

    assert_eq!(economics.max_events_per_account_per_epoch, limit);

    economics
}

#[test]
fn economics_event_limit_aggregates_same_account_deterministically() {
    let economics = economics_with_event_limit(2);

    let candidates = vec![
        candidate(
            " acct_repeat ",
            RewardInputEventClass::ProofEligible,
            true,
            false,
        ),
        candidate(
            "acct_repeat",
            RewardInputEventClass::ProofEligible,
            true,
            false,
        ),
    ];

    let first = capped_contributions_from_candidates_with_economics(
        candidates.clone(),
        &caps(),
        RewardFundingSource::ProtocolPool,
        &economics,
    )
    .expect("two events at the configured limit should pass");

    let mut reversed = candidates;
    reversed.reverse();

    let second = capped_contributions_from_candidates_with_economics(
        reversed,
        &caps(),
        RewardFundingSource::ProtocolPool,
        &economics,
    )
    .expect("reordered events at the limit should pass");

    assert_eq!(first, second);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].account, "acct_repeat");

    assert!(first[0].score().expect("aggregated score should fit") <= caps().max_score_per_account);
}

#[test]
fn economics_event_limit_rejects_over_limit_account() {
    let economics = economics_with_event_limit(2);

    let error = capped_contributions_from_candidates_with_economics(
        vec![
            candidate(
                "acct_farmer",
                RewardInputEventClass::ProofEligible,
                true,
                false,
            ),
            candidate(
                "acct_farmer",
                RewardInputEventClass::ProofEligible,
                true,
                false,
            ),
            candidate(
                "acct_farmer",
                RewardInputEventClass::ProofEligible,
                true,
                false,
            ),
        ],
        &caps(),
        RewardFundingSource::ProtocolPool,
        &economics,
    )
    .expect_err("third event must exceed the configured account limit");

    assert_eq!(error.reason(), "bad_request");
    assert!(error
        .to_string()
        .contains("exceeds economics max events per account per epoch"));
}
