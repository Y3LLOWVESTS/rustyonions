//! RO:WHAT — Pure checked QuickChain transitions for hold open, capture, release, and deterministic expiry.
//! RO:WHY — ECON/RES: holds must reserve available value, isolate concurrent lifecycles, and compact terminal state deterministically.
//! RO:INTERACTS — balance_state, hold_state, hold_error, replay evidence, and ron-proto operation intents.
//! RO:INVARIANTS — explicit epochs; checked u128 arithmetic; terminal compaction; no wall clock, IO, roots, or partial commits.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — trusted receipt references and sequence evidence are supplied by the atomic ledger boundary.
//! RO:TEST — tests/quickchain_hold_atomic_execution.rs.

use ron_proto::quickchain::{QuickChainOperationClassV1, QuickChainOperationIntentV1};

use super::{
    balance_state::QuickChainBalanceState,
    hold_error::QuickChainHoldError,
    hold_state::{
        QuickChainHoldState, QuickChainHoldTerminalStatus, QuickChainOpenHoldRecord,
        QuickChainTerminalHoldRecord,
    },
    types::QuickChainCommittedOperationRecord,
};

/// Explicit deterministic epoch input for one hold transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum QuickChainHoldEpochInput {
    /// Epoch range supplied when opening a hold.
    Open {
        /// Epoch in which the hold is created.
        created_at_epoch: u64,

        /// First epoch in which expiry is eligible.
        expires_at_epoch: u64,
    },

    /// Current deterministic epoch supplied to a terminal transition.
    Terminal {
        /// Epoch in which capture, release, or expiry is evaluated.
        current_epoch: u64,
    },
}

/// Kind of hold transition that committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum QuickChainHoldTransitionKind {
    /// A new reservation entered active hold state.
    Opened,

    /// Reserved value was transferred to a beneficiary.
    Captured,

    /// Reserved value returned to ordinary available balance.
    Released,

    /// Reserved value returned after deterministic expiry eligibility.
    Expired,
}

/// Non-receipt summary of one successful hold transition.
///
/// This is not a wire receipt, proof, root, checkpoint, or finality artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainHoldTransition {
    /// Durable operation identity.
    pub operation_id: String,

    /// Hold operation class.
    pub op_class: QuickChainOperationClassV1,

    /// Transition category.
    pub kind: QuickChainHoldTransitionKind,

    /// Durable hold lifecycle identifier.
    pub hold_id: String,

    /// Holder or payer account.
    pub actor_account_id: String,

    /// Opening beneficiary or actual capture beneficiary when present.
    pub counterparty_account_id: Option<String>,

    /// Amount applied by this operation.
    pub amount_minor: u128,

    /// Reservation amount before this transition.
    pub hold_amount_before: u128,

    /// Reservation amount remaining active afterward.
    pub hold_amount_after: u128,

    /// Holder total balance before the transition.
    pub actor_balance_before: u128,

    /// Holder total balance after the transition.
    pub actor_balance_after: u128,

    /// Beneficiary balance before capture.
    pub counterparty_balance_before: Option<u128>,

    /// Beneficiary balance after capture.
    pub counterparty_balance_after: Option<u128>,

    /// Total held value for the holder before the transition.
    pub held_minor_before: u128,

    /// Total held value for the holder after the transition.
    pub held_minor_after: u128,

    /// Available holder value before the transition.
    pub available_minor_before: u128,

    /// Available holder value after the transition.
    pub available_minor_after: u128,

    /// Epoch in which the hold originally opened.
    pub created_at_epoch: u64,

    /// First epoch in which expiry is eligible.
    pub expires_at_epoch: u64,

    /// Epoch in which this transition was applied.
    pub applied_at_epoch: u64,

    /// Terminal lifecycle status, or `None` for a newly opened hold.
    pub terminal_status: Option<QuickChainHoldTerminalStatus>,

    /// Reservation remainder made available by a partial terminal capture.
    pub uncaptured_remainder_minor: u128,
}

