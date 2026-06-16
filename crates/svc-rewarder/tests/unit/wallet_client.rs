use std::sync::{Arc, Mutex};
use std::time::Duration;

use svc_rewarder::core::{compute_manifest, AmountMinor, ComputeInput};
use svc_rewarder::inputs::{
    AccountContribution, AccountingSnapshot, ContentCid, RewardFundingSource, RewardPolicy,
};
use svc_rewarder::outputs::{
    DevWalletIssueClient, HttpWalletIssueClient, IntentResult, IntentStore, SettlementBatch,
    WalletIssueClient,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn cid() -> ContentCid {
    ContentCid::parse(format!("b3:{}", "a".repeat(64))).unwrap()
}

fn policy() -> RewardPolicy {
    RewardPolicy {
        id: "policy:v1".into(),
        hash: format!("b3:{}", "b".repeat(64)),
        signed: true,
        funding_source: RewardFundingSource::ProtocolPool,
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

#[test]
fn http_wallet_client_rejects_https_until_tls_adapter_exists() {
    let err = HttpWalletIssueClient::try_new(
        "https://127.0.0.1:8088",
        "/v1/issue",
        "dev",
        Duration::from_secs(5),
    )
    .unwrap_err();

    assert_eq!(err.reason(), "config");
}

#[tokio::test]
async fn http_wallet_client_posts_issue_requests_to_wallet_route() {
    let batch = settlement_batch();
    let expected_posts = batch.intents.len();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen_server = Arc::clone(&seen);

    let server = tokio::spawn(async move {
        for idx in 0..expected_posts {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_http_request(&mut stream).await;
            seen_server.lock().unwrap().push(request);

            let receipt = serde_json::json!({
                "txid": format!("tx_mock_{idx}"),
                "receipt_hash": format!("b3:{}", "d".repeat(64)),
                "op": "issue",
                "asset": "roc",
                "amount_minor": "1",
                "idem": format!("idem_mock_{idx}")
            });
            let body = receipt.to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        }
    });

    let client = HttpWalletIssueClient::try_new(
        format!("http://{addr}"),
        "/v1/issue",
        "dev",
        Duration::from_secs(5),
    )
    .unwrap();

    let outcome = client.emit_issue_batch(&batch, false).await.unwrap();

    server.await.unwrap();

    assert_eq!(outcome.result.as_str(), "accepted");
    assert_eq!(outcome.batch.requests.len(), expected_posts);
    assert_eq!(outcome.receipts.len(), expected_posts);

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), expected_posts);
    for request in seen.iter() {
        assert!(request.starts_with("POST /v1/issue HTTP/1.1"));
        assert!(request.contains("Authorization: Bearer dev"));
        assert!(request.contains("Idempotency-Key: b3:"));
        assert!(request.contains("\"asset\":\"roc\""));
        assert!(request.contains("\"amount_minor\":\""));
        assert!(request.contains("\"idempotency_key\":\"b3:"));
    }
}

#[tokio::test]
async fn http_wallet_client_dry_run_posts_nothing() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let client = HttpWalletIssueClient::try_new(
        format!("http://{addr}"),
        "/v1/issue",
        "dev",
        Duration::from_secs(5),
    )
    .unwrap();

    let outcome = client
        .emit_issue_batch(&settlement_batch(), true)
        .await
        .unwrap();

    assert_eq!(outcome.result.as_str(), "dry_run");
    assert!(outcome.receipts.is_empty());

    drop(listener);
}

async fn read_http_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0_u8; 1024];

    loop {
        let n = stream.read(&mut tmp).await.unwrap();
        assert!(n > 0, "client closed before headers completed");
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    let header_end = buf
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap();

    let headers = String::from_utf8(buf[..header_end].to_vec()).unwrap();
    let content_len = headers
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .and_then(|value| value.trim().parse::<usize>().ok())
        })
        .unwrap_or(0);

    let already_have = buf.len().saturating_sub(header_end + 4);
    let mut remaining = content_len.saturating_sub(already_have);
    while remaining > 0 {
        let n = stream.read(&mut tmp).await.unwrap();
        assert!(n > 0, "client closed before body completed");
        buf.extend_from_slice(&tmp[..n]);
        remaining = remaining.saturating_sub(n);
    }

    String::from_utf8(buf).unwrap()
}
