#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — FINAL_BETA Phase 19 integrated checkpoint-to-DA/archive fallback proof.
//! RO:WHY — The live committee checkpoint commits a data_availability_root,
//! so missing-data/archive recovery must stay bound to that exact checkpoint
//! hash and DA commitment.
//! RO:INTERACTS — Phase 19 committee checkpoint payload, canonical checkpoint
//! hashing, existing ron-ledger DA fallback plan/verification.
//! RO:INVARIANTS — changing the DA root changes the checkpoint hash; fallback
//! evidence must match both; archive/challenge/restore remain evidence-only;
//! pruning stays blocked; no wallet/ledger/finality authority is created.
//! RO:SECURITY — deterministic local acceptance proof only. No network,
//! external archive calls, wallet mutation, ledger mutation, settlement,
//! pruning authority, or fake finality.
//! RO:TEST — this file plus the existing ron-ledger and svc-storage DA tests.

use ron_ledger::quickchain::{
    export_da_fallback_plan, verify_da_fallback_plan, QuickChainDaFallbackExportContext,
};

use ron_proto::{
    quickchain::{
        QuickChainCanonicalEncodingV1, QuickChainCheckpointHeaderV1, QuickChainDaChunkCommitmentV1,
        QuickChainDaChunkKindV1, QuickChainDaFallbackModeV1, QuickChainDaRetentionClassV1,
        QuickChainReceiptRootSchemeV1, QuickChainStateRootSchemeV1,
        QUICKCHAIN_CHECKPOINT_HEADER_SCHEMA, QUICKCHAIN_DA_CHUNK_COMMITMENT_SCHEMA,
        QUICKCHAIN_DTO_VERSION,
    },
    to_canonical_json_vec, ContentId, QuickChainCommitteeCheckpointPayloadV1,
    QuickChainConservationV1, QuickChainSettlementModeV1, QuickChainSupplyDeltaV1,
    QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1, QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA,
};

const CHAIN_ID: &str = "roc-dev";

const EPOCH_ID: &str = "epoch:phase19";

fn cid(label: &str) -> ContentId {
    format!("b3:{}", blake3::hash(label.as_bytes(),).to_hex(),)
        .parse()
        .expect("fixture ContentId must parse")
}

fn committee_checkpoint() -> QuickChainCommitteeCheckpointPayloadV1 {
    QuickChainCommitteeCheckpointPayloadV1 {
        schema: QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA.to_owned(),

        version: QUICKCHAIN_DTO_VERSION,

        chain_id: CHAIN_ID.to_owned(),

        height: 19,

        epoch_id: EPOCH_ID.to_owned(),

        execution_spec_version: "quickchain-execution-v1".to_owned(),

        previous_checkpoint_hash: cid("phase19-da-previous-checkpoint"),

        previous_state_root: cid("phase19-da-previous-state"),

        new_state_root: cid("phase19-da-new-state"),

        receipt_root: cid("phase19-da-receipt-root"),

        accounting_snapshot_root: cid("phase19-da-accounting-root"),

        reward_manifest_root: cid("phase19-da-reward-root"),

        data_availability_root: cid("phase19-da-root"),

        policy_hash: cid("phase19-da-policy"),

        validator_set_hash: cid("phase19-da-validator-set"),

        chain_params_hash: cid("phase19-da-chain-params"),

        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,

        state_root_scheme: QuickChainStateRootSchemeV1::SortedMerkleMapV1,

        receipt_root_scheme: QuickChainReceiptRootSchemeV1::LedgerSequenceMerkleV1,

        supply_delta: QuickChainSupplyDeltaV1 {
            issued_minor: "0".to_owned(),

            burned_minor: "0".to_owned(),

            net_minor: "0".to_owned(),
        },

        conservation: QuickChainConservationV1 {
            debits_minor: "100".to_owned(),

            credits_minor: "100".to_owned(),

            issue_exceptions_minor: "0".to_owned(),

            burn_exceptions_minor: "0".to_owned(),

            valid: true,
        },

        settlement_mode: QuickChainSettlementModeV1::LocalRoot,

        started_at_ms: 1_800_000_000_000,

        ended_at_ms: 1_800_000_060_000,

        produced_at_ms: 1_800_000_061_000,
    }
}

fn checkpoint_hash(payload: &QuickChainCommitteeCheckpointPayloadV1) -> ContentId {
    payload
        .validate()
        .expect("Phase 19 committee checkpoint must validate");

    let canonical =
        to_canonical_json_vec(payload).expect("Phase 19 committee checkpoint must canonicalize");

    let mut hasher = blake3::Hasher::new();

    hasher.update(QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1.as_bytes());

    hasher.update(&[0]);

    hasher.update(&canonical);

    format!("b3:{}", hasher.finalize().to_hex(),)
        .parse()
        .expect("Phase 19 checkpoint hash must parse")
}

