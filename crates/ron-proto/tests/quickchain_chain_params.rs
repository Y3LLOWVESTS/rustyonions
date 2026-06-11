//! RO:WHAT — Tests disabled and future-validator QuickChain chain-parameter bounds.
//! RO:WHY — ECON/GOV: Phase 0 needs an explicit no-committee posture without ambiguous bounds.
//! RO:INTERACTS — QuickChainChainParamsV1 and its inert validation helpers.
//! RO:INVARIANTS — zero/zero means no committee; mixed bounds reject; byte caps stay positive.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — validation grants no runtime, validator, ledger, or settlement authority.
//! RO:TEST — this integration test is the chain-parameter DTO audit gate.

use ron_proto::{
    QuickChainCanonicalEncodingV1, QuickChainChainParamsV1, QuickChainReceiptRootSchemeV1,
    QuickChainStateRootSchemeV1, QuickChainValidationError, QUICKCHAIN_CHAIN_PARAMS_SCHEMA,
    QUICKCHAIN_DTO_VERSION,
};

fn chain_params(min_validators: u16, max_validators: u16) -> QuickChainChainParamsV1 {
    QuickChainChainParamsV1 {
        schema: QUICKCHAIN_CHAIN_PARAMS_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        enabled: false,
        epoch_duration_ms: 300_000,
        checkpoint_cadence: 1,
        challenge_window_ms: 86_400_000,
        max_receipts_per_batch: 100_000,
        max_accounting_snapshot_bytes: 16 * 1024 * 1024,
        max_reward_manifest_bytes: 16 * 1024 * 1024,
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        state_root_scheme: QuickChainStateRootSchemeV1::SortedMerkleMapV1,
        receipt_root_scheme: QuickChainReceiptRootSchemeV1::LedgerSequenceMerkleV1,
        quorum_bps: 6700,
        min_validators,
        max_validators,
        passport_required: true,
        bond_required: false,
        rox_anchor_enabled: false,
    }
}

fn assert_invalid_field(params: QuickChainChainParamsV1, expected_field: &'static str) {
    let error = params.validate().unwrap_err();

    match error {
        QuickChainValidationError::InvalidField { field, .. } => {
            assert_eq!(field, expected_field);
        }
        other => panic!("expected InvalidField for {expected_field}, got {other:?}"),
    }
}

#[test]
fn disabled_chain_params_allow_explicit_no_committee_bounds() {
    let params = chain_params(0, 0);

    params.validate().unwrap();
    params.validate_phase0_disabled().unwrap();
}

#[test]
fn ordered_nonzero_validator_bounds_remain_valid_descriptive_data() {
    let params = chain_params(4, 21);

    params.validate().unwrap();
}

#[test]
fn validator_bounds_reject_half_configured_ranges() {
    assert_invalid_field(chain_params(0, 21), "validator_bounds");
    assert_invalid_field(chain_params(4, 0), "validator_bounds");
}

#[test]
fn validator_bounds_reject_reversed_nonzero_ranges() {
    assert_invalid_field(chain_params(5, 4), "max_validators");
}

#[test]
fn chain_params_reject_zero_bounded_artifact_limits() {
    let mut params = chain_params(0, 0);
    params.max_receipts_per_batch = 0;
    assert_invalid_field(params, "max_receipts_per_batch");

    let mut params = chain_params(0, 0);
    params.max_accounting_snapshot_bytes = 0;
    assert_invalid_field(params, "max_accounting_snapshot_bytes");

    let mut params = chain_params(0, 0);
    params.max_reward_manifest_bytes = 0;
    assert_invalid_field(params, "max_reward_manifest_bytes");
}
