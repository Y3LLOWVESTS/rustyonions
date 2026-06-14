//! RO:WHAT — Shared integration-test helpers for svc-wallet invariant tests.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/RES/DX. Keeps invariant tests small and consistent.
//! RO:INTERACTS — svc_wallet::config, dto, ledger client, receipt hashing.
//! RO:INVARIANTS — tests use only wallet public module seams; ledger remains the balance truth.
//! RO:METRICS — none.
//! RO:CONFIG — uses WalletConfig::default with amnesia=true.
//! RO:SECURITY — no bearer tokens or secrets are stored.
//! RO:TEST — imported by tests/i_*.rs.

#![allow(dead_code)]

use ron_ledger::MemoryStorage;
use svc_wallet::{
    config::WalletConfig,
    dto::{
        requests::{AmountMinor, BurnRequest, IssueRequest, TransferRequest},
        responses::{Receipt, ReceiptSettlementStatus, WalletOp},
    },
    ledger::client::LocalLedgerClient,
    util::blake3_receipt::finalize_receipt,
};

pub type TestLedgerClient = LocalLedgerClient<MemoryStorage>;

pub fn cfg() -> WalletConfig {
    WalletConfig::default()
}

pub fn client() -> TestLedgerClient {
    LocalLedgerClient::in_memory().expect("in-memory wallet ledger should initialize")
}

pub fn issue_req(to: &str, amount_minor: u128) -> IssueRequest {
    IssueRequest {
        to: to.to_string(),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(amount_minor),
        idempotency_key: None,
        memo: None,
    }
}

pub fn transfer_req(from: &str, to: &str, amount_minor: u128, nonce: u64) -> TransferRequest {
    TransferRequest {
        from: from.to_string(),
        to: to.to_string(),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(amount_minor),
        nonce,
        idempotency_key: None,
        memo: None,
    }
}

pub fn burn_req(from: &str, amount_minor: u128, nonce: u64) -> BurnRequest {
    BurnRequest {
        from: from.to_string(),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(amount_minor),
        nonce,
        idempotency_key: None,
        memo: None,
    }
}

pub fn balance_of(client: &TestLedgerClient, cfg: &WalletConfig, account: &str) -> u128 {
    client
        .balance(cfg, account)
        .expect("balance read should succeed")
        .amount_minor
        .get()
}

pub fn issue_to(
    client: &TestLedgerClient,
    cfg: &WalletConfig,
    account: &str,
    amount_minor: u128,
    idem: &str,
) -> Receipt {
    client
        .issue(cfg, &issue_req(account, amount_minor), idem)
        .expect("issue should succeed")
}

pub fn dummy_receipt(txid: &str, idem: &str) -> Receipt {
    finalize_receipt(Receipt {
        txid: txid.to_string(),
        op: WalletOp::Transfer,
        from: Some("acct_a".to_string()),
        to: Some("acct_b".to_string()),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(1),
        nonce: Some(1),
        idem: idem.to_string(),
        ts: 1_777_309_851_000,
        ledger_seq_start: Some(1),
        ledger_seq_end: Some(2),
        ledger_root: "00".repeat(32),
        settlement_status: ReceiptSettlementStatus::Accepted,
        receipt_hash: String::new(),
    })
    .expect("dummy receipt should hash")
}
