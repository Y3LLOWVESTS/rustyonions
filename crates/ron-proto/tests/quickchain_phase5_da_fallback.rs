//! RO:WHAT — Tests QuickChain Phase 5 Round 2 DA/archive/challenge fallback DTOs.
//! RO:WHY — ECON/GOV: fallback evidence must stay strict, dry-run, non-mutating, and pruning-blocking.
//! RO:INTERACTS — ron_proto::quickchain::da_fallback.
//! RO:INVARIANTS — no pruning grant, no settlement, no bridge, no balance mutation, no fake finality.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — DTO strictness test only; no secrets or service calls.
//! RO:TEST — cargo test -p ron-proto --test quickchain_phase5_da_fallback.

use ron_proto::{
    ContentId, QuickChainDaChallengeReportV1, QuickChainDaChallengeStatusV1,
    QuickChainDaChunkCommitmentV1, QuickChainDaChunkKindV1, QuickChainDaFallbackModeV1,
    QuickChainDaFallbackPlanV1, QuickChainDaFallbackVerificationV1, QuickChainDaRetentionClassV1,
    QUICKCHAIN_DA_CHALLENGE_REPORT_SCHEMA, QUICKCHAIN_DA_CHUNK_COMMITMENT_SCHEMA,
    QUICKCHAIN_DA_FALLBACK_PLAN_SCHEMA, QUICKCHAIN_DA_FALLBACK_VERIFICATION_SCHEMA,
    QUICKCHAIN_DTO_VERSION,
};
use serde_json::json;

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().unwrap()
}

fn economic_chunk(id: &str, cid_ch: char) -> QuickChainDaChunkCommitmentV1 {
    QuickChainDaChunkCommitmentV1 {
        schema: QUICKCHAIN_DA_CHUNK_COMMITMENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chunk_id: id.to_string(),
        chunk_kind: QuickChainDaChunkKindV1::EconomicReceiptBatch,
        chunk_cid: cid(cid_ch),
        byte_len: 4096,
        retention_class: QuickChainDaRetentionClassV1::Hot,
        archive_ref: format!("archive:phase5-r2:{id}"),
        restore_ref: format!("restore:phase5-r2:{id}"),
        required_for_challenge: true,
        available: true,
        restore_tested: true,
        analytics_only: false,
    }
}

fn analytics_chunk(id: &str, cid_ch: char) -> QuickChainDaChunkCommitmentV1 {
    QuickChainDaChunkCommitmentV1 {
        schema: QUICKCHAIN_DA_CHUNK_COMMITMENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chunk_id: id.to_string(),
        chunk_kind: QuickChainDaChunkKindV1::AnalyticsSummary,
        chunk_cid: cid(cid_ch),
        byte_len: 1024,
        retention_class: QuickChainDaRetentionClassV1::Light,
        archive_ref: format!("archive:phase5-r2:{id}"),
        restore_ref: format!("restore:phase5-r2:{id}"),
        required_for_challenge: false,
        available: true,
        restore_tested: true,
        analytics_only: true,
    }
}

fn plan() -> QuickChainDaFallbackPlanV1 {
    QuickChainDaFallbackPlanV1 {
        schema: QUICKCHAIN_DA_FALLBACK_PLAN_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        epoch_id: "epoch:phase5-r2".to_string(),
        fallback_plan_id: "da-fallback:phase5-r2:001".to_string(),
        checkpoint_height: 9,
        checkpoint_hash: cid('a'),
        data_availability_root: cid('b'),
        fallback_mode: QuickChainDaFallbackModeV1::MissingDataChallengeDryRun,
        challenge_window_start_ms: 1_800_000_000_000,
        challenge_window_end_ms: 1_800_086_400_000,
        produced_at_ms: 1_800_000_100_000,
        chunks: vec![
            economic_chunk("chunk:0001", '1'),
            analytics_chunk("chunk:0002", '2'),
        ],
        pruning_allowed: false,
        archive_fallback_required: true,
        missing_data_challenge_supported: true,
        restore_from_archive_tested: true,
        dry_run_only: true,
        normal_node_full_archive_required: false,
        external_da_truth: false,
        external_settlement_authorized: false,
        bridge_authorized: false,
        balance_mutation_authorized: false,
        wallet_ledger_truth_replaced: false,
        finality_claimed: false,
    }
}

fn report() -> QuickChainDaChallengeReportV1 {
    QuickChainDaChallengeReportV1 {
        schema: QUICKCHAIN_DA_CHALLENGE_REPORT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        epoch_id: "epoch:phase5-r2".to_string(),
        fallback_plan_id: "da-fallback:phase5-r2:001".to_string(),
        challenge_id: "da-challenge:phase5-r2:001".to_string(),
        checkpoint_hash: cid('a'),
        challenged_chunk_id: "chunk:0001".to_string(),
        challenger_ref: "passport:watcher".to_string(),
        evidence_cid: cid('c'),
        submitted_at_ms: 1_800_000_200_000,
        status: QuickChainDaChallengeStatusV1::MissingDataChallengeOpen,
        dry_run_only: true,
        challenge_window_open: true,
        pruning_blocked: true,
        penalty_authorized: false,
        archive_reward_authorized: false,
        balance_mutation_authorized: false,
        external_settlement_authorized: false,
        finality_claimed: false,
    }
}

