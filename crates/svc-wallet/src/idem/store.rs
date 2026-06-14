//! RO:WHAT — RAM idempotency store for deterministic retry/replay behavior.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. Provides at-most-once visible results at the wallet boundary.
//! RO:INTERACTS — routes/v1 write handlers, util::blake3_receipt, dto::responses::Receipt.
//! RO:INVARIANTS — key+fingerprint returns byte-identical receipt; key+different fingerprint returns 409 conflict.
//! RO:METRICS — caller increments wallet_idem_replays_total on Some(receipt).
//! RO:CONFIG — TTL configured by WalletConfig; amnesia keeps this RAM-only.
//! RO:SECURITY — no Authorization headers or secrets are stored.
//! RO:TEST — replay_and_conflict_paths.

use std::{collections::HashMap, time::Duration};

use parking_lot::Mutex;

use crate::{
    dto::responses::Receipt,
    errors::{WalletError, WalletResult},
};

#[derive(Debug, Clone)]
struct StoredDecision {
    fingerprint: String,
    receipt: Receipt,
    expires_at_ms: u64,
}

/// In-memory idempotency store.
#[derive(Debug)]
pub struct IdempotencyStore {
    ttl: Duration,
    inner: Mutex<HashMap<String, StoredDecision>>,
}

impl IdempotencyStore {
    /// Build a RAM idempotency store.
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Lookup a key. Returns Some(receipt) for same request replay.
    pub fn lookup(
        &self,
        key: &str,
        fingerprint: &str,
        now_ms: u64,
    ) -> WalletResult<Option<Receipt>> {
        self.purge_expired(now_ms);
        let guard = self.inner.lock();
        let Some(stored) = guard.get(key) else {
            return Ok(None);
        };
        if stored.fingerprint != fingerprint {
            return Err(WalletError::idempotency_conflict(
                "same Idempotency-Key used with different request body",
            ));
        }
        Ok(Some(stored.receipt.clone()))
    }

    /// Insert a successful receipt.
    pub fn insert(&self, key: String, fingerprint: String, receipt: Receipt, now_ms: u64) {
        let ttl_ms_u128 = self.ttl.as_millis();
        let ttl_ms = u64::try_from(ttl_ms_u128).unwrap_or(u64::MAX);
        let expires_at_ms = now_ms.saturating_add(ttl_ms);
        self.inner.lock().insert(
            key,
            StoredDecision {
                fingerprint,
                receipt,
                expires_at_ms,
            },
        );
    }

    /// Purge expired entries.
    pub fn purge_expired(&self, now_ms: u64) {
        self.inner
            .lock()
            .retain(|_, stored| stored.expires_at_ms > now_ms);
    }

    /// Return current RAM entry count.
    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    /// True if no entries are stored.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{
        requests::AmountMinor,
        responses::{ReceiptSettlementStatus, WalletOp},
    };

    fn receipt() -> Receipt {
        Receipt {
            txid: "tx_a".into(),
            op: WalletOp::Issue,
            from: None,
            to: Some("acct".into()),
            asset: "roc".into(),
            amount_minor: AmountMinor(1),
            nonce: None,
            idem: "idem".into(),
            ts: 1,
            ledger_seq_start: Some(1),
            ledger_seq_end: Some(1),
            ledger_root: "00".repeat(32),
            settlement_status: ReceiptSettlementStatus::Accepted,
            receipt_hash: "b3:test".into(),
        }
    }

    #[test]
    fn replay_and_conflict_paths() {
        let store = IdempotencyStore::new(Duration::from_secs(60));
        store.insert("k".into(), "fp1".into(), receipt(), 0);
        assert!(store.lookup("k", "fp1", 1).unwrap().is_some());
        assert!(store.lookup("k", "fp2", 1).is_err());
    }
}
