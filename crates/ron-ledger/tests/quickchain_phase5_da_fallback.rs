#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Tests ron-ledger Phase 5 Round 2 DA/archive/challenge fallback helpers.
//! RO:WHY — ECON/GOV: fallback evidence may be exported and checked, but pruning and authority stay blocked.
//! RO:INTERACTS — ron_ledger::quickchain::da_fallback, ron_proto QuickChain DTOs.
//! RO:INVARIANTS — no ledger mutation, no wallet mutation, no pruning grant, no settlement, no bridge, no fake finality.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — evidence-only tests; no secrets, IO, services, or external calls.
//! RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase5_da_fallback.

use ron_ledger::quickchain::{
    export_da_fallback_plan, verify_da_challenge_report_against_plan, verify_da_fallback_plan,
    QuickChainDaFallbackExportContext,
};
use ron_proto::{
    quickchain::{
        QuickChainCanonicalEncodingV1, QuickChainCheckpointHeaderV1, QuickChainDaChallengeReportV1,
        QuickChainDaChallengeStatusV1, QuickChainDaChunkCommitmentV1, QuickChainDaChunkKindV1,
        QuickChainDaFallbackModeV1, QuickChainDaRetentionClassV1, QuickChainReceiptRootSchemeV1,
        QuickChainStateRootSchemeV1, QUICKCHAIN_CHECKPOINT_HEADER_SCHEMA,
        QUICKCHAIN_DA_CHALLENGE_REPORT_SCHEMA, QUICKCHAIN_DA_CHUNK_COMMITMENT_SCHEMA,
        QUICKCHAIN_DTO_VERSION,
    },
    ContentId,
};
use std::{fs, path::Path};

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().unwrap()
}

fn checkpoint() -> QuickChainCheckpointHeaderV1 {
    QuickChainCheckpointHeaderV1 {
        schema: QUICKCHAIN_CHECKPOINT_HEADER_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        height: 9,
        epoch_id: "epoch:phase5-r2".to_string(),
        previous_checkpoint_hash: cid('0'),
        previous_state_root: cid('1'),
        new_state_root: cid('2'),
        receipt_root: cid('3'),
        accounting_snapshot_root: cid('4'),
        reward_manifest_root: cid('5'),
        data_availability_root: cid('6'),
        policy_hash: cid('7'),
        validator_set_hash: cid('8'),
        chain_params_hash: cid('9'),
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        state_root_scheme: QuickChainStateRootSchemeV1::SortedMerkleMapV1,
        receipt_root_scheme: QuickChainReceiptRootSchemeV1::LedgerSequenceMerkleV1,
        supply_delta_minor_units: "0".to_string(),
        started_at_ms: 1_800_000_000_000,
        ended_at_ms: 1_800_000_010_000,
        produced_at_ms: 1_800_000_020_000,
        signatures: Vec::new(),
    }
}

