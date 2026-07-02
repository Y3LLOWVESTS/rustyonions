//! RO:WHAT — Internal ROC Stabilization boundary tests for ron-accounting snapshot/report non-authority.
//! RO:WHY — Product beta readiness requires accounting observations and snapshots to stay deterministic and non-mutating.
//! RO:INTERACTS — accounting/events.rs, reward_snapshot.rs, reward_projection.rs, existing Internal ROC accounting regressions.
//! RO:INVARIANTS — accounting is derivative metering/snapshot infrastructure only; wallet/ledger remain truth.
//! RO:SECURITY — no wallet/ledger mutation, fake receipt/balance/finality, paid unlock, payout execution, bridge, ROX/Solana, staking, liquidity, or external settlement.
//! RO:TEST — cargo test -p ron-accounting --test internal_roc_stabilization_snapshot_non_authority_boundary.

#![allow(clippy::missing_panics_doc)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountingSnapshotNonAuthorityContract {
    schema: String,
    source_crate: String,
    role: String,
    wallet_truth: String,
    ledger_truth: String,
    snapshot_role: String,
    payout_role: String,
    artifact_role: String,
}

#[test]
fn accounting_sources_keep_snapshot_artifacts_deterministic_and_non_authoritative() {
    let events = read_rel("src/accounting/events.rs");
    let reward_snapshot = read_rel("src/accounting/reward_snapshot.rs");
    let reward_projection = read_rel("src/accounting/reward_projection.rs");
    let slice = read_rel("src/accounting/slice.rs");
    let http_ingest = read_rel("src/http_ingest.rs");

    assert_all(
        "accounting event source",
        &events,
        &[
            "pub enum MetricKind",
            "pub struct UsageEvent",
            "#[serde(deny_unknown_fields)]",
            "record_usage_event",
            "record_usage_events",
            "EventIngestReport",
        ],
    );

    assert_all(
        "accounting reward snapshot source",
        &reward_snapshot,
        &[
            "Stable reward-snapshot export DTO consumed by svc-rewarder",
            "integer-only counters",
            "canonical account order",
            "no duplicate accounts",
            "b3 CID over canonical bytes",
            "pub struct RewardSnapshotExport",
            "#[serde(deny_unknown_fields)]",
            "pool_minor_units",
            "canonicalized",
            "canonical_bytes",
            "canonical_cid",
        ],
    );

    assert_all(
        "accounting reward projection source",
        &reward_projection,
        &[
            "project_reward_snapshot_from_slices",
            "RewardProjectionConfig",
            "RewardProjectionReport",
            "checked_add",
            "overflow during reward projection",
        ],
    );

    assert_all(
        "accounting slice source",
        &slice,
        &["SealedSlice", "digest", "SliceId"],
    );

    assert_all(
        "accounting lightweight ingest source",
        &http_ingest,
        &["/v1/usage-events", "/v1/snapshot", "record_usage_events"],
    );

    assert_none(
        "ron-accounting production source forbidden runtime markers",
        &format!("{events}\n{reward_snapshot}\n{reward_projection}\n{slice}\n{http_ingest}")
            .to_lowercase(),
        &[
            "wallet_mutate",
            "ledger_mutate",
            "fake_receipt",
            "fake_balance",
            "fake_finality",
            "cache_only_unlock",
            "unlock_from_cache",
            "payout_execution_authority",
            "bridge_runtime",
            "rox_runtime",
            "solana_runtime",
            "staking_runtime",
            "liquidity_runtime",
            "external_settlement_runtime",
            "mint_rox",
            "burn_rox",
        ],
    );
}

