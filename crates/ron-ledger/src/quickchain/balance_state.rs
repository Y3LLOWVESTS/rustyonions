//! RO:WHAT — Deterministically ordered in-memory ROC balance and supply state for QuickChain preflight transitions.
//! RO:WHY — ECON/RES: checked account arithmetic must be independently testable before persistence, receipts, holds, or roots.
//! RO:INTERACTS — transition.rs, transition_error.rs, future replay/executor composition.
//! RO:INVARIANTS — BTreeMap ordering; sum(accounts)=issued-burned=current supply; no saturation or floating point.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — state contains economic values only; it grants no wallet or supply authority.
//! RO:TEST — tests/quickchain_balance_transition.rs.

use std::collections::BTreeMap;

use super::transition_error::QuickChainTransitionError;

/// Pure deterministic ROC balance and supply state.
///
/// This is not yet persistent ledger state and does not contain holds, receipts,
/// operation indexes, account sequences, roots, or checkpoint information.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuickChainBalanceState {
    balances: BTreeMap<String, u128>,
    total_issued_minor: u128,
    total_burned_minor: u128,
    current_supply_minor: u128,
}

impl QuickChainBalanceState {
    /// Create an empty ROC balance state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return an account balance, treating an untouched account as zero.
    #[must_use]
    pub fn balance_minor(&self, account_id: &str) -> u128 {
        self.balances.get(account_id).copied().unwrap_or(0)
    }

    /// Return the cumulative amount explicitly issued by accepted transitions.
    #[must_use]
    pub const fn total_issued_minor(&self) -> u128 {
        self.total_issued_minor
    }

    /// Return the cumulative amount explicitly burned by accepted transitions.
    #[must_use]
    pub const fn total_burned_minor(&self) -> u128 {
        self.total_burned_minor
    }

    /// Return the current circulating ROC supply represented by this state.
    #[must_use]
    pub const fn current_supply_minor(&self) -> u128 {
        self.current_supply_minor
    }

    /// Return the number of accounts touched by accepted transitions.
    #[must_use]
    pub fn account_count(&self) -> usize {
        self.balances.len()
    }

    /// Iterate over account balances in deterministic account-id order.
    pub fn ordered_balances(&self) -> impl Iterator<Item = (&str, u128)> + '_ {
        self.balances
            .iter()
            .map(|(account_id, balance)| (account_id.as_str(), *balance))
    }

    pub(crate) fn set_balance(&mut self, account_id: String, balance_minor: u128) {
        self.balances.insert(account_id, balance_minor);
    }

    pub(crate) fn set_supply_counters(
        &mut self,
        total_issued_minor: u128,
        total_burned_minor: u128,
        current_supply_minor: u128,
    ) {
        self.total_issued_minor = total_issued_minor;
        self.total_burned_minor = total_burned_minor;
        self.current_supply_minor = current_supply_minor;
    }

    pub(crate) fn validate_invariants(&self) -> Result<(), QuickChainTransitionError> {
        let expected_supply = self
            .total_issued_minor
            .checked_sub(self.total_burned_minor)
            .ok_or(QuickChainTransitionError::SupplyUnderflow)?;

        if expected_supply != self.current_supply_minor {
            return Err(QuickChainTransitionError::StateInvariantViolation);
        }

        let summed_balances = self.balances.values().try_fold(0_u128, |sum, balance| {
            sum.checked_add(*balance)
                .ok_or(QuickChainTransitionError::StateInvariantViolation)
        })?;

        if summed_balances != self.current_supply_minor {
            return Err(QuickChainTransitionError::StateInvariantViolation);
        }

        Ok(())
    }
}