pub(crate) fn apply_hold_operation(
    balances: &mut QuickChainBalanceState,
    holds: &mut QuickChainHoldState,
    intent: &QuickChainOperationIntentV1,
    record: &QuickChainCommittedOperationRecord,
    epoch_input: QuickChainHoldEpochInput,
) -> Result<QuickChainHoldTransition, QuickChainHoldError> {
    intent
        .validate()
        .map_err(|error| QuickChainHoldError::InvalidIntent(error.to_string()))?;

    if record.intent() != intent {
        return Err(QuickChainHoldError::StateInvariantViolation);
    }

    let amount_minor = parse_validated_positive_amount(intent)?;
    let hold_id = validated_hold_id(intent)?;

    let transition = match intent.op_class {
        QuickChainOperationClassV1::HoldOpen => {
            let (created_at_epoch, expires_at_epoch) = open_epochs(epoch_input)?;

            apply_open(
                balances,
                holds,
                intent,
                record,
                hold_id,
                amount_minor,
                created_at_epoch,
                expires_at_epoch,
            )
        }

        QuickChainOperationClassV1::HoldCapture => {
            let current_epoch = terminal_epoch(epoch_input)?;

            apply_capture(
                balances,
                holds,
                intent,
                record,
                hold_id,
                amount_minor,
                current_epoch,
            )
        }

        QuickChainOperationClassV1::HoldRelease => {
            let current_epoch = terminal_epoch(epoch_input)?;

            apply_release(
                balances,
                holds,
                intent,
                record,
                hold_id,
                amount_minor,
                current_epoch,
            )
        }

        QuickChainOperationClassV1::HoldExpire => {
            let current_epoch = terminal_epoch(epoch_input)?;

            apply_expire(
                balances,
                holds,
                intent,
                record,
                hold_id,
                amount_minor,
                current_epoch,
            )
        }

        QuickChainOperationClassV1::Issue
        | QuickChainOperationClassV1::Transfer
        | QuickChainOperationClassV1::Burn => Err(QuickChainHoldError::UnsupportedOperationClass),

        _ => Err(QuickChainHoldError::UnsupportedOperationClass),
    }?;

    validate_combined_state(balances, holds)?;
    Ok(transition)
}

#[allow(clippy::too_many_arguments)]
fn apply_open(
    balances: &QuickChainBalanceState,
    holds: &mut QuickChainHoldState,
    intent: &QuickChainOperationIntentV1,
    record: &QuickChainCommittedOperationRecord,
    hold_id: &str,
    amount_minor: u128,
    created_at_epoch: u64,
    expires_at_epoch: u64,
) -> Result<QuickChainHoldTransition, QuickChainHoldError> {
    if holds.is_hold_id_used(hold_id) {
        return Err(QuickChainHoldError::HoldIdAlreadyUsed {
            hold_id: hold_id.to_string(),
        });
    }

    if expires_at_epoch <= created_at_epoch {
        return Err(QuickChainHoldError::InvalidEpochRange {
            created_at_epoch,
            expires_at_epoch,
        });
    }

    let actor_balance = balances.balance_minor(&intent.actor_account_id);
    let held_before = holds.held_minor(&intent.actor_account_id);
    let available_before = holds.available_minor(&intent.actor_account_id, actor_balance)?;

    if amount_minor > available_before {
        return Err(QuickChainHoldError::InsufficientAvailableFunds {
            account_id: intent.actor_account_id.clone(),
            available_minor: available_before,
            required_minor: amount_minor,
        });
    }

    let open = QuickChainOpenHoldRecord::new(
        hold_id.to_string(),
        intent.actor_account_id.clone(),
        intent.counterparty_account_id.clone(),
        amount_minor,
        created_at_epoch,
        expires_at_epoch,
        intent.operation_id.clone(),
        intent.idempotency_key.clone(),
        record.account_sequence(),
        record.ledger_sequence_start(),
        record.ledger_sequence_end(),
    )?;

    holds.insert_active(open)?;

    let held_after = holds.held_minor(&intent.actor_account_id);
    let available_after = holds.available_minor(&intent.actor_account_id, actor_balance)?;

    Ok(QuickChainHoldTransition {
        operation_id: intent.operation_id.clone(),
        op_class: intent.op_class,
        kind: QuickChainHoldTransitionKind::Opened,
        hold_id: hold_id.to_string(),
        actor_account_id: intent.actor_account_id.clone(),
        counterparty_account_id: intent.counterparty_account_id.clone(),
        amount_minor,
        hold_amount_before: 0,
        hold_amount_after: amount_minor,
        actor_balance_before: actor_balance,
        actor_balance_after: actor_balance,
        counterparty_balance_before: None,
        counterparty_balance_after: None,
        held_minor_before: held_before,
        held_minor_after: held_after,
        available_minor_before: available_before,
        available_minor_after: available_after,
        created_at_epoch,
        expires_at_epoch,
        applied_at_epoch: created_at_epoch,
        terminal_status: None,
        uncaptured_remainder_minor: 0,
    })
}

