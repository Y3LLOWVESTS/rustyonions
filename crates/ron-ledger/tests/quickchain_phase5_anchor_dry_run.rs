#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 5 Round 1 tests for inert anchor-only checkpoint commitment dry-runs.
//! RO:WHY — ECON/GOV: prove ron-ledger can export and verify compact commitments without external settlement or balance mutation.
//! RO:INTERACTS — ron_ledger::quickchain anchor dry-run helpers and ron-proto checkpoint/anchor DTOs.
//! RO:INVARIANTS — no wallet mutation; no balance mutation; no external-chain truth; no bridge; commitment-only verification.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — tests evidence-only DTO behavior; no network, file artifact writing, keys, validators, or spend authority.
//! RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase5_anchor_dry_run.

use std::{fs, path::Path};

use ron_ledger::quickchain::{
    export_anchor_dry_run_commitment, verify_anchor_dry_run_commitment,
    QuickChainAnchorDryRunError, QuickChainAnchorDryRunExportContext,
};
use ron_proto::{
    ContentId, QuickChainCanonicalEncodingV1, QuickChainCheckpointHeaderV1,
    QuickChainReceiptRootSchemeV1, QuickChainStateRootSchemeV1,
    QUICKCHAIN_CHECKPOINT_HEADER_SCHEMA, QUICKCHAIN_DTO_VERSION,
};

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().unwrap()
}

fn checkpoint() -> QuickChainCheckpointHeaderV1 {
    QuickChainCheckpointHeaderV1 {
        schema: QUICKCHAIN_CHECKPOINT_HEADER_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        height: 7,
        epoch_id: "epoch:phase5-r1".to_string(),
        previous_checkpoint_hash: cid('0'),
        previous_state_root: cid('9'),
        new_state_root: cid('1'),
        receipt_root: cid('2'),
        accounting_snapshot_root: cid('3'),
        reward_manifest_root: cid('4'),
        data_availability_root: cid('5'),
        policy_hash: cid('6'),
        validator_set_hash: cid('7'),
        chain_params_hash: cid('8'),
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        state_root_scheme: QuickChainStateRootSchemeV1::SortedMerkleMapV1,
        receipt_root_scheme: QuickChainReceiptRootSchemeV1::LedgerSequenceMerkleV1,
        supply_delta_minor_units: "0".to_string(),
        started_at_ms: 1_800_000_000_000,
        ended_at_ms: 1_800_000_060_000,
        produced_at_ms: 1_800_000_061_000,
        signatures: Vec::new(),
    }
}

fn context() -> QuickChainAnchorDryRunExportContext {
    QuickChainAnchorDryRunExportContext::new(
        "anchor:phase5-r1:001",
        "dry-run/local-file",
        "dry-run:phase5-r1:artifact-001",
        "file:quickchain-phase5-r1-anchor-001.json",
        1_800_000_062_000,
    )
}

#[test]
fn exports_anchor_only_commitment_from_checkpoint_without_authority_claims() {
    let checkpoint = checkpoint();
    let checkpoint_hash = cid('a');

    let commitment =
        export_anchor_dry_run_commitment(&checkpoint, checkpoint_hash.clone(), &context()).unwrap();

    commitment.validate().unwrap();
    assert_eq!(commitment.chain_id, checkpoint.chain_id);
    assert_eq!(commitment.epoch_id, checkpoint.epoch_id);
    assert_eq!(commitment.checkpoint_height, checkpoint.height);
    assert_eq!(commitment.checkpoint_hash, checkpoint_hash);
    assert_eq!(commitment.new_state_root, checkpoint.new_state_root);
    assert_eq!(commitment.receipt_root, checkpoint.receipt_root);
    assert_eq!(
        commitment.accounting_snapshot_root,
        checkpoint.accounting_snapshot_root
    );
    assert_eq!(
        commitment.reward_manifest_root,
        checkpoint.reward_manifest_root
    );
    assert_eq!(
        commitment.data_availability_root,
        checkpoint.data_availability_root
    );
    assert_eq!(commitment.policy_hash, checkpoint.policy_hash);
    assert_eq!(commitment.validator_set_hash, checkpoint.validator_set_hash);
    assert_eq!(commitment.chain_params_hash, checkpoint.chain_params_hash);

    assert!(commitment.dry_run_only);
    assert!(commitment.timestamp_commitment_only);
    assert!(!commitment.balance_mutation_authorized);
    assert!(!commitment.wallet_ledger_truth_replaced);
    assert!(!commitment.external_settlement_authorized);
    assert!(!commitment.bridge_authorized);
    assert!(!commitment.external_chain_truth);
    assert!(!commitment.finality_claimed);
}

