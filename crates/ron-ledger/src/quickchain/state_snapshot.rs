//! RO:WHAT — Deterministic, ordered, non-hashing snapshots of QuickChain account and active-hold execution state.
//! RO:WHY — ECON/RES: ledger-owned state must become boring, explicit projection data before any leaf, hash, root, or checkpoint work.
//! RO:INTERACTS — execution_state, balance_state, hold_state, replay_index, and future reviewed ron-proto payload adapters.
//! RO:INVARIANTS — sorted account/hold order; exact integer arithmetic; terminal holds excluded; no serde, hashes, roots, clocks, randomness, or mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none; available only through the quickchain-preflight feature.
//! RO:SECURITY — snapshots contain public economic state only and grant no wallet, policy, receipt, or spend authority.
//! RO:TEST — tests/quickchain_state_snapshot.rs.

use std::collections::BTreeMap;

use thiserror::Error;

use super::{execution_state::QuickChainAtomicState, hold_transition::validate_combined_state};

/// Failure to project an internally inconsistent QuickChain execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum QuickChainStateSnapshotError {
    /// Stored balances, held totals, supply, chain binding, or ordering evidence disagree.
    #[error("QuickChain state invariant violation during deterministic snapshot projection")]
    StateInvariantViolation,
}

/// Ledger-derived state for one account at the instant a snapshot is captured.
///
/// `account_sequence` advances once for every accepted operation that changes
/// this account's leaf-relevant balance or owned-hold state. For two-account
/// value movement, both distinct participants advance exactly once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainAccountSnapshot {
    account_id: String,
    balance_minor: u128,
    held_minor: u128,
    available_minor: u128,
    account_sequence: u64,
}

impl QuickChainAccountSnapshot {
    /// Account identifier represented by this snapshot row.
    #[must_use]
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Total ROC balance before subtracting active reservations.
    #[must_use]
    pub const fn balance_minor(&self) -> u128 {
        self.balance_minor
    }

    /// ROC reserved by currently active holds.
    #[must_use]
    pub const fn held_minor(&self) -> u128 {
        self.held_minor
    }

    /// Spendable ROC after subtracting active reservations.
    #[must_use]
    pub const fn available_minor(&self) -> u128 {
        self.available_minor
    }

    /// Last ledger-assigned sequence for a leaf-relevant account-state change.
    #[must_use]
    pub const fn account_sequence(&self) -> u64 {
        self.account_sequence
    }
}

/// Ledger-derived state for one currently open hold.
///
/// Epoch values remain numeric execution inputs. They are intentionally not
/// converted into the canonical string epoch identifiers required by future
/// ron-proto leaf payloads. Purpose and policy identity are also intentionally
/// absent because the current hold state does not own those values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainActiveHoldSnapshot {
    hold_id: String,
    account_id: String,
    counterparty_account_id: Option<String>,
    amount_minor: u128,
    created_at_epoch_number: u64,
    expires_at_epoch_number: u64,
    opened_operation_id: String,
    opened_idempotency_key: String,
}

impl QuickChainActiveHoldSnapshot {
    /// Durable identifier for this hold lifecycle.
    #[must_use]
    pub fn hold_id(&self) -> &str {
        &self.hold_id
    }

    /// Account whose balance is reserved.
    #[must_use]
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Optional beneficiary fixed when the hold opened.
    #[must_use]
    pub fn counterparty_account_id(&self) -> Option<&str> {
        self.counterparty_account_id.as_deref()
    }

    /// ROC currently reserved by this active hold.
    #[must_use]
    pub const fn amount_minor(&self) -> u128 {
        self.amount_minor
    }

    /// Numeric execution epoch in which the hold opened.
    #[must_use]
    pub const fn created_at_epoch_number(&self) -> u64 {
        self.created_at_epoch_number
    }

    /// Numeric execution epoch at which expiry first becomes eligible.
    #[must_use]
    pub const fn expires_at_epoch_number(&self) -> u64 {
        self.expires_at_epoch_number
    }

    /// Durable operation identity that opened the hold.
    #[must_use]
    pub fn opened_operation_id(&self) -> &str {
        &self.opened_operation_id
    }

    /// Scoped retry key carried by the opening operation.
    #[must_use]
    pub fn opened_idempotency_key(&self) -> &str {
        &self.opened_idempotency_key
    }
}

/// Ordered, read-only projection of the current QuickChain atomic state.
///
/// This is not a serialized snapshot, hash payload, Merkle leaf, root, proof,
/// checkpoint, receipt, persistence format, or settlement artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainStateSnapshot {
    chain_id: Option<String>,
    accounts: Vec<QuickChainAccountSnapshot>,
    active_holds: Vec<QuickChainActiveHoldSnapshot>,
    current_supply_minor: u128,
    next_ledger_sequence: u64,
    operation_count: usize,
}

