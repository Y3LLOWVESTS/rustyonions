//! RO:WHAT — In-memory strict per-account nonce reservation table.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. Prevents double-spend races before ledger commit.
//! RO:INTERACTS — routes/v1 write handlers, idem store, ledger client.
//! RO:INVARIANTS — reserve exactly last+1; rollback on failed commit; no async lock holding.
//! RO:METRICS — caller increments wallet_conflicts_total on nonce conflicts.
//! RO:CONFIG — NONCE_START.
//! RO:SECURITY — stores account ids and nonce counters only.
//! RO:TEST — enforces_strict_next; rollback_restores_previous.

use std::collections::HashMap;

use parking_lot::Mutex;

use crate::{
    config::NONCE_START,
    errors::{WalletError, WalletResult},
};

/// Strict in-memory nonce table.
#[derive(Debug, Default)]
pub struct NonceTable {
    inner: Mutex<HashMap<String, u64>>,
}

/// Reservation token returned after a successful strict reserve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonceReservation {
    account: String,
    previous: Option<u64>,
    nonce: u64,
    committed: bool,
}

impl NonceReservation {
    /// Reserved account.
    pub fn account(&self) -> &str {
        &self.account
    }

    /// Reserved nonce.
    pub const fn nonce(&self) -> u64 {
        self.nonce
    }

    /// Mark reservation as committed.
    pub fn commit(mut self) {
        self.committed = true;
    }
}

impl NonceTable {
    /// Return last committed nonce for an account.
    pub fn last_nonce(&self, account: &str) -> Option<u64> {
        self.inner.lock().get(account).copied()
    }

    /// Reserve the strict next nonce for an account.
    pub fn reserve_strict(&self, account: &str, nonce: u64) -> WalletResult<NonceReservation> {
        let mut guard = self.inner.lock();
        let previous = guard.get(account).copied();
        let expected = previous.map_or(NONCE_START, |last| last.saturating_add(1));
        if nonce != expected {
            return Err(WalletError::nonce_conflict(format!(
                "nonce conflict for account {account}: expected {expected}, got {nonce}"
            )));
        }
        guard.insert(account.to_string(), nonce);
        Ok(NonceReservation {
            account: account.to_string(),
            previous,
            nonce,
            committed: false,
        })
    }

    /// Roll back an uncommitted reservation.
    pub fn rollback(&self, reservation: NonceReservation) {
        if reservation.committed {
            return;
        }
        let mut guard = self.inner.lock();
        match reservation.previous {
            Some(prev) => {
                guard.insert(reservation.account, prev);
            }
            None => {
                guard.remove(&reservation.account);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_strict_next() {
        let table = NonceTable::default();
        let r1 = table.reserve_strict("acct", 1).unwrap();
        r1.commit();
        assert!(table.reserve_strict("acct", 1).is_err());
        assert!(table.reserve_strict("acct", 3).is_err());
        table.reserve_strict("acct", 2).unwrap().commit();
    }

    #[test]
    fn rollback_restores_previous() {
        let table = NonceTable::default();
        let r1 = table.reserve_strict("acct", 1).unwrap();
        table.rollback(r1);
        assert_eq!(table.last_nonce("acct"), None);
        table.reserve_strict("acct", 1).unwrap().commit();
    }
}
