//! RO:WHAT — Local ron-ledger adapter for issue, transfer, burn, hold, capture, release, and balance reads.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. Ensures svc-wallet never becomes its own durable truth store.
//! RO:INTERACTS — ron_ledger::{Ledger, IngestRequest, Entry}, dto requests/responses, util::blake3_receipt.
//! RO:INVARIANTS — transfers are balanced; issue/burn are explicit supply exceptions; escrow moves through ledger.
//! RO:METRICS — caller records commit latency and rejects; this adapter emits no metrics directly.
//! RO:CONFIG — WalletConfig amount ceilings and asset validation happen before commit.
//! RO:SECURITY — stores KID/capability refs as identifiers only; token verification must occur before this adapter is called.
//! RO:TEST — issue_and_transfer_flow_updates_balances; i_13_hold_capture_release.

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use ron_ledger::{
    api::IngestRequest,
    config::LedgerConfig,
    engine::{Ledger, MemoryStorage, Storage},
    types::{AccountId, CapabilityRef, Entry, EntryKind, Kid, Nonce},
};

use crate::{
    config::WalletConfig,
    dto::{
        requests::{AmountMinor, BurnRequest, IssueRequest, TransferRequest},
        responses::{BalanceResponse, Receipt, ReceiptSettlementStatus, WalletOp},
    },
    errors::{WalletError, WalletResult},
    ledger::types::LedgerIdentity,
    util::blake3_receipt::{finalize_receipt, ledger_nonce_b64, txid_for},
};

/// Local in-process ledger adapter.
pub struct LocalLedgerClient<S: Storage> {
    ledger: Arc<Ledger<S>>,
    identity: LedgerIdentity,
}

impl LocalLedgerClient<MemoryStorage> {
    /// Build an in-memory adapter for tests, amnesia demos, and local dev.
    pub fn in_memory() -> WalletResult<Self> {
        let ledger = Ledger::new(MemoryStorage::default(), LedgerConfig::default())?;
        Ok(Self::new(ledger, LedgerIdentity::default()))
    }
}

impl<S: Storage> LocalLedgerClient<S> {
    /// Build an adapter from a concrete ledger.
    pub fn new(ledger: Ledger<S>, identity: LedgerIdentity) -> Self {
        Self {
            ledger: Arc::new(ledger),
            identity,
        }
    }

    /// Read balance from ron-ledger.
    pub fn balance(&self, cfg: &WalletConfig, account: &str) -> WalletResult<BalanceResponse> {
        let account_id = AccountId::new(account)?;
        let amount = self.ledger.balance(&account_id)?;
        Ok(BalanceResponse {
            account: account.to_string(),
            asset: cfg.asset.clone(),
            amount_minor: AmountMinor(amount),
            as_of_height: None,
            stale_ms: 0,
        })
    }

    /// Commit an issue/mint transaction.
    pub fn issue(
        &self,
        cfg: &WalletConfig,
        req: &IssueRequest,
        idem: &str,
    ) -> WalletResult<Receipt> {
        req.validate(cfg)?;
        let amount = req.amount_minor.try_as_u64_for_ledger()?;
        let ts = now_millis();
        let txid = txid_for(WalletOp::Issue, idem, req)?;

        let entry = self.entry(
            format!("{txid}:mint"),
            ts,
            EntryKind::Mint,
            &req.to,
            amount,
            ledger_nonce_b64(&["issue", idem, &req.to]),
        )?;

        let resp = self.ledger.ingest(IngestRequest {
            batch: vec![entry],
            idem_id: Some(idem.to_string()),
        })?;

        if !resp.accepted {
            return Err(WalletError::nonce_conflict("ledger rejected issue batch"));
        }

        finalize_receipt(Receipt {
            txid,
            op: WalletOp::Issue,
            from: None,
            to: Some(req.to.clone()),
            asset: req.asset.clone(),
            amount_minor: req.amount_minor,
            nonce: None,
            idem: idem.to_string(),
            ts,
            ledger_seq_start: resp.seq_start.map(|seq| seq.get()),
            ledger_seq_end: resp.seq_end.map(|seq| seq.get()),
            ledger_root: resp.new_root.to_hex(),
            settlement_status: ReceiptSettlementStatus::Accepted,
            receipt_hash: String::new(),
        })
    }

    /// Commit a balanced transfer transaction.
    pub fn transfer(
        &self,
        cfg: &WalletConfig,
        req: &TransferRequest,
        idem: &str,
    ) -> WalletResult<Receipt> {
        self.commit_balanced_move(
            cfg,
            req,
            idem,
            WalletOp::Transfer,
            EntryKind::Debit,
            EntryKind::Credit,
        )
    }

