//! RO:WHAT — QuickChain Phase-0 settlement-boundary preflight tests for svc-storage.
//! RO:WHY — Storage may plan/call wallet capture/release, but must not become ledger, bridge, root, or settlement authority.
//! RO:INTERACTS — policy::settlement, policy::paid_write, /paid/o settlement plan semantics.
//! RO:INVARIANTS — wallet is mutation front-door; integer minor units only; capture <= hold; deterministic idempotency.
//! RO:METRICS — none; settlement outcome metrics are covered by paid-object route tests.
//! RO:CONFIG — no env needed; pure settlement plan and static-source guard.
//! RO:SECURITY — prevents direct ledger/root/checkpoint/validator/bridge/external-settlement creep.
//! RO:TEST — cargo test -p svc-storage --test quickchain_preflight_settlement_boundary.

use std::{fs, path::PathBuf};

use svc_storage::policy::{
    paid_write::{paid_storage_context_idem, PaidWriteProof},
    settlement::PaidStorageSettlementPlan,
};

const CID: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PAYER: &str = "acct_qc_storage_payer";
const ESCROW: &str = "escrow_qc_storage_hold";
const PAYEE: &str = "svc_storage_provider";
const ASSET: &str = "roc";

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    fs::read_to_string(crate_dir().join(relative)).unwrap_or_else(|err| {
        panic!("failed to read {relative}: {err}");
    })
}

fn proof(estimate_minor: u128) -> PaidWriteProof {
    PaidWriteProof {
        txid: "tx_qc_storage_hold_1".to_string(),
        receipt_hash: "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            .to_string(),
        payer: PAYER.to_string(),
        escrow: ESCROW.to_string(),
        asset: ASSET.to_string(),
        estimate_minor,
        idem: Some(
            paid_storage_context_idem(CID, PAYER, ESCROW, ASSET, estimate_minor)
                .expect("context idem should build"),
        ),
    }
}

#[test]
fn settlement_plan_is_integer_bounded_and_deterministic_without_roots_or_finality() {
    let proof = proof(100);

    let plan =
        PaidStorageSettlementPlan::from_paid_write_with_capture_amount(&proof, CID, 84, PAYEE)
            .expect("capture below hold should plan");

    assert_eq!(plan.cid, CID);
    assert_eq!(plan.hold_txid, proof.txid);
    assert_eq!(plan.payer, PAYER);
    assert_eq!(plan.escrow, ESCROW);
    assert_eq!(plan.payee, PAYEE);
    assert_eq!(plan.asset, ASSET);
    assert_eq!(plan.held_amount_minor, 100);
    assert_eq!(plan.capture_amount_minor, 84);
    assert_eq!(plan.release_amount_minor, 16);

    let capture_idem = plan.capture_idem();
    let release_idem = plan.release_idem();
    let failed_release_idem = plan.failed_write_release_idem();

    assert!(capture_idem.starts_with("storage_cap:"));
    assert!(release_idem.starts_with("storage_rel:"));
    assert!(failed_release_idem.starts_with("storage_failrel:"));

    assert_ne!(capture_idem, release_idem);
    assert_ne!(capture_idem, failed_release_idem);
    assert_ne!(release_idem, failed_release_idem);

    let same_plan =
        PaidStorageSettlementPlan::from_paid_write_with_capture_amount(&proof, CID, 84, PAYEE)
            .expect("same inputs should plan");

    assert_eq!(capture_idem, same_plan.capture_idem());
    assert_eq!(release_idem, same_plan.release_idem());
    assert_eq!(failed_release_idem, same_plan.failed_write_release_idem());
}

#[test]
fn settlement_plan_rejects_overcapture_zero_capture_and_escrow_self_payee() {
    let proof = proof(100);

    let overcapture =
        PaidStorageSettlementPlan::from_paid_write_with_capture_amount(&proof, CID, 101, PAYEE)
            .expect_err("capture above hold must reject");

    assert!(
        overcapture
            .reason()
            .contains("below actual paid-storage cost"),
        "overcapture rejection should explain hold/capture mismatch"
    );

    let zero_capture =
        PaidStorageSettlementPlan::from_paid_write_with_capture_amount(&proof, CID, 0, PAYEE)
            .expect_err("zero capture must reject");

    assert!(
        zero_capture.reason().contains("greater than zero"),
        "zero capture rejection should explain integer minor-unit floor"
    );

    let escrow_self_payee =
        PaidStorageSettlementPlan::from_paid_write_with_capture_amount(&proof, CID, 84, ESCROW)
            .expect_err("payee cannot be the escrow account");

    assert!(
        escrow_self_payee
            .reason()
            .contains("payee cannot be the escrow account"),
        "self-payee rejection should preserve escrow boundary"
    );
}

#[test]
fn settlement_source_uses_wallet_front_door_only_and_no_chain_authority() {
    let settlement = read("src/policy/settlement.rs").to_ascii_lowercase();
    let paid_object = read("src/http/routes/paid_object.rs").to_ascii_lowercase();
    let combined = format!("{settlement}\n--- paid_object ---\n{paid_object}");

    for required in [
        "wallet-backed",
        "/v1/capture",
        "/v1/release",
        "integer minor units",
        "capture cannot exceed hold",
        "wallet remains mutation front-door",
        "paid_storage_context_idem",
        "capture_amount_minor",
        "release_amount_minor",
    ] {
        assert!(
            combined.contains(required),
            "settlement source should retain wallet-front-door marker `{required}`"
        );
    }

    for forbidden in [
        "ron_ledger",
        "ledger::",
        "state_root",
        "receipt_root",
        "checkpoint_hash",
        "validator_signature",
        "bridge_settled",
        "external settlement",
        "solana",
        "rox",
        "staking",
        "liquidity",
        "/v1/issue",
        "/v1/transfer",
        "/v1/burn",
        "public bridge",
        "anchor_receipt",
        "anchor_root",
    ] {
        assert!(
            !combined.contains(forbidden),
            "svc-storage settlement path must not smuggle chain authority or external settlement via `{forbidden}`"
        );
    }
}