#[test]
fn existing_accounting_regressions_are_wired_into_stabilization_surface() {
    let paid_content = read_rel("tests/internal_roc_beta_paid_content_snapshot_non_authority.rs");
    let replay = read_rel("tests/internal_roc_beta_phase2_snapshot_cannot_change_replay.rs");
    let event_class = read_rel("tests/internal_roc_beta_phase3_snapshot_event_class_boundary.rs");
    let payout = read_rel("tests/internal_roc_beta_phase3_approved_payout_observation_boundary.rs");
    let config = read_rel("tests/internal_roc_beta_phase5_config_label_non_authority.rs");
    let antifarming = read_rel("tests/internal_roc_beta_phase5_event_class_antifarming.rs");

    assert_all(
        "paid-content accounting snapshot regression",
        &paid_content,
        &[
            "wallet_receipt_observations_seal_as_derivative_snapshot_not_balance_truth",
            "accounting_usage_events_reject_paid_content_authority_poison_fields",
            "raw_paid_content_engagement_cannot_project_rewards_without_verification_or_receipt_truth",
        ],
    );

    assert_all(
        "phase2 accounting replay non-authority regression",
        &replay,
        &[
            "accounting_snapshot_observes_accepted_receipts_only_and_cannot_change_replay",
            "usage_event_rejects_replay_balance_and_receipt_authority_poison_fields",
        ],
    );

    assert_all(
        "phase3 accounting event-class regression",
        &event_class,
        &[
            "paid_wallet_receipt_events_become_snapshot_input_only",
            "accounting_snapshot_rows_are_deterministic_for_same_events_regardless_input_order",
            "reward_snapshot_exports_remain_planning_material_not_wallet_authority",
            "usage_events_reject_client_supplied_classification_and_reward_authority",
        ],
    );

    assert_all(
        "approved payout observation regression",
        &payout,
        &[
            "executed_payout_receipts_become_accounting_snapshot_input_only",
            "payout_receipt_observation_rows_are_deterministic_regardless_input_order",
            "planned_payout_candidates_are_not_executed_wallet_receipt_observations",
            "usage_event_rejects_client_supplied_payout_execution_authority",
            "reward_snapshot_export_for_payout_observations_stays_planning_material",
        ],
    );

    assert_all(
        "config label non-authority regression",
        &config,
        &["config", "non_authority"],
    );

    assert_all(
        "event-class anti-farming regression",
        &antifarming,
        &[
            "analytics_only",
            "metering",
            "proof_eligible",
            "ad_budgeted",
            "economic_receipt",
        ],
    );
}

#[test]
fn accounting_snapshot_contract_rejects_authority_poison_fields() {
    for field in [
        "wallet_mutation_authority",
        "ledger_mutation_authority",
        "issue_authority",
        "transfer_authority",
        "burn_authority",
        "hold_authority",
        "capture_authority",
        "release_authority",
        "receipt_truth",
        "balance_truth",
        "payout_truth",
        "paid_unlock_authority",
        "fake_receipt",
        "fake_balance",
        "fake_finality",
        "bridge_authority",
        "external_settlement_authority",
        "rox_solana_authority",
        "raw_engagement_mint_authority",
    ] {
        let mut value = json!({
            "schema": "ron-accounting.internal-roc-stabilization-snapshot-non-authority.v1",
            "source_crate": "ron-accounting",
            "role": "derivative_metering_snapshot_report_infrastructure",
            "wallet_truth": "svc-wallet_only",
            "ledger_truth": "ron-ledger_only",
            "snapshot_role": "deterministic_derivative_artifact_only",
            "payout_role": "planning_input_only_not_execution",
            "artifact_role": "b3_cid_reference_not_root_or_finality"
        });

        value
            .as_object_mut()
            .expect("contract object")
            .insert(field.to_owned(), json!(true));

        let err = serde_json::from_value::<AccountingSnapshotNonAuthorityContract>(value)
            .expect_err("authority poison fields must reject");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn stabilization_doc_states_accounting_product_beta_truth_boundary() {
    let doc = read_rel("docs/internal-roc-stabilization-snapshot-non-authority.md");

    assert_all(
        "ron-accounting stabilization doc",
        &doc,
        &[
            "derivative metering, snapshot, report, and reward-planning input infrastructure",
            "accounting may observe accepted wallet/ledger-derived receipts",
            "accounting must never mutate wallet/ledger state",
            "usage event ingest",
            "authority-poison rejection",
            "deterministic snapshot artifacts",
            "downstream reward-planning input",
            "accounting as balance truth",
            "accounting as receipt truth",
            "accounting as payout truth",
            "raw-engagement direct mint authority",
        ],
    );
}

fn assert_all(label: &str, haystack: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            haystack.contains(needle),
            "{label} must contain required marker {needle:?}"
        );
    }
}

fn assert_none(label: &str, haystack: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            !haystack.contains(needle),
            "{label} must not contain forbidden marker {needle:?}"
        );
    }
}

fn read_rel(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}
