#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Tests ron-ledger Phase 5 Round 3 chosen external posture boundary helpers.
//! RO:WHY — ECON/GOV: ledger may observe anchor-only posture status but must not let it become mutation, receipt, unlock, bridge, or settlement authority.
//! RO:INTERACTS — ron_ledger::quickchain::external_posture and ron-proto posture DTOs.
//! RO:INVARIANTS — read-only evidence; no wallet mutation; no balance mutation; no paid unlock; no external truth.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — no secrets, IO, services, external calls, or spend authority.
//! RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase5_external_posture.

use std::{fs, path::Path};

use ron_ledger::quickchain::{
    verify_external_posture_decision_read_only, QuickChainExternalPostureBoundaryError,
};
use ron_proto::{
    ContentId, QuickChainCanonicalEncodingV1, QuickChainExternalIntegrationPostureV1,
    QuickChainExternalPostureDecisionV1, QuickChainExternalPostureSemanticsV1,
    QUICKCHAIN_DTO_VERSION, QUICKCHAIN_EXTERNAL_POSTURE_DECISION_SCHEMA,
};

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().unwrap()
}

fn sample_decision() -> QuickChainExternalPostureDecisionV1 {
    QuickChainExternalPostureDecisionV1 {
        schema: QUICKCHAIN_EXTERNAL_POSTURE_DECISION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        epoch_id: "epoch:phase5-r3".to_string(),
        posture_id: "external-posture:phase5-r3:001".to_string(),
        decision_ref: "governance:phase5-r3:anchor-only".to_string(),
        anchor_commitment_hash: cid('a'),
        evidence_ref: "anchor-evidence:phase5-r3:001".to_string(),
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        chosen_posture: QuickChainExternalIntegrationPostureV1::AnchorOnly,
        posture_semantics: QuickChainExternalPostureSemanticsV1::EvidenceAndAnchoringOnly,
        produced_at_ms: 1_800_000_300_000,
        anchor_only_selected: true,
        evidence_and_anchoring_only: true,
        wallet_ledger_truth_canonical: true,
        external_da_selected: false,
        external_l2_selected: false,
        hybrid_selected: false,
        balance_mutation_authorized: false,
        receipt_authority_authorized: false,
        paid_unlock_authorized: false,
        external_settlement_authorized: false,
        bridge_authorized: false,
        rox_solana_runtime_authorized: false,
        exchange_facing_authorized: false,
        liquidity_authorized: false,
        staking_authorized: false,
    }
}

#[test]
fn verifies_anchor_only_external_posture_as_read_only_evidence() {
    let decision = sample_decision();
    let verification = verify_external_posture_decision_read_only(&decision).unwrap();

    verification.validate().unwrap();
    assert_eq!(verification.chain_id, decision.chain_id);
    assert_eq!(verification.epoch_id, decision.epoch_id);
    assert_eq!(verification.posture_id, decision.posture_id);
    assert_eq!(
        verification.chosen_posture,
        QuickChainExternalIntegrationPostureV1::AnchorOnly
    );
    assert_eq!(
        verification.anchor_commitment_hash,
        decision.anchor_commitment_hash
    );
    assert_eq!(verification.observed_evidence_ref, decision.evidence_ref);

    assert!(verification.single_path_selected);
    assert!(verification.verified_evidence_only);
    assert!(verification.wallet_ledger_truth_canonical);
    assert!(!verification.balance_mutation_detected);
    assert!(!verification.receipt_authority_detected);
    assert!(!verification.paid_unlock_detected);
    assert!(!verification.external_settlement_detected);
    assert!(!verification.bridge_detected);
    assert!(!verification.rox_solana_runtime_detected);
    assert!(!verification.exchange_facing_detected);
    assert!(!verification.liquidity_detected);
    assert!(!verification.staking_detected);
}

#[test]
fn rejects_deferred_external_postures_before_any_verification_artifact_exists() {
    let mut decision = sample_decision();
    decision.chosen_posture =
        QuickChainExternalIntegrationPostureV1::ExternalDataAvailabilityDeferred;

    let err = verify_external_posture_decision_read_only(&decision).unwrap_err();
    assert!(matches!(
        err,
        QuickChainExternalPostureBoundaryError::InvalidDecision { .. }
    ));

    let mut decision = sample_decision();
    decision.hybrid_selected = true;

    let err = verify_external_posture_decision_read_only(&decision).unwrap_err();
    assert!(matches!(
        err,
        QuickChainExternalPostureBoundaryError::InvalidDecision { .. }
    ));
}

#[test]
fn rejects_external_posture_authority_creep_before_ledger_boundary() {
    let mut decision = sample_decision();
    decision.balance_mutation_authorized = true;
    assert!(verify_external_posture_decision_read_only(&decision).is_err());

    let mut decision = sample_decision();
    decision.receipt_authority_authorized = true;
    assert!(verify_external_posture_decision_read_only(&decision).is_err());

    let mut decision = sample_decision();
    decision.paid_unlock_authorized = true;
    assert!(verify_external_posture_decision_read_only(&decision).is_err());

    let mut decision = sample_decision();
    decision.external_settlement_authorized = true;
    assert!(verify_external_posture_decision_read_only(&decision).is_err());

    let mut decision = sample_decision();
    decision.bridge_authorized = true;
    assert!(verify_external_posture_decision_read_only(&decision).is_err());

    let mut decision = sample_decision();
    decision.rox_solana_runtime_authorized = true;
    assert!(verify_external_posture_decision_read_only(&decision).is_err());

    let mut decision = sample_decision();
    decision.exchange_facing_authorized = true;
    assert!(verify_external_posture_decision_read_only(&decision).is_err());

    let mut decision = sample_decision();
    decision.liquidity_authorized = true;
    assert!(verify_external_posture_decision_read_only(&decision).is_err());

    let mut decision = sample_decision();
    decision.staking_authorized = true;
    assert!(verify_external_posture_decision_read_only(&decision).is_err());
}

#[test]
fn external_posture_source_stays_inert_and_non_mutating() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest.join("src/quickchain/external_posture.rs");
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
        "std::fs",
        "std::net",
        "tokio::",
        ".await",
        "reqwest::",
        "axum::",
    ] {
        assert!(
            !code.contains(forbidden),
            "external_posture.rs must remain inert and non-mutating; found `{forbidden}`"
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
