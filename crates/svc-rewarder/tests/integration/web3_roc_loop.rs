//! RO:WHAT — WEB3 ROC loop smoke proving rewarder-issued wallet requests mutate balances through svc-wallet.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/RES/DX. The first closed-loop proof must keep wallet as mutation front-door.
//! RO:INTERACTS — ron_accounting interop vector, svc_rewarder compute/output modules, svc_wallet HTTP router.
//! RO:INVARIANTS — rewarder does not mutate ledger; wallet issue is idempotent; replay does not double issue.
//! RO:METRICS — indirectly proves wallet issue/replay metrics through the HTTP route path.
//! RO:CONFIG — uses svc-wallet dev/amnesia state and svc-rewarder dev policy defaults.
//! RO:SECURITY — dev-only Authorization: Bearer dev; no real macaroons or secrets.
//! RO:TEST — cargo test -p svc-rewarder --test integration web3_roc_loop.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use ron_accounting::reward_snapshot_interop_vector_v1;
use serde_json::Value;
use svc_rewarder::core::{compute_manifest, ComputeInput};
use svc_rewarder::inputs::{canonical_snapshot_cid, AccountingSnapshot, ContentCid, RewardPolicy};
use svc_rewarder::outputs::{IntentResult, SettlementBatch};
use svc_wallet::routes as wallet_routes;
use tower::ServiceExt;

const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const IDEMPOTENCY_SALT: &str = "svc-rewarder|v1";

fn load_rewarder_snapshot_from_accounting_vector() -> (String, AccountingSnapshot) {
    let vector = reward_snapshot_interop_vector_v1().expect("ron-accounting interop vector");
    vector.validate().expect("accounting vector validates");

    let snapshot = serde_json::from_str::<AccountingSnapshot>(&vector.canonical_snapshot_json)
        .expect("rewarder should parse accounting vector snapshot");

    let rewarder_cid = canonical_snapshot_cid(snapshot.clone()).expect("canonical snapshot cid");
    assert_eq!(rewarder_cid, vector.snapshot_cid);

    (vector.epoch_id, snapshot)
}

fn compute_settlement_batch(epoch_id: String, snapshot: AccountingSnapshot) -> SettlementBatch {
    let inputs_cid = canonical_snapshot_cid(snapshot.clone()).expect("snapshot cid");
    let manifest = compute_manifest(
        ComputeInput {
            epoch_id,
            inputs_cid: ContentCid::parse(&inputs_cid).expect("snapshot cid parses"),
            policy: RewardPolicy::dev_default("policy:v1", POLICY_HASH),
            snapshot,
            dry_run: false,
            idempotency_salt: IDEMPOTENCY_SALT.to_string(),
        },
        IntentResult::Accepted,
    )
    .expect("reward manifest computes");

    assert_eq!(manifest.totals.payout_minor_units.get(), 999);
    assert_eq!(manifest.totals.residual_minor_units.get(), 1);

    SettlementBatch::from_manifest(&manifest).expect("settlement batch plans")
}

async fn post_wallet_issue(app: axum::Router, body: Value, idem: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri("/v1/issue")
        .header("authorization", "Bearer dev")
        .header("idempotency-key", idem)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("issue request builds");

    let res = app.oneshot(req).await.expect("wallet issue route responds");
    let status = res.status();
    let bytes = to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("wallet issue body bytes");
    let value = serde_json::from_slice::<Value>(&bytes).expect("wallet issue body json");

    (status, value)
}

async fn get_wallet_balance(app: axum::Router, account: &str) -> Value {
    let uri = format!("/v1/balance?account={account}&asset=roc");
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .header("authorization", "Bearer dev")
        .body(Body::empty())
        .expect("balance request builds");

    let res = app
        .oneshot(req)
        .await
        .expect("wallet balance route responds");
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("wallet balance body bytes");
    serde_json::from_slice::<Value>(&bytes).expect("wallet balance body json")
}

async fn get_wallet_metrics(app: axum::Router) -> String {
    let req = Request::builder()
        .method("GET")
        .uri("/metrics")
        .body(Body::empty())
        .expect("metrics request builds");

    let res = app
        .oneshot(req)
        .await
        .expect("wallet metrics route responds");
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("wallet metrics body bytes");

    String::from_utf8(bytes.to_vec()).expect("wallet metrics text")
}

#[tokio::test]
async fn rewarder_settlement_issues_through_wallet_once_without_double_issue() {
    let (epoch_id, snapshot) = load_rewarder_snapshot_from_accounting_vector();
    let settlement = compute_settlement_batch(epoch_id, snapshot);
    let wallet_batch = settlement.to_wallet_issue_batch();

    assert_eq!(wallet_batch.wallet_path, "/v1/issue");
    assert_eq!(wallet_batch.total_minor_units, "999");
    assert_eq!(wallet_batch.requests.len(), 2);

    let wallet_state = wallet_routes::WalletState::dev().expect("wallet dev state");
    let wallet = wallet_routes::router(wallet_state);

    let mut first_receipts = Vec::new();

    for request in &wallet_batch.requests {
        let body = serde_json::to_value(request).expect("wallet issue request serializes");
        let idem = request
            .idempotency_key
            .as_deref()
            .expect("rewarder request includes idempotency key");

        let (status, receipt) = post_wallet_issue(wallet.clone(), body, idem).await;
        assert_eq!(status, StatusCode::OK);

        assert_eq!(receipt["op"], "issue");
        assert_eq!(receipt["to"], request.to);
        assert_eq!(receipt["asset"], "roc");
        assert_eq!(receipt["amount_minor"], request.amount_minor);
        assert_eq!(receipt["idem"], idem);
        assert!(receipt["txid"].as_str().unwrap().starts_with("tx_"));
        assert!(receipt["receipt_hash"].as_str().unwrap().starts_with("b3:"));

        first_receipts.push(receipt);
    }

    let acct_a_balance = get_wallet_balance(wallet.clone(), "acct_a").await;
    let acct_b_balance = get_wallet_balance(wallet.clone(), "acct_b").await;

    assert_eq!(acct_a_balance["account"], "acct_a");
    assert_eq!(acct_a_balance["asset"], "roc");
    assert_eq!(acct_a_balance["amount_minor"], "356");

    assert_eq!(acct_b_balance["account"], "acct_b");
    assert_eq!(acct_b_balance["asset"], "roc");
    assert_eq!(acct_b_balance["amount_minor"], "643");

    for (idx, request) in wallet_batch.requests.iter().enumerate() {
        let body = serde_json::to_value(request).expect("wallet issue request serializes");
        let idem = request
            .idempotency_key
            .as_deref()
            .expect("rewarder request includes idempotency key");

        let (status, replay_receipt) = post_wallet_issue(wallet.clone(), body, idem).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(replay_receipt, first_receipts[idx]);
    }

    let acct_a_after_replay = get_wallet_balance(wallet.clone(), "acct_a").await;
    let acct_b_after_replay = get_wallet_balance(wallet.clone(), "acct_b").await;

    assert_eq!(acct_a_after_replay["amount_minor"], "356");
    assert_eq!(acct_b_after_replay["amount_minor"], "643");

    let metrics = get_wallet_metrics(wallet.clone()).await;
    assert!(metrics.contains("wallet_ops_total{op=\"issue\"} 2"));
    assert!(metrics.contains("wallet_idempotency_replays_total 2"));
}