    /// Reserve funds into an escrow account.
    ///
    /// This is the ledger-level primitive behind the future `POST /v1/hold`.
    /// It debits the payer and credits an escrow account atomically.
    pub fn hold(
        &self,
        cfg: &WalletConfig,
        req: &TransferRequest,
        idem: &str,
    ) -> WalletResult<Receipt> {
        self.commit_balanced_move(
            cfg,
            req,
            idem,
            WalletOp::Hold,
            EntryKind::Hold,
            EntryKind::Credit,
        )
    }

    /// Capture funds from escrow into a payee account.
    ///
    /// This is the ledger-level primitive behind the future `POST /v1/capture`.
    pub fn capture(
        &self,
        cfg: &WalletConfig,
        req: &TransferRequest,
        idem: &str,
    ) -> WalletResult<Receipt> {
        self.commit_balanced_move(
            cfg,
            req,
            idem,
            WalletOp::Capture,
            EntryKind::Debit,
            EntryKind::Credit,
        )
    }

    /// Release remaining escrow funds back to the payer.
    ///
    /// This is the ledger-level primitive behind the future `POST /v1/release`.
    pub fn release(
        &self,
        cfg: &WalletConfig,
        req: &TransferRequest,
        idem: &str,
    ) -> WalletResult<Receipt> {
        self.commit_balanced_move(
            cfg,
            req,
            idem,
            WalletOp::Release,
            EntryKind::Debit,
            EntryKind::Credit,
        )
    }

    /// Commit a burn transaction.
    pub fn burn(&self, cfg: &WalletConfig, req: &BurnRequest, idem: &str) -> WalletResult<Receipt> {
        req.validate(cfg)?;
        let amount = req.amount_minor.try_as_u64_for_ledger()?;
        let ts = now_millis();
        let txid = txid_for(WalletOp::Burn, idem, req)?;

        let entry = self.entry(
            format!("{txid}:burn"),
            ts,
            EntryKind::Burn,
            &req.from,
            amount,
            ledger_nonce_b64(&["burn", idem, &req.from, &req.nonce.to_string()]),
        )?;

        let resp = self.ledger.ingest(IngestRequest {
            batch: vec![entry],
            idem_id: Some(idem.to_string()),
        })?;

        if !resp.accepted {
            return Err(WalletError::nonce_conflict("ledger rejected burn batch"));
        }

        finalize_receipt(Receipt {
            txid,
            op: WalletOp::Burn,
            from: Some(req.from.clone()),
            to: None,
            asset: req.asset.clone(),
            amount_minor: req.amount_minor,
            nonce: Some(req.nonce),
            idem: idem.to_string(),
            ts,
            ledger_seq_start: resp.seq_start.map(|seq| seq.get()),
            ledger_seq_end: resp.seq_end.map(|seq| seq.get()),
            ledger_root: resp.new_root.to_hex(),
            settlement_status: ReceiptSettlementStatus::Accepted,
            receipt_hash: String::new(),
        })
    }

    fn commit_balanced_move(
        &self,
        cfg: &WalletConfig,
        req: &TransferRequest,
        idem: &str,
        op: WalletOp,
        debit_kind: EntryKind,
        credit_kind: EntryKind,
    ) -> WalletResult<Receipt> {
        req.validate(cfg)?;
        let amount = req.amount_minor.try_as_u64_for_ledger()?;
        let ts = now_millis();
        let txid = txid_for(op, idem, req)?;
        let op_label = op.as_str();
        let nonce_string = req.nonce.to_string();

        let debit = self.entry(
            format!("{txid}:{op_label}:debit"),
            ts,
            debit_kind,
            &req.from,
            amount,
            ledger_nonce_b64(&[op_label, idem, &req.from, &nonce_string, "debit"]),
        )?;

        let credit = self.entry(
            format!("{txid}:{op_label}:credit"),
            ts,
            credit_kind,
            &req.to,
            amount,
            ledger_nonce_b64(&[op_label, idem, &req.to, &nonce_string, "credit"]),
        )?;

        let resp = self.ledger.ingest(IngestRequest {
            batch: vec![debit, credit],
            idem_id: Some(idem.to_string()),
        })?;

        if !resp.accepted {
            return Err(WalletError::nonce_conflict(format!(
                "ledger rejected {op_label} batch"
            )));
        }

        finalize_receipt(Receipt {
            txid,
            op,
            from: Some(req.from.clone()),
            to: Some(req.to.clone()),
            asset: req.asset.clone(),
            amount_minor: req.amount_minor,
            nonce: Some(req.nonce),
            idem: idem.to_string(),
            ts,
            ledger_seq_start: resp.seq_start.map(|seq| seq.get()),
            ledger_seq_end: resp.seq_end.map(|seq| seq.get()),
            ledger_root: resp.new_root.to_hex(),
            settlement_status: ReceiptSettlementStatus::Accepted,
            receipt_hash: String::new(),
        })
    }

