//! RO:WHAT — Phase 14D economics identity binding tests for svc-rewarder manifests.
//! RO:WHY — Reward plans must bind the exact normalized economics profile used for deterministic planning.
//! RO:INTERACTS — ron-policy economics hashing, svc-rewarder planning projection, run key, and manifest commitment.
//! RO:INVARIANTS — same config/evidence is replay-stable; changed profile changes plan identity; malformed bindings fail closed.
//! RO:SECURITY — economics identity is declarative planning input only and grants no wallet or ledger authority.
//! RO:TEST — this integration target.

#![allow(clippy::missing_panics_doc)]

use svc_rewarder::core::{compute_manifest, compute_manifest_with_economics, ComputeInput};
use svc_rewarder::inputs::{
    load_internal_roc_planning_economics_toml, AccountContribution, AccountingSnapshot, ContentCid,
    InternalRocRewardPlanningEconomics,
};
use svc_rewarder::outputs::IntentResult;

const POLICY_ID: &str = "policy:phase14d";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

const CANONICAL: &[u8] = include_bytes!("../../../configs/roc-economics.toml");

const DEVELOPMENT: &[u8] = include_bytes!("../../../configs/roc-economics.dev.toml");

fn input(economics: &InternalRocRewardPlanningEconomics) -> ComputeInput {
    let policy = economics
        .to_reward_policy(POLICY_ID, POLICY_HASH)
        .expect("economics should project to reward policy");

    ComputeInput {
        epoch_id: "epoch-phase14d-economics-binding".to_owned(),
        inputs_cid: ContentCid::parse(format!("b3:{}", "a".repeat(64)))
            .expect("test CID should parse"),
        policy,
        snapshot: AccountingSnapshot {
            produced_at_millis: 1,
            pool_minor_units: svc_rewarder::core::AmountMinor(1_000),
            contributions: vec![
                AccountContribution {
                    account: "acct_b".to_owned(),
                    bytes_stored: 200,
                    bytes_served: 80,
                    uptime_seconds: 20,
                },
                AccountContribution {
                    account: "acct_a".to_owned(),
                    bytes_stored: 100,
                    bytes_served: 40,
                    uptime_seconds: 10,
                },
            ],
        },
        dry_run: true,
        idempotency_salt: "svc-rewarder|phase14d|economics-binding".to_owned(),
    }
}

#[test]
fn same_economics_and_evidence_produce_same_bound_plan() {
    let economics = load_internal_roc_planning_economics_toml(CANONICAL)
        .expect("canonical economics should load");

    let first =
        compute_manifest_with_economics(input(&economics), IntentResult::DryRun, &economics)
            .expect("first bound manifest should compute");

    let second =
        compute_manifest_with_economics(input(&economics), IntentResult::DryRun, &economics)
            .expect("second bound manifest should compute");

    assert_eq!(first, second);
    assert_eq!(first.economics_config_hash, economics.economics_config_hash);
    assert_eq!(first.economics_config_schema, economics.schema);
    assert_eq!(first.economics_config_version, economics.version);
    assert_eq!(first.economics_profile, economics.profile);
}

#[test]
fn changing_profile_changes_reward_plan_identity() {
    let canonical = load_internal_roc_planning_economics_toml(CANONICAL)
        .expect("canonical economics should load");

    let development = load_internal_roc_planning_economics_toml(DEVELOPMENT)
        .expect("development economics should load");

    let canonical_manifest =
        compute_manifest_with_economics(input(&canonical), IntentResult::DryRun, &canonical)
            .expect("canonical manifest should compute");

    let development_manifest =
        compute_manifest_with_economics(input(&development), IntentResult::DryRun, &development)
            .expect("development manifest should compute");

    assert_ne!(
        canonical.economics_config_hash,
        development.economics_config_hash
    );
    assert_ne!(canonical_manifest.run_key, development_manifest.run_key);
    assert_ne!(
        canonical_manifest.commitment,
        development_manifest.commitment
    );
    assert_eq!(canonical_manifest.economics_profile, "canonical");
    assert_eq!(development_manifest.economics_profile, "development");
}