fn context() -> QuickChainDaFallbackExportContext {
    QuickChainDaFallbackExportContext::new(
        "da-fallback:phase5-r2:001",
        QuickChainDaFallbackModeV1::MissingDataChallengeDryRun,
        1_800_000_020_000,
        1_800_086_400_000,
        1_800_000_030_000,
    )
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

fn chunks() -> Vec<QuickChainDaChunkCommitmentV1> {
    vec![
        economic_chunk("chunk:0001", 'a'),
        QuickChainDaChunkCommitmentV1 {
            schema: QUICKCHAIN_DA_CHUNK_COMMITMENT_SCHEMA.to_string(),
            version: QUICKCHAIN_DTO_VERSION,
            chunk_id: "chunk:0002".to_string(),
            chunk_kind: QuickChainDaChunkKindV1::AnalyticsSummary,
            chunk_cid: cid('b'),
            byte_len: 1024,
            retention_class: QuickChainDaRetentionClassV1::Light,
            archive_ref: "archive:phase5-r2:chunk:0002".to_string(),
            restore_ref: "restore:phase5-r2:chunk:0002".to_string(),
            required_for_challenge: false,
            available: true,
            restore_tested: true,
            analytics_only: true,
        },
    ]
}

fn challenge_report() -> QuickChainDaChallengeReportV1 {
    QuickChainDaChallengeReportV1 {
        schema: QUICKCHAIN_DA_CHALLENGE_REPORT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        epoch_id: "epoch:phase5-r2".to_string(),
        fallback_plan_id: "da-fallback:phase5-r2:001".to_string(),
        challenge_id: "da-challenge:phase5-r2:001".to_string(),
        checkpoint_hash: cid('c'),
        challenged_chunk_id: "chunk:0001".to_string(),
        challenger_ref: "passport:watcher".to_string(),
        evidence_cid: cid('d'),
        submitted_at_ms: 1_800_000_040_000,
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

#[test]
fn exports_and_verifies_da_fallback_plan_without_pruning_authority() {
    let checkpoint = checkpoint();
    let checkpoint_hash = cid('c');

    let plan = export_da_fallback_plan(&checkpoint, checkpoint_hash.clone(), &context(), chunks())
        .unwrap();

    assert_eq!(plan.chain_id, checkpoint.chain_id);
    assert_eq!(plan.epoch_id, checkpoint.epoch_id);
    assert_eq!(plan.checkpoint_height, checkpoint.height);
    assert_eq!(plan.checkpoint_hash, checkpoint_hash);
    assert_eq!(
        plan.data_availability_root,
        checkpoint.data_availability_root
    );
    assert!(!plan.pruning_allowed);
    assert!(plan.archive_fallback_required);
    assert!(plan.missing_data_challenge_supported);
    assert!(plan.restore_from_archive_tested);
    assert!(plan.dry_run_only);
    assert!(!plan.external_da_truth);
    assert!(!plan.balance_mutation_authorized);
    assert!(!plan.finality_claimed);

    let verification = verify_da_fallback_plan(&plan, &checkpoint, &checkpoint_hash).unwrap();

    assert_eq!(verification.checked_chunk_count, 2);
    assert!(verification.pruning_blocked);
    assert!(verification.archive_fallback_checked);
    assert!(verification.missing_data_challenge_checked);
    assert!(verification.restore_path_checked);
    assert!(verification.dry_run_only);
    assert!(!verification.external_da_truth);
    assert!(!verification.balance_mutation_authorized);
    assert!(!verification.finality_claimed);
}

#[test]
fn da_fallback_verification_rejects_checkpoint_hash_mismatch() {
    let checkpoint = checkpoint();
    let plan = export_da_fallback_plan(&checkpoint, cid('c'), &context(), chunks()).unwrap();

    let err = verify_da_fallback_plan(&plan, &checkpoint, &cid('e')).unwrap_err();

    assert_eq!(
        err.to_string(),
        "QuickChain DA fallback plan mismatch: checkpoint_hash"
    );
}

#[test]
fn da_fallback_verification_rejects_data_availability_root_mismatch() {
    let checkpoint = checkpoint();
    let plan = export_da_fallback_plan(&checkpoint, cid('c'), &context(), chunks()).unwrap();

    let mut tampered_checkpoint = checkpoint.clone();
    tampered_checkpoint.data_availability_root = cid('f');

    let err = verify_da_fallback_plan(&plan, &tampered_checkpoint, &cid('c')).unwrap_err();

    assert_eq!(
        err.to_string(),
        "QuickChain DA fallback plan mismatch: data_availability_root"
    );
}

#[test]
fn da_fallback_export_rejects_invalid_context_or_chunks() {
    let checkpoint = checkpoint();

    let invalid_context = QuickChainDaFallbackExportContext::new(
        "da-fallback:phase5-r2:001",
        QuickChainDaFallbackModeV1::MissingDataChallengeDryRun,
        1_800_000_020_000,
        1_800_086_400_000,
        0,
    );
    let err =
        export_da_fallback_plan(&checkpoint, cid('c'), &invalid_context, chunks()).unwrap_err();
    assert!(err
        .to_string()
        .contains("invalid QuickChain DA fallback plan"));

    let mut bad_chunks = chunks();
    bad_chunks.reverse();
    let err = export_da_fallback_plan(&checkpoint, cid('c'), &context(), bad_chunks).unwrap_err();
    assert!(err
        .to_string()
        .contains("invalid QuickChain DA fallback plan"));
}

#[test]
fn da_challenge_report_matches_plan_without_side_effects() {
    let checkpoint = checkpoint();
    let checkpoint_hash = cid('c');
    let plan = export_da_fallback_plan(&checkpoint, checkpoint_hash, &context(), chunks()).unwrap();

    let report = challenge_report();

    verify_da_challenge_report_against_plan(&report, &plan).unwrap();
}

#[test]
fn da_challenge_report_rejects_wrong_chunk_or_checkpoint() {
    let checkpoint = checkpoint();
    let plan = export_da_fallback_plan(&checkpoint, cid('c'), &context(), chunks()).unwrap();

    let mut wrong_chunk = challenge_report();
    wrong_chunk.challenged_chunk_id = "chunk:9999".to_string();

    let err = verify_da_challenge_report_against_plan(&wrong_chunk, &plan).unwrap_err();
    assert_eq!(
        err.to_string(),
        "QuickChain DA challenge report mismatch: challenged_chunk_id"
    );

    let mut wrong_checkpoint = challenge_report();
    wrong_checkpoint.checkpoint_hash = cid('e');

    let err = verify_da_challenge_report_against_plan(&wrong_checkpoint, &plan).unwrap_err();
    assert_eq!(
        err.to_string(),
        "QuickChain DA fallback plan mismatch: checkpoint_hash"
    );
}

#[test]
fn da_fallback_source_stays_inert_and_non_mutating() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest.join("src/quickchain/da_fallback.rs");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let compact = strip_comments_and_literals(&raw).to_ascii_lowercase();

    for forbidden in [
        "std::fs",
        "std::net",
        "tokio::",
        "reqwest::",
        "axum::",
        "systemtime",
        "unix_epoch",
        "rand::",
        ".spawn(",
        "mint(",
        "transfer(",
        "burn(",
        "open_hold(",
        "capture_hold(",
        "release_hold(",
        "expire_hold(",
        "unlock_paid",
        "settle_external",
        "authorize_pruning(",
    ] {
        assert!(
            !compact.contains(forbidden),
            "da_fallback.rs must remain inert and non-mutating; found `{forbidden}`"
        );
    }
}

fn strip_comments_and_literals(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                if i < bytes.len() {
                    out.push('\n');
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < bytes.len() {
                    if bytes[i] == b'\n' {
                        out.push('\n');
                    }

                    if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                        i += 2;
                        break;
                    }

                    i += 1;
                }
            }
            b'"' => {
                out.push('"');
                i += 1;

                while i < bytes.len() {
                    match bytes[i] {
                        b'\\' if i + 1 < bytes.len() => {
                            out.push(' ');
                            out.push(' ');
                            i += 2;
                        }
                        b'"' => {
                            out.push('"');
                            i += 1;
                            break;
                        }
                        b'\n' => {
                            out.push('\n');
                            i += 1;
                        }
                        _ => {
                            out.push(' ');
                            i += 1;
                        }
                    }
                }
            }
            b'\'' => {
                out.push('\'');
                i += 1;

                while i < bytes.len() {
                    match bytes[i] {
                        b'\\' if i + 1 < bytes.len() => {
                            out.push(' ');
                            out.push(' ');
                            i += 2;
                        }
                        b'\'' => {
                            out.push('\'');
                            i += 1;
                            break;
                        }
                        b'\n' => {
                            out.push('\n');
                            i += 1;
                            break;
                        }
                        _ => {
                            out.push(' ');
                            i += 1;
                        }
                    }
                }
            }
            byte => {
                out.push(byte as char);
                i += 1;
            }
        }
    }

    out
}