    fn entry(
        &self,
        id: String,
        ts: u64,
        kind: EntryKind,
        account: &str,
        amount: u64,
        nonce_b64: String,
    ) -> WalletResult<Entry> {
        Ok(Entry::new(
            id,
            ts,
            kind,
            AccountId::new(account)?,
            amount,
            Nonce::from_base64(nonce_b64)?,
            Kid::new(self.identity.kid.clone())?,
            CapabilityRef::new(self.identity.capability_ref.clone())?,
            1,
        )?)
    }
}

fn now_millis() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    u64::try_from(millis).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_and_transfer_flow_updates_balances() {
        let cfg = WalletConfig::default();
        let client = LocalLedgerClient::in_memory().unwrap();

        let issue = IssueRequest {
            to: "acct_a".into(),
            asset: "roc".into(),
            amount_minor: AmountMinor(100),
            idempotency_key: None,
            memo: None,
        };

        client.issue(&cfg, &issue, "idem_issue").unwrap();

        assert_eq!(
            client.balance(&cfg, "acct_a").unwrap().amount_minor.get(),
            100
        );

        let transfer = TransferRequest {
            from: "acct_a".into(),
            to: "acct_b".into(),
            asset: "roc".into(),
            amount_minor: AmountMinor(40),
            nonce: 1,
            idempotency_key: None,
            memo: None,
        };

        client.transfer(&cfg, &transfer, "idem_transfer").unwrap();

        assert_eq!(
            client.balance(&cfg, "acct_a").unwrap().amount_minor.get(),
            60
        );
        assert_eq!(
            client.balance(&cfg, "acct_b").unwrap().amount_minor.get(),
            40
        );
    }

    #[test]
    fn hold_capture_release_flow_updates_escrow_balances() {
        let cfg = WalletConfig::default();
        let client = LocalLedgerClient::in_memory().unwrap();

        let issue = IssueRequest {
            to: "acct_user".into(),
            asset: "roc".into(),
            amount_minor: AmountMinor(100),
            idempotency_key: None,
            memo: None,
        };
        client.issue(&cfg, &issue, "idem_issue_user").unwrap();

        let hold = TransferRequest {
            from: "acct_user".into(),
            to: "escrow_hold_1".into(),
            asset: "roc".into(),
            amount_minor: AmountMinor(70),
            nonce: 1,
            idempotency_key: None,
            memo: Some("storage hold".into()),
        };
        let hold_receipt = client.hold(&cfg, &hold, "idem_hold_1").unwrap();

        assert_eq!(hold_receipt.op, WalletOp::Hold);
        assert_eq!(
            client
                .balance(&cfg, "acct_user")
                .unwrap()
                .amount_minor
                .get(),
            30
        );
        assert_eq!(
            client
                .balance(&cfg, "escrow_hold_1")
                .unwrap()
                .amount_minor
                .get(),
            70
        );

        let capture = TransferRequest {
            from: "escrow_hold_1".into(),
            to: "svc_storage".into(),
            asset: "roc".into(),
            amount_minor: AmountMinor(40),
            nonce: 1,
            idempotency_key: None,
            memo: Some("storage capture".into()),
        };
        let capture_receipt = client.capture(&cfg, &capture, "idem_capture_1").unwrap();

        assert_eq!(capture_receipt.op, WalletOp::Capture);
        assert_eq!(
            client
                .balance(&cfg, "escrow_hold_1")
                .unwrap()
                .amount_minor
                .get(),
            30
        );
        assert_eq!(
            client
                .balance(&cfg, "svc_storage")
                .unwrap()
                .amount_minor
                .get(),
            40
        );

        let release = TransferRequest {
            from: "escrow_hold_1".into(),
            to: "acct_user".into(),
            asset: "roc".into(),
            amount_minor: AmountMinor(30),
            nonce: 2,
            idempotency_key: None,
            memo: Some("storage release".into()),
        };
        let release_receipt = client.release(&cfg, &release, "idem_release_1").unwrap();

        assert_eq!(release_receipt.op, WalletOp::Release);
        assert_eq!(
            client
                .balance(&cfg, "escrow_hold_1")
                .unwrap()
                .amount_minor
                .get(),
            0
        );
        assert_eq!(
            client
                .balance(&cfg, "acct_user")
                .unwrap()
                .amount_minor
                .get(),
            60
        );

        let total = client
            .balance(&cfg, "acct_user")
            .unwrap()
            .amount_minor
            .get()
            + client
                .balance(&cfg, "escrow_hold_1")
                .unwrap()
                .amount_minor
                .get()
            + client
                .balance(&cfg, "svc_storage")
                .unwrap()
                .amount_minor
                .get();

        assert_eq!(total, 100);
    }
}