#[test]
fn default_compute_path_binds_reviewed_canonical_profile() {
    let canonical = load_internal_roc_planning_economics_toml(CANONICAL)
        .expect("canonical economics should load");

    let manifest = compute_manifest(input(&canonical), IntentResult::DryRun)
        .expect("default manifest should compute");

    assert_eq!(
        manifest.economics_config_hash,
        canonical.economics_config_hash
    );
    assert_eq!(manifest.economics_profile, "canonical");
}

#[test]
fn malformed_economics_hash_fails_closed() {
    let mut economics = load_internal_roc_planning_economics_toml(CANONICAL)
        .expect("canonical economics should load");

    economics.economics_config_hash = "not-a-canonical-hash".to_owned();

    let error = compute_manifest_with_economics(
        input(
            &load_internal_roc_planning_economics_toml(CANONICAL)
                .expect("valid policy source should load"),
        ),
        IntentResult::DryRun,
        &economics,
    )
    .expect_err("malformed economics hash must reject");

    assert!(error.to_string().contains("economics_config_hash"));
}

#[test]
fn economics_epoch_pool_cap_overrides_larger_policy_and_snapshot() {
    let economics = load_internal_roc_planning_economics_toml(CANONICAL)
        .expect("canonical economics should load");

    let mut compute_input = input(&economics);
    compute_input.snapshot.pool_minor_units = svc_rewarder::core::AmountMinor(2_000_000);
    compute_input.policy.max_payout_minor_units = svc_rewarder::core::AmountMinor(u128::MAX);

    let manifest = compute_manifest_with_economics(compute_input, IntentResult::DryRun, &economics)
        .expect("economics-capped manifest should compute");

    assert_eq!(
        manifest.totals.pool_minor_units,
        economics.epoch_pool_cap_minor
    );
    assert_eq!(
        manifest.totals.pool_minor_units,
        svc_rewarder::core::AmountMinor(1_000_000)
    );
}

#[test]
fn economics_account_cap_limits_every_planned_payout() {
    let economics = load_internal_roc_planning_economics_toml(CANONICAL)
        .expect("canonical economics should load");

    let mut compute_input = input(&economics);
    compute_input.snapshot.pool_minor_units = svc_rewarder::core::AmountMinor(1_000_000);

    let manifest = compute_manifest_with_economics(compute_input, IntentResult::DryRun, &economics)
        .expect("account-capped manifest should compute");

    assert_eq!(manifest.payouts.len(), 2);

    assert!(manifest.payouts.iter().all(|payout| {
        payout.amount_minor_units <= economics.max_reward_minor_per_account_per_epoch
    }));

    assert_eq!(
        manifest.totals.payout_minor_units,
        svc_rewarder::core::AmountMinor(20_000)
    );
    assert_eq!(
        manifest.totals.residual_minor_units,
        svc_rewarder::core::AmountMinor(980_000)
    );
}

#[test]
fn changing_account_cap_changes_plan_deterministically() {
    let canonical = load_internal_roc_planning_economics_toml(CANONICAL)
        .expect("canonical economics should load");

    let canonical_raw = std::str::from_utf8(CANONICAL).expect("canonical TOML should be UTF-8");

    let changed_raw = canonical_raw.replacen(
        "max_reward_minor_per_account_per_epoch = \"10000\"",
        "max_reward_minor_per_account_per_epoch = \"9000\"",
        1,
    );

    assert_ne!(
        changed_raw, canonical_raw,
        "test must change the reviewed account cap"
    );

    let changed = load_internal_roc_planning_economics_toml(changed_raw.as_bytes())
        .expect("changed standalone economics should load");

    let mut canonical_input = input(&canonical);
    canonical_input.snapshot.pool_minor_units = svc_rewarder::core::AmountMinor(1_000_000);

    let mut changed_input = input(&changed);
    changed_input.snapshot.pool_minor_units = svc_rewarder::core::AmountMinor(1_000_000);

    let canonical_manifest =
        compute_manifest_with_economics(canonical_input, IntentResult::DryRun, &canonical)
            .expect("canonical manifest should compute");

    let changed_manifest =
        compute_manifest_with_economics(changed_input, IntentResult::DryRun, &changed)
            .expect("changed manifest should compute");

    assert_eq!(
        canonical_manifest.totals.payout_minor_units,
        svc_rewarder::core::AmountMinor(20_000)
    );
    assert_eq!(
        changed_manifest.totals.payout_minor_units,
        svc_rewarder::core::AmountMinor(18_000)
    );

    assert_ne!(
        canonical.economics_config_hash,
        changed.economics_config_hash
    );
    assert_ne!(canonical_manifest.run_key, changed_manifest.run_key);
    assert_ne!(canonical_manifest.commitment, changed_manifest.commitment);
}

