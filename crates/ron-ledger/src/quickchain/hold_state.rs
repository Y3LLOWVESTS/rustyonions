//! RO:WHAT — Deterministically ordered active-hold state, terminal hold evidence, and per-account held totals.
//! RO:WHY — ECON/RES: concurrent holds must reserve available value without losing durable lifecycle or beneficiary identity.
//! RO:INTERACTS — hold_error.rs, future hold_transition.rs, execution_state.rs, balance_state.rs, and replay persistence.
//! RO:INVARIANTS — BTreeMap ordering; active/terminal IDs are disjoint; held totals equal active-hold sums; expiry is epoch-based.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — records contain public economic identifiers only and grant no wallet, policy, or spend authority.
//! RO:TEST — future tests/quickchain_hold_transition.rs and tests/quickchain_hold_atomic_execution.rs.

use std::collections::BTreeMap;

use super::hold_error::QuickChainHoldError;

/// Terminal state reached by one completed hold lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum QuickChainHoldTerminalStatus {
    /// Reserved value was captured for a beneficiary.
    Captured,

    /// Reserved value was returned to the holder's available balance.
    Released,

    /// Reserved value was deterministically released after epoch eligibility.
    Expired,
}

/// One currently open hold.
///
/// This is internal execution state, not the frozen wire DTO and not a future
/// active-hold root leaf. Root-specific fields remain deferred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainOpenHoldRecord {
    hold_id: String,
    account_id: String,
    counterparty_account_id: Option<String>,
    amount_minor: u128,
    created_at_epoch: u64,
    expires_at_epoch: u64,
    opened_operation_id: String,
    opened_idempotency_key: String,
    opened_account_sequence: u64,
    opened_ledger_sequence_start: u64,
    opened_ledger_sequence_end: u64,
}

impl QuickChainOpenHoldRecord {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        hold_id: String,
        account_id: String,
        counterparty_account_id: Option<String>,
        amount_minor: u128,
        created_at_epoch: u64,
        expires_at_epoch: u64,
        opened_operation_id: String,
        opened_idempotency_key: String,
        opened_account_sequence: u64,
        opened_ledger_sequence_start: u64,
        opened_ledger_sequence_end: u64,
    ) -> Result<Self, QuickChainHoldError> {
        if amount_minor == 0 {
            return Err(QuickChainHoldError::ZeroAmount);
        }

        if expires_at_epoch <= created_at_epoch {
            return Err(QuickChainHoldError::InvalidEpochRange {
                created_at_epoch,
                expires_at_epoch,
            });
        }

        if hold_id.is_empty()
            || account_id.is_empty()
            || opened_operation_id.is_empty()
            || opened_idempotency_key.is_empty()
            || opened_account_sequence == 0
            || opened_ledger_sequence_start == 0
            || opened_ledger_sequence_end < opened_ledger_sequence_start
        {
            return Err(QuickChainHoldError::StateInvariantViolation);
        }

        Ok(Self {
            hold_id,
            account_id,
            counterparty_account_id,
            amount_minor,
            created_at_epoch,
            expires_at_epoch,
            opened_operation_id,
            opened_idempotency_key,
            opened_account_sequence,
            opened_ledger_sequence_start,
            opened_ledger_sequence_end,
        })
    }

    /// Durable identifier for this one hold lifecycle.
    #[must_use]
    pub fn hold_id(&self) -> &str {
        &self.hold_id
    }

    /// Holder account whose value is reserved.
    #[must_use]
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Optional beneficiary fixed when the hold opened.
    #[must_use]
    pub fn counterparty_account_id(&self) -> Option<&str> {
        self.counterparty_account_id.as_deref()
    }

    /// Value currently reserved by this open hold.
    #[must_use]
    pub const fn amount_minor(&self) -> u128 {
        self.amount_minor
    }

    /// Deterministic epoch in which the hold opened.
    #[must_use]
    pub const fn created_at_epoch(&self) -> u64 {
        self.created_at_epoch
    }

    /// First deterministic epoch in which expiry is eligible.
    #[must_use]
    pub const fn expires_at_epoch(&self) -> u64 {
        self.expires_at_epoch
    }

    /// Durable operation identity that opened the hold.
    #[must_use]
    pub fn opened_operation_id(&self) -> &str {
        &self.opened_operation_id
    }

    /// Scoped retry key that opened the hold.
    #[must_use]
    pub fn opened_idempotency_key(&self) -> &str {
        &self.opened_idempotency_key
    }

    /// Ledger-assigned holder-account sequence for the opening operation.
    #[must_use]
    pub const fn opened_account_sequence(&self) -> u64 {
        self.opened_account_sequence
    }

    /// First primitive ledger sequence occupied by the opening operation.
    #[must_use]
    pub const fn opened_ledger_sequence_start(&self) -> u64 {
        self.opened_ledger_sequence_start
    }

    /// Last primitive ledger sequence occupied by the opening operation.
    #[must_use]
    pub const fn opened_ledger_sequence_end(&self) -> u64 {
        self.opened_ledger_sequence_end
    }

    /// Return whether this hold is eligible for deterministic expiry.
    #[must_use]
    pub const fn is_expiry_eligible(&self, current_epoch: u64) -> bool {
        current_epoch >= self.expires_at_epoch
    }
}

