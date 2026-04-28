use std::sync::Arc;

use svc_rewarder::core::{compute_manifest, AmountMinor, ComputeInput};
use svc_rewarder::inputs::{AccountContribution, AccountingSnapshot, ContentCid, RewardPolicy};
use svc_rewarder::outputs::{
    DevWalletIssueClient, IntentResult, IntentStore, SettlementBatch, WalletIssueClient,
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
                account: "acct_a".into(),
                bytes_stored: 100,
                bytes_served: 0,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: "acct_b".into(),
                bytes_stored: 200,
                bytes_served: 0,
                uptime_seconds: 0,
            },
        ],
    }
}

fn settlement_batch() -> SettlementBatch {
    let input = ComputeInput {
        epoch_id: "epoch-wallet-1".into(),
        inputs_cid: cid(),
        policy: policy(),
        snapshot: snapshot(),
        dry_run: false,
        idempotency_salt: "test".into(),
    };
    let manifest = compute_manifest(input, IntentResult::Accepted).unwrap();
    SettlementBatch::from_manifest(&manifest).unwrap()
}

#[test]
fn dev_wallet_client_previews_issue_batch_without_emitting() {
    let store = Arc::new(IntentStore::default());
    let client = DevWalletIssueClient::new(store, "/v1/issue");
    let batch = settlement_batch();

    let preview = client.preview_issue_batch(&batch).unwrap();

    assert_eq!(preview.wallet_path, "/v1/issue");
    assert_eq!(preview.run_key, batch.run_key);
    assert_eq!(preview.requests.len(), batch.intents.len());

    for req in preview.requests {
        assert!(req.idempotency_key.unwrap().starts_with("b3:"));
        assert!(req.amount_minor.parse::<u128>().unwrap() > 0);
    }
}

#[test]
fn dev_wallet_client_emit_is_idempotent() {
    let store = Arc::new(IntentStore::default());
    let client = DevWalletIssueClient::new(store, "/v1/issue");
    let batch = settlement_batch();

    let first = client.emit_issue_batch(&batch, false).unwrap();
    let second = client.emit_issue_batch(&batch, false).unwrap();

    assert_eq!(first.result.as_str(), "accepted");
    assert_eq!(second.result.as_str(), "dup");
}

#[test]
fn dev_wallet_client_dry_run_does_not_consume_run_key() {
    let store = Arc::new(IntentStore::default());
    let client = DevWalletIssueClient::new(store, "/v1/issue");
    let batch = settlement_batch();

    let dry = client.emit_issue_batch(&batch, true).unwrap();
    let live = client.emit_issue_batch(&batch, false).unwrap();

    assert_eq!(dry.result.as_str(), "dry_run");
    assert_eq!(live.result.as_str(), "accepted");
}