fn fallback_header(
    candidate: &QuickChainCommitteeCheckpointPayloadV1,
) -> QuickChainCheckpointHeaderV1 {
    QuickChainCheckpointHeaderV1 {
        schema: QUICKCHAIN_CHECKPOINT_HEADER_SCHEMA.to_owned(),

        version: QUICKCHAIN_DTO_VERSION,

        chain_id: candidate.chain_id.clone(),

        height: candidate.height,

        epoch_id: candidate.epoch_id.clone(),

        previous_checkpoint_hash: candidate.previous_checkpoint_hash.clone(),

        previous_state_root: candidate.previous_state_root.clone(),

        new_state_root: candidate.new_state_root.clone(),

        receipt_root: candidate.receipt_root.clone(),

        accounting_snapshot_root: candidate.accounting_snapshot_root.clone(),

        reward_manifest_root: candidate.reward_manifest_root.clone(),

        data_availability_root: candidate.data_availability_root.clone(),

        policy_hash: candidate.policy_hash.clone(),

        validator_set_hash: candidate.validator_set_hash.clone(),

        chain_params_hash: candidate.chain_params_hash.clone(),

        canonical_encoding: candidate.canonical_encoding,

        state_root_scheme: candidate.state_root_scheme,

        receipt_root_scheme: candidate.receipt_root_scheme,

        supply_delta_minor_units: candidate.supply_delta.net_minor.clone(),

        started_at_ms: candidate.started_at_ms,

        ended_at_ms: candidate.ended_at_ms,

        produced_at_ms: candidate.produced_at_ms,

        signatures: Vec::new(),
    }
}

fn fallback_context() -> QuickChainDaFallbackExportContext {
    QuickChainDaFallbackExportContext::new(
        "da-fallback:phase19:001",
        QuickChainDaFallbackModeV1::MissingDataChallengeDryRun,
        1_800_000_061_000,
        1_800_086_400_000,
        1_800_000_062_000,
    )
}

fn chunks() -> Vec<QuickChainDaChunkCommitmentV1> {
    vec![
        QuickChainDaChunkCommitmentV1 {
            schema: QUICKCHAIN_DA_CHUNK_COMMITMENT_SCHEMA.to_owned(),

            version: QUICKCHAIN_DTO_VERSION,

            chunk_id: "chunk:phase19:0001".to_owned(),

            chunk_kind: QuickChainDaChunkKindV1::EconomicReceiptBatch,

            chunk_cid: cid("phase19-da-economic-receipt-batch"),

            byte_len: 4096,

            retention_class: QuickChainDaRetentionClassV1::Hot,

            archive_ref: "archive:phase19:chunk:0001".to_owned(),

            restore_ref: "restore:phase19:chunk:0001".to_owned(),

            required_for_challenge: true,

            available: true,

            restore_tested: true,

            analytics_only: false,
        },
        QuickChainDaChunkCommitmentV1 {
            schema: QUICKCHAIN_DA_CHUNK_COMMITMENT_SCHEMA.to_owned(),

            version: QUICKCHAIN_DTO_VERSION,

            chunk_id: "chunk:phase19:0002".to_owned(),

            chunk_kind: QuickChainDaChunkKindV1::AnalyticsSummary,

            chunk_cid: cid("phase19-da-analytics-summary"),

            byte_len: 1024,

            retention_class: QuickChainDaRetentionClassV1::Light,

            archive_ref: "archive:phase19:chunk:0002".to_owned(),

            restore_ref: "restore:phase19:chunk:0002".to_owned(),

            required_for_challenge: false,

            available: true,

            restore_tested: true,

            analytics_only: true,
        },
    ]
}