/// Durable terminal evidence for one closed hold lifecycle.
///
/// Terminal records do not contribute to held totals. Both the optional
/// beneficiary fixed at opening and the actual capture beneficiary are retained,
/// because `hold_open` permits no counterparty while `hold_capture` requires one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainTerminalHoldRecord {
    hold_id: String,
    account_id: String,
    opened_counterparty_account_id: Option<String>,
    terminal_counterparty_account_id: Option<String>,
    original_amount_minor: u128,
    terminal_amount_minor: u128,
    status: QuickChainHoldTerminalStatus,
    created_at_epoch: u64,
    expires_at_epoch: u64,
    terminal_at_epoch: u64,
    opened_operation_id: String,
    terminal_operation_id: String,
    terminal_receipt_txid: String,
    opened_account_sequence: u64,
    terminal_account_sequence: u64,
    terminal_ledger_sequence_start: u64,
    terminal_ledger_sequence_end: u64,
}

impl QuickChainTerminalHoldRecord {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_open(
        open: &QuickChainOpenHoldRecord,
        terminal_counterparty_account_id: Option<String>,
        terminal_amount_minor: u128,
        status: QuickChainHoldTerminalStatus,
        terminal_at_epoch: u64,
        terminal_operation_id: String,
        terminal_receipt_txid: String,
        terminal_account_sequence: u64,
        terminal_ledger_sequence_start: u64,
        terminal_ledger_sequence_end: u64,
    ) -> Result<Self, QuickChainHoldError> {
        if terminal_amount_minor == 0 {
            return Err(QuickChainHoldError::ZeroAmount);
        }

        if terminal_amount_minor > open.amount_minor {
            return Err(QuickChainHoldError::RequestedAmountExceedsHeld {
                hold_id: open.hold_id.clone(),
                held_minor: open.amount_minor,
                requested_minor: terminal_amount_minor,
            });
        }

        match status {
            QuickChainHoldTerminalStatus::Captured => {
                let actual_counterparty = terminal_counterparty_account_id
                    .as_deref()
                    .ok_or(QuickChainHoldError::StateInvariantViolation)?;

                if let Some(expected_counterparty) = open.counterparty_account_id.as_deref() {
                    if expected_counterparty != actual_counterparty {
                        return Err(QuickChainHoldError::HoldCounterpartyMismatch {
                            hold_id: open.hold_id.clone(),
                            expected_counterparty_account_id: expected_counterparty.to_string(),
                            actual_counterparty_account_id: actual_counterparty.to_string(),
                        });
                    }
                }
            }

            QuickChainHoldTerminalStatus::Released | QuickChainHoldTerminalStatus::Expired => {
                if terminal_counterparty_account_id.is_some() {
                    return Err(QuickChainHoldError::StateInvariantViolation);
                }

                if terminal_amount_minor != open.amount_minor {
                    return Err(QuickChainHoldError::TerminalAmountMismatch {
                        hold_id: open.hold_id.clone(),
                        expected_minor: open.amount_minor,
                        actual_minor: terminal_amount_minor,
                    });
                }
            }
        }

        if terminal_at_epoch < open.created_at_epoch
            || terminal_operation_id.is_empty()
            || terminal_receipt_txid.is_empty()
            || terminal_account_sequence == 0
            || terminal_ledger_sequence_start == 0
            || terminal_ledger_sequence_end < terminal_ledger_sequence_start
        {
            return Err(QuickChainHoldError::StateInvariantViolation);
        }

        Ok(Self {
            hold_id: open.hold_id.clone(),
            account_id: open.account_id.clone(),
            opened_counterparty_account_id: open.counterparty_account_id.clone(),
            terminal_counterparty_account_id,
            original_amount_minor: open.amount_minor,
            terminal_amount_minor,
            status,
            created_at_epoch: open.created_at_epoch,
            expires_at_epoch: open.expires_at_epoch,
            terminal_at_epoch,
            opened_operation_id: open.opened_operation_id.clone(),
            terminal_operation_id,
            terminal_receipt_txid,
            opened_account_sequence: open.opened_account_sequence,
            terminal_account_sequence,
            terminal_ledger_sequence_start,
            terminal_ledger_sequence_end,
        })
    }