#[allow(clippy::too_many_arguments)]
fn apply_capture(
    balances: &mut QuickChainBalanceState,
    holds: &mut QuickChainHoldState,
    intent: &QuickChainOperationIntentV1,
    record: &QuickChainCommittedOperationRecord,
    hold_id: &str,
    amount_minor: u128,
    current_epoch: u64,
) -> Result<QuickChainHoldTransition, QuickChainHoldError> {
    let open = load_active_hold(holds, hold_id, &intent.actor_account_id)?;
    ensure_before_expiry(&open, current_epoch)?;

    if amount_minor > open.amount_minor() {
        return Err(QuickChainHoldError::RequestedAmountExceedsHeld {
            hold_id: hold_id.to_string(),
            held_minor: open.amount_minor(),
            requested_minor: amount_minor,
        });
    }

    let counterparty_account_id = intent
        .counterparty_account_id
        .as_ref()
        .ok_or(QuickChainHoldError::StateInvariantViolation)?;

    if let Some(expected_counterparty) = open.counterparty_account_id() {
        if expected_counterparty != counterparty_account_id {
            return Err(QuickChainHoldError::HoldCounterpartyMismatch {
                hold_id: hold_id.to_string(),
                expected_counterparty_account_id: expected_counterparty.to_string(),
                actual_counterparty_account_id: counterparty_account_id.clone(),
            });
        }
    }

    let actor_balance_before = balances.balance_minor(&intent.actor_account_id);
    let held_before = holds.held_minor(&intent.actor_account_id);
    let available_before = holds.available_minor(&intent.actor_account_id, actor_balance_before)?;
    let counterparty_balance_before = balances.balance_minor(counterparty_account_id);

    let terminal = QuickChainTerminalHoldRecord::from_open(
        &open,
        Some(counterparty_account_id.clone()),
        amount_minor,
        QuickChainHoldTerminalStatus::Captured,
        current_epoch,
        intent.operation_id.clone(),
        record.receipt_txid().to_string(),
        record.account_sequence(),
        record.ledger_sequence_start(),
        record.ledger_sequence_end(),
    )?;

    let (actor_balance_after, counterparty_balance_after) =
        if counterparty_account_id == &intent.actor_account_id {
            (actor_balance_before, counterparty_balance_before)
        } else {
            let actor_after = actor_balance_before
                .checked_sub(amount_minor)
                .ok_or_else(|| QuickChainHoldError::CaptureBalanceUnderflow {
                    account_id: intent.actor_account_id.clone(),
                    balance_minor: actor_balance_before,
                    captured_minor: amount_minor,
                })?;

            let counterparty_after = counterparty_balance_before
                .checked_add(amount_minor)
                .ok_or_else(|| QuickChainHoldError::CaptureCreditOverflow {
                    account_id: counterparty_account_id.clone(),
                })?;

            balances.set_balance(intent.actor_account_id.clone(), actor_after);
            balances.set_balance(counterparty_account_id.clone(), counterparty_after);

            (actor_after, counterparty_after)
        };

    let removed = holds.remove_active(hold_id)?;
    if removed != open {
        return Err(QuickChainHoldError::StateInvariantViolation);
    }

    holds.insert_terminal(terminal)?;

    let held_after = holds.held_minor(&intent.actor_account_id);
    let available_after = holds.available_minor(&intent.actor_account_id, actor_balance_after)?;

    Ok(QuickChainHoldTransition {
        operation_id: intent.operation_id.clone(),
        op_class: intent.op_class,
        kind: QuickChainHoldTransitionKind::Captured,
        hold_id: hold_id.to_string(),
        actor_account_id: intent.actor_account_id.clone(),
        counterparty_account_id: Some(counterparty_account_id.clone()),
        amount_minor,
        hold_amount_before: open.amount_minor(),
        hold_amount_after: 0,
        actor_balance_before,
        actor_balance_after,
        counterparty_balance_before: Some(counterparty_balance_before),
        counterparty_balance_after: Some(counterparty_balance_after),
        held_minor_before: held_before,
        held_minor_after: held_after,
        available_minor_before: available_before,
        available_minor_after: available_after,
        created_at_epoch: open.created_at_epoch(),
        expires_at_epoch: open.expires_at_epoch(),
        applied_at_epoch: current_epoch,
        terminal_status: Some(QuickChainHoldTerminalStatus::Captured),
        uncaptured_remainder_minor: open.amount_minor() - amount_minor,
    })
}