#[test]
fn canonical_category_caps_project_in_deterministic_order() {
    let economics = load_internal_roc_planning_economics_toml(CANONICAL)
        .expect("canonical economics should load");

    let categories = economics
        .category_caps
        .iter()
        .map(|cap| cap.category.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        categories,
        vec!["creator_rewards", "moderation", "node_delivery",]
    );

    let delivery = economics
        .category_cap("node_delivery")
        .expect("node_delivery category should exist");

    assert_eq!(delivery.pool_bps, 2_000);
    assert_eq!(
        delivery.category_cap_minor,
        svc_rewarder::core::AmountMinor(200_000)
    );
}

#[test]
fn effective_category_cap_applies_bps_and_absolute_ceiling() {
    let economics = load_internal_roc_planning_economics_toml(CANONICAL)
        .expect("canonical economics should load");

    assert_eq!(
        economics
            .effective_category_pool_cap("node_delivery", svc_rewarder::core::AmountMinor(100_000),)
            .expect("small category pool should compute"),
        svc_rewarder::core::AmountMinor(20_000)
    );

    assert_eq!(
        economics
            .effective_category_pool_cap(
                "node_delivery",
                svc_rewarder::core::AmountMinor(2_000_000),
            )
            .expect("epoch-limited category pool should compute"),
        svc_rewarder::core::AmountMinor(200_000)
    );
}

#[test]
fn unknown_reward_category_fails_closed() {
    let economics = load_internal_roc_planning_economics_toml(CANONICAL)
        .expect("canonical economics should load");

    let error = economics
        .category_cap("unknown_category")
        .expect_err("unknown category must reject");

    assert!(error.to_string().contains("unknown reward category"));
}

#[test]
fn changing_category_cap_changes_identity_and_effective_cap() {
    let canonical = load_internal_roc_planning_economics_toml(CANONICAL)
        .expect("canonical economics should load");

    let canonical_raw =
        std::str::from_utf8(CANONICAL).expect("canonical economics should be UTF-8");

    let old = r#"category = "node_delivery"
pool_bps = 2000
category_cap_minor = "200000""#;

    let new = r#"category = "node_delivery"
pool_bps = 2000
category_cap_minor = "150000""#;

    assert_eq!(
        canonical_raw.matches(old).count(),
        1,
        "test must find node_delivery exactly once"
    );

    let changed_raw = canonical_raw.replacen(old, new, 1);

    let changed = load_internal_roc_planning_economics_toml(changed_raw.as_bytes())
        .expect("changed complete economics should load");

    assert_ne!(
        canonical.economics_config_hash,
        changed.economics_config_hash
    );

    assert_eq!(
        canonical
            .effective_category_pool_cap(
                "node_delivery",
                svc_rewarder::core::AmountMinor(1_000_000),
            )
            .expect("canonical category cap should compute"),
        svc_rewarder::core::AmountMinor(200_000)
    );

    assert_eq!(
        changed
            .effective_category_pool_cap(
                "node_delivery",
                svc_rewarder::core::AmountMinor(1_000_000),
            )
            .expect("changed category cap should compute"),
        svc_rewarder::core::AmountMinor(150_000)
    );
}
