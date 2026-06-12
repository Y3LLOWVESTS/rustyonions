//! RO:WHAT — Atomic copy-on-write composition of QuickChain balances, holds, replay identity, and ledger-owned sequences.
//! RO:WHY — ECON/RES: arithmetic, reservations, lifecycle evidence, and identity indexes must commit together or not at all.
//! RO:INTERACTS — balance_state, hold_state, hold_transition, replay_index, execution_error, and ron-proto intents.
//! RO:INVARIANTS — classify retry first; reserved ROC is not spendable; explicit epochs only; no IO, clocks, roots, or partial commits.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — receipt references and supply decisions are trusted boundary inputs, not capabilities by themselves.
//! RO:TEST — quickchain_atomic_execution.rs and quickchain_hold_atomic_execution.rs.

use ron_proto::quickchain::{QuickChainOperationClassV1, QuickChainOperationIntentV1};

use super::{
    balance_state::QuickChainBalanceState,
    error::QuickChainReplayError,
    execution_error::QuickChainExecutionError,
    hold_error::QuickChainHoldError,
    hold_state::{QuickChainHoldState, QuickChainOpenHoldRecord, QuickChainTerminalHoldRecord},
    hold_transition::{
        apply_hold_operation, validate_combined_state, QuickChainHoldEpochInput,
        QuickChainHoldTransition,
    },
    replay_index::QuickChainReplayIndex,
    transition::{QuickChainBalanceTransition, QuickChainSupplyDecision},
    transition_error::QuickChainTransitionError,
    types::{QuickChainCommittedOperationRecord, QuickChainSubmissionDecision},
};

/// Whether an atomic execution created a new commit or returned prior evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum QuickChainExecutionDisposition {
    /// A fresh operation changed economic and replay state atomically.
    Committed,

    /// An exact retry returned original committed evidence without mutation.
    Retried,
}

/// Result of one accepted or replayed basic balance operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainBalanceExecutionOutcome {
    disposition: QuickChainExecutionDisposition,
    record: Box<QuickChainCommittedOperationRecord>,
    transition: Option<Box<QuickChainBalanceTransition>>,
}

impl QuickChainBalanceExecutionOutcome {
    /// Return whether this call committed fresh state or replayed prior evidence.
    #[must_use]
    pub const fn disposition(&self) -> QuickChainExecutionDisposition {
        self.disposition
    }

    /// Return the original committed operation evidence.
    #[must_use]
    pub fn record(&self) -> &QuickChainCommittedOperationRecord {
        self.record.as_ref()
    }

    /// Return the economic transition for a fresh commit.
    #[must_use]
    pub fn transition(&self) -> Option<&QuickChainBalanceTransition> {
        self.transition.as_deref()
    }

    /// Return true when this call created a fresh atomic commit.
    #[must_use]
    pub const fn is_committed(&self) -> bool {
        matches!(self.disposition, QuickChainExecutionDisposition::Committed)
    }

    /// Return true when this call returned an earlier committed outcome.
    #[must_use]
    pub const fn is_retry(&self) -> bool {
        matches!(self.disposition, QuickChainExecutionDisposition::Retried)
    }

    fn committed(
        record: QuickChainCommittedOperationRecord,
        transition: QuickChainBalanceTransition,
    ) -> Self {
        Self {
            disposition: QuickChainExecutionDisposition::Committed,
            record: Box::new(record),
            transition: Some(Box::new(transition)),
        }
    }

    fn retried(record: Box<QuickChainCommittedOperationRecord>) -> Self {
        Self {
            disposition: QuickChainExecutionDisposition::Retried,
            record,
            transition: None,
        }
    }
}

/// Result of one accepted or replayed hold lifecycle operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainHoldExecutionOutcome {
    disposition: QuickChainExecutionDisposition,
    record: Box<QuickChainCommittedOperationRecord>,
    transition: Option<Box<QuickChainHoldTransition>>,
}

impl QuickChainHoldExecutionOutcome {
    /// Return whether this call committed fresh state or replayed prior evidence.
    #[must_use]
    pub const fn disposition(&self) -> QuickChainExecutionDisposition {
        self.disposition
    }