    /// Durable identifier for the completed lifecycle.
    #[must_use]
    pub fn hold_id(&self) -> &str {
        &self.hold_id
    }

    /// Holder account whose value was reserved.
    #[must_use]
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Optional beneficiary fixed by the opening operation.
    #[must_use]
    pub fn opened_counterparty_account_id(&self) -> Option<&str> {
        self.opened_counterparty_account_id.as_deref()
    }

    /// Actual beneficiary of a capture operation.
    ///
    /// Release and expiry records always return `None`.
    #[must_use]
    pub fn terminal_counterparty_account_id(&self) -> Option<&str> {
        self.terminal_counterparty_account_id.as_deref()
    }

    /// Amount originally reserved when the hold opened.
    #[must_use]
    pub const fn original_amount_minor(&self) -> u128 {
        self.original_amount_minor
    }

    /// Amount applied by the terminal operation.
    ///
    /// A capture may consume less than the original reservation. Any unused
    /// remainder becomes available when the lifecycle closes. Release and expiry
    /// must equal the complete original reservation.
    #[must_use]
    pub const fn terminal_amount_minor(&self) -> u128 {
        self.terminal_amount_minor
    }

    /// Uncaptured reservation released when a partial capture closes the hold.
    #[must_use]
    pub const fn uncaptured_remainder_minor(&self) -> u128 {
        match self.status {
            QuickChainHoldTerminalStatus::Captured => {
                self.original_amount_minor - self.terminal_amount_minor
            }
            QuickChainHoldTerminalStatus::Released | QuickChainHoldTerminalStatus::Expired => 0,
        }
    }

    /// Terminal lifecycle status.
    #[must_use]
    pub const fn status(&self) -> QuickChainHoldTerminalStatus {
        self.status
    }

    /// Epoch in which the lifecycle opened.
    #[must_use]
    pub const fn created_at_epoch(&self) -> u64 {
        self.created_at_epoch
    }

    /// First epoch in which expiry was eligible.
    #[must_use]
    pub const fn expires_at_epoch(&self) -> u64 {
        self.expires_at_epoch
    }

    /// Epoch in which the terminal transition committed.
    #[must_use]
    pub const fn terminal_at_epoch(&self) -> u64 {
        self.terminal_at_epoch
    }

    /// Operation identity that opened the lifecycle.
    #[must_use]
    pub fn opened_operation_id(&self) -> &str {
        &self.opened_operation_id
    }

    /// Operation identity that closed the lifecycle.
    #[must_use]
    pub fn terminal_operation_id(&self) -> &str {
        &self.terminal_operation_id
    }

    /// Trusted backend receipt reference for the terminal transition.
    #[must_use]
    pub fn terminal_receipt_txid(&self) -> &str {
        &self.terminal_receipt_txid
    }

