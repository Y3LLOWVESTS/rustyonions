use svc_rewarder::core::{compute_manifest, AmountMinor, ComputeInput};
use svc_rewarder::inputs::{AccountContribution, AccountingSnapshot, ContentCid, RewardPolicy};
use svc_rewarder::outputs::{
    IntentResult, IntentStore, SettlementBatch, WalletIssueBatch, ROC_ASSET, WALLET_ISSUE_PATH,
};

fn cid() -> ContentCid {
    ContentCid::parse(format!("b3:{}", "a".repeat(64))).unwrap()
}

fn policy() -> RewardPolicy {
    RewardPolicy {
        id: "policy:v1".into(),
        hash: format!("b3:{}", "b".repeat(64)),
        signed: true,
        max_payout_minor_units: AmountMinor(1_000),
        min_payout_minor_units: AmountMinor(1),
        weight_bps: 10_000,
        rounding: "floor".into(),
    }
}

fn snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: "acct_b".into(),
                bytes_stored: 200,
                bytes_served: 0,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: "acct_a".into(),
                bytes_stored: 100,
                bytes_served: 0,
                uptime_seconds: 0,
            },
        ],
    }
}

fn input() -> ComputeInput {
    ComputeInput {
        epoch_id: "epoch-1".into(),
        inputs_cid: cid(),
        policy: policy(),
        snapshot: snapshot(),
        dry_run: false,
        idempotency_salt: "test".into(),
    }
}

#[test]
fn settlement_batch_matches_manifest_payout_total() {
    let manifest = compute_manifest(input(), IntentResult::Accepted).unwrap();
    let batch = SettlementBatch::from_manifest(&manifest).unwrap();

    assert_eq!(batch.run_key, manifest.run_key);
    assert_eq!(batch.epoch_id, "epoch-1");
    assert_eq!(batch.manifest_commitment, manifest.commitment);
    assert_eq!(batch.total_minor_units, manifest.totals.payout_minor_units);
    assert_eq!(batch.intents.len(), manifest.payouts.len());

    for intent in &batch.intents {
        assert_eq!(intent.asset, ROC_ASSET);
        assert_eq!(intent.run_key, manifest.run_key);
        assert_eq!(intent.epoch_id, manifest.epoch_id);
        assert_eq!(intent.manifest_commitment, manifest.commitment);
        assert!(intent.idempotency_key.starts_with("b3:"));
        assert!(
            intent.idempotency_key.len() <= 64,
            "wallet Idempotency-Key must be <=64 bytes"
        );
    }
}

#[test]
fn settlement_intents_are_sorted_by_recipient() {
    let manifest = compute_manifest(input(), IntentResult::Accepted).unwrap();
    let batch = SettlementBatch::from_manifest(&manifest).unwrap();

    let accounts = batch
        .intents
        .iter()
        .map(|intent| intent.to.as_str())
        .collect::<Vec<_>>();

    assert_eq!(accounts, vec!["acct_a", "acct_b"]);
}

#[test]
fn emit_batch_once_is_idempotent_by_run_key() {
    let manifest = compute_manifest(input(), IntentResult::Accepted).unwrap();
    let batch = SettlementBatch::from_manifest(&manifest).unwrap();
    let store = IntentStore::default();

    assert_eq!(store.emit_batch_once(&batch, false).as_str(), "accepted");
    assert_eq!(store.emit_batch_once(&batch, false).as_str(), "dup");
    assert_eq!(store.emit_batch_once(&batch, true).as_str(), "dry_run");
}

#[test]
fn wallet_issue_batch_matches_wallet_issue_shape() {
    let manifest = compute_manifest(input(), IntentResult::Accepted).unwrap();
    let settlement = SettlementBatch::from_manifest(&manifest).unwrap();
    let wallet_batch: WalletIssueBatch = settlement.to_wallet_issue_batch();

    assert_eq!(wallet_batch.run_key, manifest.run_key);
    assert_eq!(wallet_batch.epoch_id, manifest.epoch_id);
    assert_eq!(wallet_batch.manifest_commitment, manifest.commitment);
    assert_eq!(wallet_batch.wallet_path, WALLET_ISSUE_PATH);
    assert_eq!(
        wallet_batch.total_minor_units,
        manifest.totals.payout_minor_units.get().to_string()
    );
    assert_eq!(wallet_batch.requests.len(), manifest.payouts.len());

    for req in &wallet_batch.requests {
        assert_eq!(req.asset, ROC_ASSET);
        assert!(req.amount_minor.parse::<u128>().unwrap() > 0);
        assert!(req.memo.as_ref().unwrap().starts_with("svc-rewarder:"));
        assert!(req.idempotency_key.as_ref().unwrap().len() <= 64);
    }
}

#[test]
fn wallet_issue_request_serializes_amount_as_string() {
    let manifest = compute_manifest(input(), IntentResult::Accepted).unwrap();
    let settlement = SettlementBatch::from_manifest(&manifest).unwrap();
    let wallet_batch = settlement.to_wallet_issue_batch();

    let encoded = serde_json::to_string(&wallet_batch.requests[0]).unwrap();

    assert!(encoded.contains(r#""amount_minor":""#));
    assert!(!encoded.contains(r#""amount_minor":0"#));
    assert!(encoded.contains(r#""idempotency_key":"#));
}