    /// Return original committed operation evidence.
    #[must_use]
    pub fn record(&self) -> &QuickChainCommittedOperationRecord {
        self.record.as_ref()
    }

    /// Return the hold transition for a fresh commit.
    #[must_use]
    pub fn transition(&self) -> Option<&QuickChainHoldTransition> {
        self.transition.as_deref()
    }

    /// Return true when this call created a fresh atomic commit.
    #[must_use]
    pub const fn is_committed(&self) -> bool {
        matches!(self.disposition, QuickChainExecutionDisposition::Committed)
    }

    /// Return true when this call returned original evidence.
    #[must_use]
    pub const fn is_retry(&self) -> bool {
        matches!(self.disposition, QuickChainExecutionDisposition::Retried)
    }

    fn committed(
        record: QuickChainCommittedOperationRecord,
        transition: QuickChainHoldTransition,
    ) -> Self {
        Self {
            disposition: QuickChainExecutionDisposition::Committed,
            record: Box::new(record),
            transition: Some(Box::new(transition)),
        }
    }

    fn retried(record: Box<QuickChainCommittedOperationRecord>) -> Self {
        Self {
            disposition: QuickChainExecutionDisposition::Retried,
            record,
            transition: None,
        }
    }
}

/// Pure in-memory atomic QuickChain execution state.
///
/// This state is not durable storage. It contains no hashes, roots,
/// checkpoints, validators, signatures, sockets, or wall-clock behavior.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuickChainAtomicState {
    balances: QuickChainBalanceState,
    holds: QuickChainHoldState,
    replay: QuickChainReplayIndex,
}

impl QuickChainAtomicState {
    /// Create empty atomic preflight state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow deterministic balance and supply state.
    #[must_use]
    pub const fn balance_state(&self) -> &QuickChainBalanceState {
        &self.balances
    }

    /// Borrow deterministic active and terminal hold state.
    #[must_use]
    pub const fn hold_state(&self) -> &QuickChainHoldState {
        &self.holds
    }

    /// Borrow deterministic operation and idempotency state.
    #[must_use]
    pub const fn replay_index(&self) -> &QuickChainReplayIndex {
        &self.replay
    }

    /// Return one account's total ROC balance.
    #[must_use]
    pub fn balance_minor(&self, account_id: &str) -> u128 {
        self.balances.balance_minor(account_id)
    }

    /// Return one account's currently reserved ROC.
    #[must_use]
    pub fn held_minor(&self, account_id: &str) -> u128 {
        self.holds.held_minor(account_id)
    }

    /// Return one account's unreserved, spendable ROC.
    pub fn available_minor(&self, account_id: &str) -> Result<u128, QuickChainHoldError> {
        self.holds
            .available_minor(account_id, self.balances.balance_minor(account_id))
    }

    /// Return an active hold by lifecycle identifier.
    #[must_use]
    pub fn active_hold(&self, hold_id: &str) -> Option<&QuickChainOpenHoldRecord> {
        self.holds.active_hold(hold_id)
    }

    /// Return terminal hold evidence by lifecycle identifier.
    #[must_use]
    pub fn terminal_hold(&self, hold_id: &str) -> Option<&QuickChainTerminalHoldRecord> {
        self.holds.terminal_hold(hold_id)
    }

    /// Return current circulating ROC supply.
    #[must_use]
    pub const fn current_supply_minor(&self) -> u128 {
        self.balances.current_supply_minor()
    }