#[test]
fn verifies_anchor_only_commitment_against_checkpoint_input() {
    let checkpoint = checkpoint();
    let checkpoint_hash = cid('a');
    let commitment =
        export_anchor_dry_run_commitment(&checkpoint, checkpoint_hash.clone(), &context()).unwrap();

    let verification =
        verify_anchor_dry_run_commitment(&commitment, &checkpoint, &checkpoint_hash).unwrap();

    verification.validate().unwrap();
    assert_eq!(verification.chain_id, checkpoint.chain_id);
    assert_eq!(verification.epoch_id, checkpoint.epoch_id);
    assert_eq!(verification.anchor_id, commitment.anchor_id);
    assert_eq!(verification.checkpoint_height, checkpoint.height);
    assert_eq!(verification.checkpoint_hash, checkpoint_hash);
    assert_eq!(
        verification.observed_external_reference,
        commitment.external_reference
    );

    assert!(verification.verified_commitment_only);
    assert!(!verification.balance_mutation_detected);
    assert!(!verification.wallet_ledger_truth_replaced);
    assert!(!verification.external_settlement_detected);
    assert!(!verification.bridge_detected);
    assert!(!verification.finality_detected);
}

#[test]
fn verification_rejects_tampered_checkpoint_hash_or_roots() {
    let checkpoint = checkpoint();
    let checkpoint_hash = cid('a');
    let commitment =
        export_anchor_dry_run_commitment(&checkpoint, checkpoint_hash.clone(), &context()).unwrap();

    let err = verify_anchor_dry_run_commitment(&commitment, &checkpoint, &cid('b')).unwrap_err();
    assert_eq!(
        err,
        QuickChainAnchorDryRunError::CommitmentMismatch {
            field: "checkpoint_hash"
        }
    );

    let mut tampered_checkpoint = checkpoint.clone();
    tampered_checkpoint.receipt_root = cid('c');

    let err = verify_anchor_dry_run_commitment(&commitment, &tampered_checkpoint, &checkpoint_hash)
        .unwrap_err();
    assert_eq!(
        err,
        QuickChainAnchorDryRunError::CommitmentMismatch {
            field: "receipt_root"
        }
    );
}

#[test]
fn export_rejects_invalid_context_before_any_artifact_authority_exists() {
    let checkpoint = checkpoint();
    let invalid_context = QuickChainAnchorDryRunExportContext::new(
        "anchor:phase5-r1:bad",
        "dry-run/local-file",
        "dry-run:phase5-r1:artifact-002",
        "file:quickchain-phase5-r1-anchor-002.json",
        0,
    );

    let err =
        export_anchor_dry_run_commitment(&checkpoint, cid('a'), &invalid_context).unwrap_err();
    assert!(matches!(
        err,
        QuickChainAnchorDryRunError::InvalidCommitment { .. }
    ));
}

#[test]
fn anchor_dry_run_source_stays_inert_and_non_mutating() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest.join("src/quickchain/anchor_dry_run.rs");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let code = strip_comments_and_literals(&raw).to_ascii_lowercase();

    for forbidden in [
        "quickchainatomicstate",
        "quickchainbalancestate",
        "quickchainholdstate",
        "quickchainbalancetransition",
        "quickchainsupplydecision",
        "execute_balance_operation",
        "execute_hold_operation",
        "apply_hold_operation",
        "credit(",
        "debit(",
        "capture(",
        "release(",
        "tokio::",
        ".await",
        "std::fs",
        "std::net",
        "reqwest::",
        "axum::",
    ] {
        assert!(
            !code.contains(forbidden),
            "anchor_dry_run.rs must remain inert and non-mutating; found `{forbidden}`"
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