    /// Holder-account sequence assigned to the opening transition.
    #[must_use]
    pub const fn opened_account_sequence(&self) -> u64 {
        self.opened_account_sequence
    }

    /// Holder-account sequence assigned to the terminal transition.
    #[must_use]
    pub const fn terminal_account_sequence(&self) -> u64 {
        self.terminal_account_sequence
    }

    /// First primitive ledger sequence occupied by the terminal transition.
    #[must_use]
    pub const fn terminal_ledger_sequence_start(&self) -> u64 {
        self.terminal_ledger_sequence_start
    }

    /// Last primitive ledger sequence occupied by the terminal transition.
    #[must_use]
    pub const fn terminal_ledger_sequence_end(&self) -> u64 {
        self.terminal_ledger_sequence_end
    }
}

/// Deterministic active and terminal hold state.
///
/// Active holds contribute to `held_minor`. Terminal records preserve durable
/// lifecycle identity but do not reserve balance.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuickChainHoldState {
    active_holds: BTreeMap<String, QuickChainOpenHoldRecord>,
    terminal_holds: BTreeMap<String, QuickChainTerminalHoldRecord>,
    held_by_account: BTreeMap<String, u128>,
}

impl QuickChainHoldState {
    /// Create empty hold state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of currently open holds.
    #[must_use]
    pub fn active_hold_count(&self) -> usize {
        self.active_holds.len()
    }

    /// Number of terminal lifecycle records retained for replay safety.
    #[must_use]
    pub fn terminal_hold_count(&self) -> usize {
        self.terminal_holds.len()
    }

    /// Return an active hold by lifecycle identifier.
    #[must_use]
    pub fn active_hold(&self, hold_id: &str) -> Option<&QuickChainOpenHoldRecord> {
        self.active_holds.get(hold_id)
    }

    /// Return terminal evidence by lifecycle identifier.
    #[must_use]
    pub fn terminal_hold(&self, hold_id: &str) -> Option<&QuickChainTerminalHoldRecord> {
        self.terminal_holds.get(hold_id)
    }

    /// Return true when a hold ID exists in active or terminal history.
    #[must_use]
    pub fn is_hold_id_used(&self, hold_id: &str) -> bool {
        self.active_holds.contains_key(hold_id) || self.terminal_holds.contains_key(hold_id)
    }

    /// Return the total value reserved by open holds for one account.
    #[must_use]
    pub fn held_minor(&self, account_id: &str) -> u128 {
        self.held_by_account.get(account_id).copied().unwrap_or(0)
    }

    /// Derive available value from total balance and current open holds.
    pub fn available_minor(
        &self,
        account_id: &str,
        balance_minor: u128,
    ) -> Result<u128, QuickChainHoldError> {
        balance_minor
            .checked_sub(self.held_minor(account_id))
            .ok_or(QuickChainHoldError::StateInvariantViolation)
    }

    /// Iterate over open holds in deterministic `hold_id` order.
    pub fn ordered_active_holds(&self) -> impl Iterator<Item = (&str, &QuickChainOpenHoldRecord)> {
        self.active_holds
            .iter()
            .map(|(hold_id, hold)| (hold_id.as_str(), hold))
    }

    /// Iterate over terminal evidence in deterministic `hold_id` order.
    pub fn ordered_terminal_holds(
        &self,
    ) -> impl Iterator<Item = (&str, &QuickChainTerminalHoldRecord)> {
        self.terminal_holds
            .iter()
            .map(|(hold_id, hold)| (hold_id.as_str(), hold))
    }

    pub(crate) fn insert_active(
        &mut self,
        hold: QuickChainOpenHoldRecord,
    ) -> Result<(), QuickChainHoldError> {
        if self.is_hold_id_used(hold.hold_id()) {
            return Err(QuickChainHoldError::HoldIdAlreadyUsed {
                hold_id: hold.hold_id().to_string(),
            });
        }

        let current_held = self.held_minor(hold.account_id());
        let next_held = current_held
            .checked_add(hold.amount_minor())
            .ok_or_else(|| QuickChainHoldError::HeldBalanceOverflow {
                account_id: hold.account_id().to_string(),
            })?;

        self.held_by_account
            .insert(hold.account_id().to_string(), next_held);
        self.active_holds.insert(hold.hold_id().to_string(), hold);

        self.validate_invariants()
    }