    /// Return number of committed operations.
    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.replay.operation_count()
    }

    /// Return next primitive ledger sequence.
    #[must_use]
    pub const fn next_ledger_sequence(&self) -> u64 {
        self.replay.next_ledger_sequence()
    }

    /// Return the last ledger-assigned account-state sequence.
    #[must_use]
    pub fn last_account_sequence(&self, account_id: &str) -> u64 {
        self.replay.last_account_sequence(account_id)
    }

    /// Look up committed operation evidence.
    #[must_use]
    pub fn committed_operation(
        &self,
        operation_id: &str,
    ) -> Option<&QuickChainCommittedOperationRecord> {
        self.replay.committed(operation_id)
    }

    /// Execute one issue, transfer, or burn atomically.
    ///
    /// Transfer and burn use available balance when active holds exist.
    pub fn execute_balance_operation(
        &mut self,
        intent: &QuickChainOperationIntentV1,
        supply_decision: QuickChainSupplyDecision,
        trusted_receipt_txid: impl Into<String>,
    ) -> Result<QuickChainBalanceExecutionOutcome, QuickChainExecutionError> {
        ensure_balance_operation_class(intent.op_class)?;

        match self.replay.classify_submission(intent)? {
            QuickChainSubmissionDecision::ReturnOriginal(record) => {
                return Ok(QuickChainBalanceExecutionOutcome::retried(record));
            }
            QuickChainSubmissionDecision::Fresh => {}
        }

        let mut candidate = self.clone();

        candidate.ensure_available_for_balance_operation(intent)?;

        let transition = candidate
            .balances
            .apply_balance_operation(intent, supply_decision)?;

        let account_sequence = candidate.next_account_sequence(&intent.actor_account_id)?;

        let ledger_sequence_start = candidate.replay.next_ledger_sequence();
        let posting_count = balance_posting_count(transition.op_class)?;

        let ledger_sequence_end = ledger_sequence_start
            .checked_add(posting_count - 1)
            .ok_or(QuickChainReplayError::SequenceOverflow)?;

        let record = QuickChainCommittedOperationRecord::new(
            intent.clone(),
            trusted_receipt_txid.into(),
            account_sequence,
            ledger_sequence_start,
            ledger_sequence_end,
        )?;

        candidate.replay.record_committed(record.clone())?;
        candidate.validate_combined_invariants()?;

        *self = candidate;

        Ok(QuickChainBalanceExecutionOutcome::committed(
            record, transition,
        ))
    }

    /// Execute one hold open, capture, release, or expiry atomically.
    ///
    /// Exact retries return original evidence before re-evaluating epoch input
    /// or a newly supplied receipt reference.
    pub fn execute_hold_operation(
        &mut self,
        intent: &QuickChainOperationIntentV1,
        epoch_input: QuickChainHoldEpochInput,
        trusted_receipt_txid: impl Into<String>,
    ) -> Result<QuickChainHoldExecutionOutcome, QuickChainExecutionError> {
        ensure_hold_operation_class(intent.op_class)?;

        match self.replay.classify_submission(intent)? {
            QuickChainSubmissionDecision::ReturnOriginal(record) => {
                return Ok(QuickChainHoldExecutionOutcome::retried(record));
            }
            QuickChainSubmissionDecision::Fresh => {}
        }

        let mut candidate = self.clone();

        let account_sequence = candidate.next_account_sequence(&intent.actor_account_id)?;

        let ledger_sequence_start = candidate.replay.next_ledger_sequence();
        let posting_count = hold_posting_count(intent.op_class)?;

        let ledger_sequence_end = ledger_sequence_start
            .checked_add(posting_count - 1)
            .ok_or(QuickChainReplayError::SequenceOverflow)?;

        let record = QuickChainCommittedOperationRecord::new(
            intent.clone(),
            trusted_receipt_txid.into(),
            account_sequence,
            ledger_sequence_start,
            ledger_sequence_end,
        )?;

        let transition = apply_hold_operation(
            &mut candidate.balances,
            &mut candidate.holds,
            intent,
            &record,
            epoch_input,
        )?;

        candidate.replay.record_committed(record.clone())?;
        candidate.validate_combined_invariants()?;

        *self = candidate;

        Ok(QuickChainHoldExecutionOutcome::committed(
            record, transition,
        ))
    }

    fn next_account_sequence(&self, account_id: &str) -> Result<u64, QuickChainReplayError> {
        self.replay
            .last_account_sequence(account_id)
            .checked_add(1)
            .ok_or(QuickChainReplayError::SequenceOverflow)
    }

    fn ensure_available_for_balance_operation(
        &self,
        intent: &QuickChainOperationIntentV1,
    ) -> Result<(), QuickChainExecutionError> {
        if !matches!(
            intent.op_class,
            QuickChainOperationClassV1::Transfer | QuickChainOperationClassV1::Burn
        ) {
            return Ok(());
        }

        // Preserve the ordinary balance-transition rejection taxonomy when no
        // ROC is reserved. The hold-specific available-balance guard exists only
        // to prevent active reservations from being spent by transfer or burn.
        if self.held_minor(&intent.actor_account_id) == 0 {
            return Ok(());
        }

        let amount_minor = intent
            .amount_minor
            .as_deref()
            .ok_or(QuickChainTransitionError::StateInvariantViolation)?
            .parse::<u128>()
            .map_err(|_| QuickChainTransitionError::StateInvariantViolation)?;

        let available_minor = self.available_minor(&intent.actor_account_id)?;

        if amount_minor > available_minor {
            return Err(QuickChainHoldError::InsufficientAvailableFunds {
                account_id: intent.actor_account_id.clone(),
                available_minor,
                required_minor: amount_minor,
            }
            .into());
        }

        Ok(())
    }

    fn validate_combined_invariants(&self) -> Result<(), QuickChainExecutionError> {
        validate_combined_state(&self.balances, &self.holds)?;
        Ok(())
    }
}