#[allow(clippy::too_many_arguments)]
fn apply_release(
    balances: &QuickChainBalanceState,
    holds: &mut QuickChainHoldState,
    intent: &QuickChainOperationIntentV1,
    record: &QuickChainCommittedOperationRecord,
    hold_id: &str,
    amount_minor: u128,
    current_epoch: u64,
) -> Result<QuickChainHoldTransition, QuickChainHoldError> {
    let open = load_active_hold(holds, hold_id, &intent.actor_account_id)?;
    ensure_before_expiry(&open, current_epoch)?;

    if amount_minor != open.amount_minor() {
        return Err(QuickChainHoldError::TerminalAmountMismatch {
            hold_id: hold_id.to_string(),
            expected_minor: open.amount_minor(),
            actual_minor: amount_minor,
        });
    }

    finish_without_balance_change(
        balances,
        holds,
        intent,
        record,
        &open,
        amount_minor,
        current_epoch,
        QuickChainHoldTerminalStatus::Released,
        QuickChainHoldTransitionKind::Released,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_expire(
    balances: &QuickChainBalanceState,
    holds: &mut QuickChainHoldState,
    intent: &QuickChainOperationIntentV1,
    record: &QuickChainCommittedOperationRecord,
    hold_id: &str,
    amount_minor: u128,
    current_epoch: u64,
) -> Result<QuickChainHoldTransition, QuickChainHoldError> {
    let open = load_active_hold(holds, hold_id, &intent.actor_account_id)?;

    if current_epoch < open.expires_at_epoch() {
        return Err(QuickChainHoldError::ExpiryNotEligible {
            hold_id: hold_id.to_string(),
            current_epoch,
            expires_at_epoch: open.expires_at_epoch(),
        });
    }

    if amount_minor != open.amount_minor() {
        return Err(QuickChainHoldError::TerminalAmountMismatch {
            hold_id: hold_id.to_string(),
            expected_minor: open.amount_minor(),
            actual_minor: amount_minor,
        });
    }

    finish_without_balance_change(
        balances,
        holds,
        intent,
        record,
        &open,
        amount_minor,
        current_epoch,
        QuickChainHoldTerminalStatus::Expired,
        QuickChainHoldTransitionKind::Expired,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_without_balance_change(
    balances: &QuickChainBalanceState,
    holds: &mut QuickChainHoldState,
    intent: &QuickChainOperationIntentV1,
    record: &QuickChainCommittedOperationRecord,
    open: &QuickChainOpenHoldRecord,
    amount_minor: u128,
    current_epoch: u64,
    terminal_status: QuickChainHoldTerminalStatus,
    transition_kind: QuickChainHoldTransitionKind,
) -> Result<QuickChainHoldTransition, QuickChainHoldError> {
    let actor_balance = balances.balance_minor(&intent.actor_account_id);
    let held_before = holds.held_minor(&intent.actor_account_id);
    let available_before = holds.available_minor(&intent.actor_account_id, actor_balance)?;

    let terminal = QuickChainTerminalHoldRecord::from_open(
        open,
        None,
        amount_minor,
        terminal_status,
        current_epoch,
        intent.operation_id.clone(),
        record.receipt_txid().to_string(),
        record.account_sequence(),
        record.ledger_sequence_start(),
        record.ledger_sequence_end(),
    )?;

    let removed = holds.remove_active(open.hold_id())?;
    if removed != *open {
        return Err(QuickChainHoldError::StateInvariantViolation);
    }

    holds.insert_terminal(terminal)?;

    let held_after = holds.held_minor(&intent.actor_account_id);
    let available_after = holds.available_minor(&intent.actor_account_id, actor_balance)?;

    Ok(QuickChainHoldTransition {
        operation_id: intent.operation_id.clone(),
        op_class: intent.op_class,
        kind: transition_kind,
        hold_id: open.hold_id().to_string(),
        actor_account_id: intent.actor_account_id.clone(),
        counterparty_account_id: None,
        amount_minor,
        hold_amount_before: open.amount_minor(),
        hold_amount_after: 0,
        actor_balance_before: actor_balance,
        actor_balance_after: actor_balance,
        counterparty_balance_before: None,
        counterparty_balance_after: None,
        held_minor_before: held_before,
        held_minor_after: held_after,
        available_minor_before: available_before,
        available_minor_after: available_after,
        created_at_epoch: open.created_at_epoch(),
        expires_at_epoch: open.expires_at_epoch(),
        applied_at_epoch: current_epoch,
        terminal_status: Some(terminal_status),
        uncaptured_remainder_minor: 0,
    })
}

fn load_active_hold(
    holds: &QuickChainHoldState,
    hold_id: &str,
    actor_account_id: &str,
) -> Result<QuickChainOpenHoldRecord, QuickChainHoldError> {
    if holds.terminal_hold(hold_id).is_some() {
        return Err(QuickChainHoldError::HoldAlreadyTerminal {
            hold_id: hold_id.to_string(),
        });
    }

    let open =
        holds
            .active_hold(hold_id)
            .cloned()
            .ok_or_else(|| QuickChainHoldError::HoldNotFound {
                hold_id: hold_id.to_string(),
            })?;

    if open.account_id() != actor_account_id {
        return Err(QuickChainHoldError::HoldAccountMismatch {
            hold_id: hold_id.to_string(),
            expected_account_id: open.account_id().to_string(),
            actual_account_id: actor_account_id.to_string(),
        });
    }

    Ok(open)
}

fn ensure_before_expiry(
    open: &QuickChainOpenHoldRecord,
    current_epoch: u64,
) -> Result<(), QuickChainHoldError> {
    if current_epoch < open.created_at_epoch() {
        return Err(QuickChainHoldError::TerminalEpochBeforeOpen {
            hold_id: open.hold_id().to_string(),
            current_epoch,
            created_at_epoch: open.created_at_epoch(),
        });
    }

    if current_epoch >= open.expires_at_epoch() {
        return Err(QuickChainHoldError::HoldPastExpiry {
            hold_id: open.hold_id().to_string(),
            current_epoch,
            expires_at_epoch: open.expires_at_epoch(),
        });
    }

    Ok(())
}

fn open_epochs(epoch_input: QuickChainHoldEpochInput) -> Result<(u64, u64), QuickChainHoldError> {
    match epoch_input {
        QuickChainHoldEpochInput::Open {
            created_at_epoch,
            expires_at_epoch,
        } => Ok((created_at_epoch, expires_at_epoch)),

        QuickChainHoldEpochInput::Terminal { .. } => Err(QuickChainHoldError::EpochInputMismatch),
    }
}

fn terminal_epoch(epoch_input: QuickChainHoldEpochInput) -> Result<u64, QuickChainHoldError> {
    match epoch_input {
        QuickChainHoldEpochInput::Terminal { current_epoch } => Ok(current_epoch),

        QuickChainHoldEpochInput::Open { .. } => Err(QuickChainHoldError::EpochInputMismatch),
    }
}

fn parse_validated_positive_amount(
    intent: &QuickChainOperationIntentV1,
) -> Result<u128, QuickChainHoldError> {
    let amount_minor = intent
        .amount_minor
        .as_deref()
        .ok_or(QuickChainHoldError::StateInvariantViolation)?;

    let amount = amount_minor
        .parse::<u128>()
        .map_err(|_| QuickChainHoldError::StateInvariantViolation)?;

    if amount == 0 {
        return Err(QuickChainHoldError::ZeroAmount);
    }

    Ok(amount)
}

fn validated_hold_id(intent: &QuickChainOperationIntentV1) -> Result<&str, QuickChainHoldError> {
    intent
        .hold_id
        .as_deref()
        .ok_or(QuickChainHoldError::StateInvariantViolation)
}

pub(crate) fn validate_combined_state(
    balances: &QuickChainBalanceState,
    holds: &QuickChainHoldState,
) -> Result<(), QuickChainHoldError> {
    balances
        .validate_invariants()
        .map_err(|_| QuickChainHoldError::StateInvariantViolation)?;
    holds.validate_invariants()?;

    for (_, hold) in holds.ordered_active_holds() {
        if holds.held_minor(hold.account_id()) > balances.balance_minor(hold.account_id()) {
            return Err(QuickChainHoldError::StateInvariantViolation);
        }
    }

    Ok(())
}