fn verification() -> QuickChainDaFallbackVerificationV1 {
    QuickChainDaFallbackVerificationV1 {
        schema: QUICKCHAIN_DA_FALLBACK_VERIFICATION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        epoch_id: "epoch:phase5-r2".to_string(),
        fallback_plan_id: "da-fallback:phase5-r2:001".to_string(),
        checkpoint_height: 9,
        checkpoint_hash: cid('a'),
        data_availability_root: cid('b'),
        checked_chunk_count: 2,
        pruning_blocked: true,
        archive_fallback_checked: true,
        missing_data_challenge_checked: true,
        restore_path_checked: true,
        dry_run_only: true,
        external_da_truth: false,
        external_settlement_authorized: false,
        bridge_authorized: false,
        balance_mutation_authorized: false,
        wallet_ledger_truth_replaced: false,
        finality_claimed: false,
    }
}

#[test]
fn da_fallback_plan_validates_and_roundtrips_json() {
    let dto = plan();

    dto.validate().unwrap();

    let encoded = serde_json::to_string(&dto).unwrap();
    let decoded: QuickChainDaFallbackPlanV1 = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, dto);
    decoded.validate().unwrap();
}

#[test]
fn da_fallback_plan_rejects_unknown_fields() {
    let mut value = serde_json::to_value(plan()).unwrap();
    value["pruning_grant"] = json!(true);

    let err = serde_json::from_value::<QuickChainDaFallbackPlanV1>(value).unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn da_fallback_plan_rejects_pruning_and_authority_claims() {
    let mut dto = plan();
    dto.pruning_allowed = true;
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.archive_fallback_required = false;
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.missing_data_challenge_supported = false;
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.restore_from_archive_tested = false;
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.dry_run_only = false;
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.normal_node_full_archive_required = true;
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.external_da_truth = true;
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.external_settlement_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.bridge_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.balance_mutation_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.wallet_ledger_truth_replaced = true;
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.finality_claimed = true;
    assert!(dto.validate().is_err());
}

#[test]
fn da_fallback_plan_rejects_bad_windows_and_unsorted_chunks() {
    let mut dto = plan();
    dto.challenge_window_end_ms = dto.challenge_window_start_ms;
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.produced_at_ms = dto.challenge_window_end_ms + 1;
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.chunks.reverse();
    assert!(dto.validate().is_err());

    let mut dto = plan();
    dto.chunks.clear();
    assert!(dto.validate().is_err());
}

#[test]
fn da_chunk_commitment_rejects_invalid_economic_and_analytics_shapes() {
    let mut chunk = economic_chunk("chunk:0001", '1');
    chunk.byte_len = 0;
    assert!(chunk.validate().is_err());

    let mut chunk = economic_chunk("chunk:0001", '1');
    chunk.available = false;
    chunk.restore_tested = true;
    assert!(chunk.validate().is_err());

    let mut chunk = economic_chunk("chunk:0001", '1');
    chunk.analytics_only = true;
    assert!(chunk.validate().is_err());

    let mut chunk = analytics_chunk("chunk:0002", '2');
    chunk.required_for_challenge = true;
    assert!(chunk.validate().is_err());

    let mut chunk = analytics_chunk("chunk:0002", '2');
    chunk.analytics_only = false;
    assert!(chunk.validate().is_err());
}

#[test]
fn da_challenge_report_validates_and_rejects_authority_claims() {
    let dto = report();
    dto.validate().unwrap();

    let encoded = serde_json::to_string(&dto).unwrap();
    let decoded: QuickChainDaChallengeReportV1 = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, dto);

    let mut dto = report();
    dto.dry_run_only = false;
    assert!(dto.validate().is_err());

    let mut dto = report();
    dto.pruning_blocked = false;
    assert!(dto.validate().is_err());

    let mut dto = report();
    dto.penalty_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = report();
    dto.archive_reward_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = report();
    dto.balance_mutation_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = report();
    dto.external_settlement_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = report();
    dto.finality_claimed = true;
    assert!(dto.validate().is_err());
}

#[test]
fn da_challenge_report_status_window_rules_are_strict() {
    let mut dto = report();
    dto.challenge_window_open = false;
    assert!(dto.validate().is_err());

    let mut dto = report();
    dto.status = QuickChainDaChallengeStatusV1::MissingDataRestored;
    dto.challenge_window_open = true;
    assert!(dto.validate().is_err());

    let mut dto = report();
    dto.status = QuickChainDaChallengeStatusV1::MissingDataRestored;
    dto.challenge_window_open = false;
    assert!(dto.validate().is_ok());

    let mut value = serde_json::to_value(report()).unwrap();
    value["settlement_authority"] = json!(true);
    let err = serde_json::from_value::<QuickChainDaChallengeReportV1>(value).unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn da_fallback_verification_validates_and_rejects_authority_claims() {
    let dto = verification();
    dto.validate().unwrap();

    let encoded = serde_json::to_string(&dto).unwrap();
    let decoded: QuickChainDaFallbackVerificationV1 = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, dto);

    let mut dto = verification();
    dto.checked_chunk_count = 0;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.pruning_blocked = false;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.archive_fallback_checked = false;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.missing_data_challenge_checked = false;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.restore_path_checked = false;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.dry_run_only = false;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.external_da_truth = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.external_settlement_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.bridge_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.balance_mutation_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.wallet_ledger_truth_replaced = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.finality_claimed = true;
    assert!(dto.validate().is_err());
}