fn ensure_balance_operation_class(
    op_class: QuickChainOperationClassV1,
) -> Result<(), QuickChainExecutionError> {
    match op_class {
        QuickChainOperationClassV1::Issue
        | QuickChainOperationClassV1::Transfer
        | QuickChainOperationClassV1::Burn => Ok(()),

        QuickChainOperationClassV1::HoldOpen
        | QuickChainOperationClassV1::HoldCapture
        | QuickChainOperationClassV1::HoldRelease
        | QuickChainOperationClassV1::HoldExpire => {
            Err(QuickChainTransitionError::UnsupportedOperationClass.into())
        }

        _ => Err(QuickChainTransitionError::UnsupportedOperationClass.into()),
    }
}

fn ensure_hold_operation_class(
    op_class: QuickChainOperationClassV1,
) -> Result<(), QuickChainExecutionError> {
    match op_class {
        QuickChainOperationClassV1::HoldOpen
        | QuickChainOperationClassV1::HoldCapture
        | QuickChainOperationClassV1::HoldRelease
        | QuickChainOperationClassV1::HoldExpire => Ok(()),

        QuickChainOperationClassV1::Issue
        | QuickChainOperationClassV1::Transfer
        | QuickChainOperationClassV1::Burn => {
            Err(QuickChainHoldError::UnsupportedOperationClass.into())
        }

        _ => Err(QuickChainHoldError::UnsupportedOperationClass.into()),
    }
}

fn balance_posting_count(
    op_class: QuickChainOperationClassV1,
) -> Result<u64, QuickChainExecutionError> {
    match op_class {
        QuickChainOperationClassV1::Issue | QuickChainOperationClassV1::Burn => Ok(1),

        QuickChainOperationClassV1::Transfer => Ok(2),

        QuickChainOperationClassV1::HoldOpen
        | QuickChainOperationClassV1::HoldCapture
        | QuickChainOperationClassV1::HoldRelease
        | QuickChainOperationClassV1::HoldExpire => {
            Err(QuickChainTransitionError::UnsupportedOperationClass.into())
        }

        _ => Err(QuickChainTransitionError::UnsupportedOperationClass.into()),
    }
}

fn hold_posting_count(
    op_class: QuickChainOperationClassV1,
) -> Result<u64, QuickChainExecutionError> {
    match op_class {
        QuickChainOperationClassV1::HoldOpen
        | QuickChainOperationClassV1::HoldRelease
        | QuickChainOperationClassV1::HoldExpire => Ok(1),

        QuickChainOperationClassV1::HoldCapture => Ok(2),

        QuickChainOperationClassV1::Issue
        | QuickChainOperationClassV1::Transfer
        | QuickChainOperationClassV1::Burn => {
            Err(QuickChainHoldError::UnsupportedOperationClass.into())
        }

        _ => Err(QuickChainHoldError::UnsupportedOperationClass.into()),
    }
}
