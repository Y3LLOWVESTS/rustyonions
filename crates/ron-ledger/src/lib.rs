//! ron-ledger: token ledger core (traits + reference in-memory implementation).
//!
//! Goals:
//! - Storage-agnostic trait (`TokenLedger`) for mint/burn/transfer, balances, supply.
//! - Safe invariants: non-negative balances; conservation of supply; overflow checks.
//! - Simple in-memory impl for early integration; swap backends later (SQLite/sled/…).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub type Amount = u128;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub String);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Op {
    Mint,
    Burn,
    Transfer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: u64,
    pub ts_ms: u128,
    pub op: Op,
    pub from: Option<AccountId>,
    pub to: Option<AccountId>,
    pub amount: Amount,
    pub reason: Option<String>,
    pub supply_after: Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub entry_id: u64,
    pub balance_after: Option<Amount>,
    pub supply_after: Amount,
}

#[derive(Debug)]
pub enum TokenError {
    ZeroAmount,
    InsufficientFunds { account: AccountId, needed: Amount, available: Amount },
    Overflow,
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenError::ZeroAmount => write!(f, "amount must be > 0"),
            TokenError::InsufficientFunds { account, needed, available } =>
                write!(f, "insufficient funds in {}: need {}, have {}", account.0, needed, available),
            TokenError::Overflow => write!(f, "arithmetic overflow"),
        }
    }
}
impl std::error::Error for TokenError {}

/// Storage-agnostic token ledger interface.
///
/// NOTE: trait is sync & non-async to keep implementations flexible (can
/// layer async at service boundary if needed).
pub trait TokenLedger {
    fn total_supply(&self) -> Amount;
    fn balance(&self, account: &AccountId) -> Amount;
    fn entries(&self) -> Vec<LedgerEntry>; // copy out; backends may add streaming later

    fn mint(&mut self, to: AccountId, amount: Amount, reason: Option<String>) -> Result<Receipt, TokenError>;
    fn burn(&mut self, from: AccountId, amount: Amount, reason: Option<String>) -> Result<Receipt, TokenError>;
    fn transfer(&mut self, from: AccountId, to: AccountId, amount: Amount, reason: Option<String>) -> Result<Receipt, TokenError>;
}

/// A simple in-memory, single-threaded ledger. Wrap in a lock for concurrency.
#[derive(Debug, Default)]
pub struct InMemoryLedger {
    next_id: u64,
    total_supply: Amount,
    balances: HashMap<AccountId, Amount>,
    entries: Vec<LedgerEntry>,
}

impl InMemoryLedger {
    pub fn new() -> Self {
        Self::default()
    }

    fn push_entry(
        &mut self,
        op: Op,
        from: Option<AccountId>,
        to: Option<AccountId>,
        amount: Amount,
        reason: Option<String>,
    ) -> &LedgerEntry {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let ts_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        self.entries.push(LedgerEntry {
            id,
            ts_ms,
            op,
            from,
            to,
            amount,
            reason,
            supply_after: self.total_supply,
        });
        self.entries.last().unwrap()
    }
}

impl TokenLedger for InMemoryLedger {
    #[inline]
    fn total_supply(&self) -> Amount {
        self.total_supply
    }

    #[inline]
    fn balance(&self, account: &AccountId) -> Amount {
        *self.balances.get(account).unwrap_or(&0)
    }

    fn entries(&self) -> Vec<LedgerEntry> {
        self.entries.clone()
    }

    fn mint(
        &mut self,
        to: AccountId,
        amount: Amount,
        reason: Option<String>,
    ) -> Result<Receipt, TokenError> {
        if amount == 0 {
            return Err(TokenError::ZeroAmount);
        }
        let bal = self.balances.entry(to.clone()).or_insert(0);
        *bal = bal.checked_add(amount).ok_or(TokenError::Overflow)?;
        self.total_supply = self.total_supply.checked_add(amount).ok_or(TokenError::Overflow)?;
        let entry = self.push_entry(Op::Mint, None, Some(to.clone()), amount, reason);
        Ok(Receipt {
            entry_id: entry.id,
            balance_after: Some(*bal),
            supply_after: entry.supply_after,
        })
    }

    fn burn(
        &mut self,
        from: AccountId,
        amount: Amount,
        reason: Option<String>,
    ) -> Result<Receipt, TokenError> {
        if amount == 0 {
            return Err(TokenError::ZeroAmount);
        }
        let bal = self.balances.entry(from.clone()).or_insert(0);
        if *bal < amount {
            return Err(TokenError::InsufficientFunds {
                account: from,
                needed: amount,
                available: *bal,
            });
        }
        *bal -= amount;
        self.total_supply = self.total_supply.checked_sub(amount).ok_or(TokenError::Overflow)?;
        let entry = self.push_entry(Op::Burn, None, None, amount, reason);
        Ok(Receipt {
            entry_id: entry.id,
            balance_after: Some(*bal),
            supply_after: entry.supply_after,
        })
    }

    fn transfer(
        &mut self,
        from: AccountId,
        to: AccountId,
        amount: Amount,
        reason: Option<String>,
    ) -> Result<Receipt, TokenError> {
        if amount == 0 {
            return Err(TokenError::ZeroAmount);
        }
        if from == to {
            // no-op; surface current state
            return Ok(Receipt {
                entry_id: self.next_id,
                balance_after: Some(self.balance(&to)),
                supply_after: self.total_supply,
            });
        }
        // debit
        let from_bal = self.balances.entry(from.clone()).or_insert(0);
        if *from_bal < amount {
            return Err(TokenError::InsufficientFunds {
                account: from,
                needed: amount,
                available: *from_bal,
            });
        }
        *from_bal -= amount;

        // credit
        let to_bal = self.balances.entry(to.clone()).or_insert(0);
        *to_bal = to_bal.checked_add(amount).ok_or(TokenError::Overflow)?;

        // supply unchanged
        let entry = self.push_entry(Op::Transfer, Some(from), Some(to.clone()), amount, reason);
        Ok(Receipt {
            entry_id: entry.id,
            balance_after: Some(*to_bal),
            supply_after: entry.supply_after,
        })
    }
}