#[test]
fn phase19_committee_checkpoint_binds_existing_da_archive_fallback_contract() {
    let candidate = committee_checkpoint();

    let candidate_hash = checkpoint_hash(&candidate);

    let header = fallback_header(&candidate);

    header
        .validate()
        .expect("Phase 19 DA bridge header must validate");

    assert_eq!(header.chain_id, candidate.chain_id,);

    assert_eq!(header.epoch_id, candidate.epoch_id,);

    assert_eq!(header.height, candidate.height,);

    assert_eq!(
        header.data_availability_root,
        candidate.data_availability_root,
    );

    assert_eq!(header.validator_set_hash, candidate.validator_set_hash,);

    let plan = export_da_fallback_plan(
        &header,
        candidate_hash.clone(),
        &fallback_context(),
        chunks(),
    )
    .expect("Phase 19 checkpoint must export existing DA fallback evidence");

    assert_eq!(plan.checkpoint_hash, candidate_hash,);

    assert_eq!(
        plan.data_availability_root,
        candidate.data_availability_root,
    );

    assert_eq!(plan.checkpoint_height, candidate.height,);

    assert_eq!(plan.chain_id, candidate.chain_id,);

    assert_eq!(plan.epoch_id, candidate.epoch_id,);

    assert!(plan.archive_fallback_required,);

    assert!(plan.missing_data_challenge_supported,);

    assert!(plan.restore_from_archive_tested,);

    assert!(plan.dry_run_only,);

    assert!(!plan.pruning_allowed,);

    assert!(!plan.external_da_truth,);

    assert!(!plan.external_settlement_authorized,);

    assert!(!plan.bridge_authorized,);

    assert!(!plan.balance_mutation_authorized,);

    assert!(!plan.wallet_ledger_truth_replaced,);

    assert!(!plan.finality_claimed,);

    let verification = verify_da_fallback_plan(&plan, &header, &candidate_hash)
        .expect("Phase 19 DA fallback evidence must verify against exact checkpoint commitment");

    assert_eq!(verification.checked_chunk_count, 2,);

    assert!(verification.pruning_blocked,);

    assert!(verification.archive_fallback_checked,);

    assert!(verification.missing_data_challenge_checked,);

    assert!(verification.restore_path_checked,);

    assert!(verification.dry_run_only,);

    assert!(!verification.external_da_truth,);

    assert!(!verification.balance_mutation_authorized,);

    assert!(!verification.finality_claimed,);
}

#[test]
fn changing_phase19_da_root_changes_checkpoint_hash_and_invalidates_existing_fallback_binding() {
    let original = committee_checkpoint();

    let original_hash = checkpoint_hash(&original);

    let original_header = fallback_header(&original);

    let plan = export_da_fallback_plan(
        &original_header,
        original_hash.clone(),
        &fallback_context(),
        chunks(),
    )
    .expect("original Phase 19 fallback plan must export");

    let mut changed = original.clone();

    changed.data_availability_root = cid("phase19-different-da-root");

    changed
        .validate()
        .expect("changed DA commitment remains structurally valid");

    let changed_hash = checkpoint_hash(&changed);

    assert_ne!(
        changed_hash, original_hash,
        "changing the committed DA root must change the canonical checkpoint hash",
    );

    let changed_header = fallback_header(&changed);

    let old_hash_error = verify_da_fallback_plan(&plan, &changed_header, &original_hash)
        .expect_err("old fallback plan must reject changed checkpoint DA root");

    assert_eq!(
        old_hash_error.to_string(),
        "QuickChain DA fallback plan mismatch: data_availability_root",
    );

    let changed_hash_error = verify_da_fallback_plan(&plan, &changed_header, &changed_hash)
        .expect_err("old fallback plan must also reject the newly derived checkpoint hash");

    assert_eq!(
        changed_hash_error.to_string(),
        "QuickChain DA fallback plan mismatch: checkpoint_hash",
    );
}

#[test]
fn fallback_evidence_never_upgrades_archive_recovery_into_protocol_authority() {
    let candidate = committee_checkpoint();

    let candidate_hash = checkpoint_hash(&candidate);

    let header = fallback_header(&candidate);

    let plan = export_da_fallback_plan(
        &header,
        candidate_hash.clone(),
        &fallback_context(),
        chunks(),
    )
    .expect("Phase 19 fallback plan must export");

    let verification = verify_da_fallback_plan(&plan, &header, &candidate_hash)
        .expect("Phase 19 fallback plan must verify");

    assert!(
        verification.archive_fallback_checked,
        "archive recovery evidence must be checked",
    );

    assert!(
        verification.restore_path_checked,
        "restore evidence must be checked",
    );

    assert!(
        verification.missing_data_challenge_checked,
        "missing-data challenge capability must be checked",
    );

    assert!(
        verification.pruning_blocked,
        "DA fallback evidence must not itself grant pruning",
    );

    assert!(
        verification.dry_run_only,
        "existing fallback verification remains non-authoritative",
    );

    assert!(
        !verification.external_da_truth,
        "archive evidence must not become external DA truth",
    );

    assert!(
        !verification.balance_mutation_authorized,
        "archive evidence must not mutate economic truth",
    );

    assert!(
        !verification.finality_claimed,
        "archive evidence must not manufacture finality",
    );
}