impl QuickChainStateSnapshot {
    fn capture(state: &QuickChainAtomicState) -> Result<Self, QuickChainStateSnapshotError> {
        validate_combined_state(state.balance_state(), state.hold_state())
            .map_err(|_| QuickChainStateSnapshotError::StateInvariantViolation)?;

        let operation_count = state.operation_count();
        let next_ledger_sequence = state.next_ledger_sequence();
        let chain_id = state.replay_index().chain_id().map(str::to_owned);

        match (chain_id.is_some(), operation_count) {
            (false, 0) => {
                if next_ledger_sequence != 1 {
                    return Err(QuickChainStateSnapshotError::StateInvariantViolation);
                }
            }
            (true, count) if count > 0 => {}
            _ => return Err(QuickChainStateSnapshotError::StateInvariantViolation),
        }

        let mut derived_held_by_account = BTreeMap::<String, u128>::new();
        let mut active_holds = Vec::with_capacity(state.hold_state().active_hold_count());

        for (map_hold_id, hold) in state.hold_state().ordered_active_holds() {
            if map_hold_id != hold.hold_id()
                || hold.amount_minor() == 0
                || hold.expires_at_epoch() <= hold.created_at_epoch()
            {
                return Err(QuickChainStateSnapshotError::StateInvariantViolation);
            }

            let held_total = derived_held_by_account
                .entry(hold.account_id().to_string())
                .or_insert(0);

            *held_total = held_total
                .checked_add(hold.amount_minor())
                .ok_or(QuickChainStateSnapshotError::StateInvariantViolation)?;

            active_holds.push(QuickChainActiveHoldSnapshot {
                hold_id: hold.hold_id().to_string(),
                account_id: hold.account_id().to_string(),
                counterparty_account_id: hold.counterparty_account_id().map(str::to_owned),
                amount_minor: hold.amount_minor(),
                created_at_epoch_number: hold.created_at_epoch(),
                expires_at_epoch_number: hold.expires_at_epoch(),
                opened_operation_id: hold.opened_operation_id().to_string(),
                opened_idempotency_key: hold.opened_idempotency_key().to_string(),
            });
        }

        let mut accounts = Vec::with_capacity(state.balance_state().account_count());
        let mut summed_supply_minor = 0_u128;

        for (account_id, balance_minor) in state.balance_state().ordered_balances() {
            let held_minor = state.held_minor(account_id);
            let derived_held_minor = derived_held_by_account.remove(account_id).unwrap_or(0);

            if held_minor != derived_held_minor {
                return Err(QuickChainStateSnapshotError::StateInvariantViolation);
            }

            let available_minor = balance_minor
                .checked_sub(held_minor)
                .ok_or(QuickChainStateSnapshotError::StateInvariantViolation)?;

            summed_supply_minor = summed_supply_minor
                .checked_add(balance_minor)
                .ok_or(QuickChainStateSnapshotError::StateInvariantViolation)?;

            accounts.push(QuickChainAccountSnapshot {
                account_id: account_id.to_string(),
                balance_minor,
                held_minor,
                available_minor,
                account_sequence: state.last_account_sequence(account_id),
            });
        }

        if !derived_held_by_account.is_empty()
            || summed_supply_minor != state.current_supply_minor()
        {
            return Err(QuickChainStateSnapshotError::StateInvariantViolation);
        }

        if chain_id.is_none()
            && (!accounts.is_empty()
                || !active_holds.is_empty()
                || state.current_supply_minor() != 0)
        {
            return Err(QuickChainStateSnapshotError::StateInvariantViolation);
        }

        Ok(Self {
            chain_id,
            accounts,
            active_holds,
            current_supply_minor: state.current_supply_minor(),
            next_ledger_sequence,
            operation_count,
        })
    }

    /// Chain identity bound by the first accepted operation, or `None` for empty state.
    #[must_use]
    pub fn chain_id(&self) -> Option<&str> {
        self.chain_id.as_deref()
    }

    /// Account rows in ascending bytewise account-ID order.
    #[must_use]
    pub fn accounts(&self) -> &[QuickChainAccountSnapshot] {
        &self.accounts
    }

    /// Open-hold rows in ascending bytewise hold-ID order.
    #[must_use]
    pub fn active_holds(&self) -> &[QuickChainActiveHoldSnapshot] {
        &self.active_holds
    }

    /// Current circulating ROC supply represented by all account rows.
    #[must_use]
    pub const fn current_supply_minor(&self) -> u128 {
        self.current_supply_minor
    }

    /// Next primitive ledger sequence after all accepted operations in this state.
    #[must_use]
    pub const fn next_ledger_sequence(&self) -> u64 {
        self.next_ledger_sequence
    }

    /// Number of accepted operation records represented by the source state.
    #[must_use]
    pub const fn operation_count(&self) -> usize {
        self.operation_count
    }
}

impl QuickChainAtomicState {
    /// Capture a deterministic, ordered, non-hashing projection of current state.
    ///
    /// The source state is only read. No clock, randomness, serialization,
    /// hashing, persistence, root production, or receipt construction occurs.
    pub fn state_snapshot(&self) -> Result<QuickChainStateSnapshot, QuickChainStateSnapshotError> {
        QuickChainStateSnapshot::capture(self)
    }
}