    pub(crate) fn remove_active(
        &mut self,
        hold_id: &str,
    ) -> Result<QuickChainOpenHoldRecord, QuickChainHoldError> {
        let hold =
            self.active_holds
                .remove(hold_id)
                .ok_or_else(|| QuickChainHoldError::HoldNotFound {
                    hold_id: hold_id.to_string(),
                })?;

        let current_held = self.held_minor(hold.account_id());
        let next_held = current_held
            .checked_sub(hold.amount_minor())
            .ok_or(QuickChainHoldError::StateInvariantViolation)?;

        if next_held == 0 {
            self.held_by_account.remove(hold.account_id());
        } else {
            self.held_by_account
                .insert(hold.account_id().to_string(), next_held);
        }

        self.validate_invariants()?;
        Ok(hold)
    }

    pub(crate) fn insert_terminal(
        &mut self,
        terminal: QuickChainTerminalHoldRecord,
    ) -> Result<(), QuickChainHoldError> {
        if self.active_holds.contains_key(terminal.hold_id())
            || self.terminal_holds.contains_key(terminal.hold_id())
        {
            return Err(QuickChainHoldError::HoldIdAlreadyUsed {
                hold_id: terminal.hold_id().to_string(),
            });
        }

        self.terminal_holds
            .insert(terminal.hold_id().to_string(), terminal);

        self.validate_invariants()
    }

    pub(crate) fn validate_invariants(&self) -> Result<(), QuickChainHoldError> {
        if self
            .active_holds
            .keys()
            .any(|hold_id| self.terminal_holds.contains_key(hold_id))
        {
            return Err(QuickChainHoldError::StateInvariantViolation);
        }

        let mut derived_held_by_account = BTreeMap::<String, u128>::new();

        for (map_hold_id, hold) in &self.active_holds {
            if map_hold_id != hold.hold_id()
                || hold.amount_minor() == 0
                || hold.expires_at_epoch() <= hold.created_at_epoch()
            {
                return Err(QuickChainHoldError::StateInvariantViolation);
            }

            let account_total = derived_held_by_account
                .entry(hold.account_id().to_string())
                .or_insert(0);

            *account_total = account_total
                .checked_add(hold.amount_minor())
                .ok_or_else(|| QuickChainHoldError::HeldBalanceOverflow {
                    account_id: hold.account_id().to_string(),
                })?;
        }

        if derived_held_by_account != self.held_by_account {
            return Err(QuickChainHoldError::StateInvariantViolation);
        }

        for (map_hold_id, terminal) in &self.terminal_holds {
            if map_hold_id != terminal.hold_id()
                || terminal.original_amount_minor() == 0
                || terminal.terminal_amount_minor() == 0
                || terminal.terminal_amount_minor() > terminal.original_amount_minor()
                || terminal.expires_at_epoch() <= terminal.created_at_epoch()
                || terminal.terminal_at_epoch() < terminal.created_at_epoch()
                || terminal.terminal_operation_id().is_empty()
                || terminal.terminal_receipt_txid().is_empty()
            {
                return Err(QuickChainHoldError::StateInvariantViolation);
            }

            match terminal.status() {
                QuickChainHoldTerminalStatus::Captured => {
                    let terminal_counterparty = terminal
                        .terminal_counterparty_account_id()
                        .ok_or(QuickChainHoldError::StateInvariantViolation)?;

                    if let Some(opened_counterparty) = terminal.opened_counterparty_account_id() {
                        if opened_counterparty != terminal_counterparty {
                            return Err(QuickChainHoldError::StateInvariantViolation);
                        }
                    }
                }

                QuickChainHoldTerminalStatus::Released | QuickChainHoldTerminalStatus::Expired => {
                    if terminal.terminal_counterparty_account_id().is_some()
                        || terminal.terminal_amount_minor() != terminal.original_amount_minor()
                    {
                        return Err(QuickChainHoldError::StateInvariantViolation);
                    }
                }
            }
        }

        Ok(())
    }
}
