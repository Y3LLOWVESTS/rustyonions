//! RO:WHAT — Phase 4 Round 2 replayable disputed-bond simulation for ron-ledger.
//! RO:WHY — ECON/GOV: model challenge/freeze/appeal flow before live slash enforcement exists.
//! RO:INTERACTS — ron-proto bond dispute DTOs and Phase 4 bond accounting model tests.
//! RO:INVARIANTS — pure deterministic replay; COW outputs; no wallet mutation; no live irreversible slash; no staking/liquidity/bridge.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — simulation grants no spend, slash, finality, bridge, settlement, staking, or paid-unlock authority.
//! RO:TEST — tests/quickchain_phase4_bond_dispute_simulation.rs.

use ron_proto::quickchain::{
    QuickChainBondDisputeEventKindV1, QuickChainBondDisputeEventV1,
    QuickChainBondDisputeRejectionCodeV1, QuickChainBondDisputeStatusV1, QuickChainBondDisputeV1,
};
use thiserror::Error;

/// Deterministic rejection taxonomy for disputed-bond simulation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum QuickChainBondDisputeSimulationError {
    /// Initial or current dispute state failed DTO validation.
    #[error("invalid bond dispute: {0}")]
    InvalidDispute(String),

    /// Event failed DTO validation.
    #[error("invalid bond dispute event: {0}")]
    InvalidDisputeEvent(String),

    /// Event does not bind to the current dispute.
    #[error("bond dispute event binding mismatch")]
    BindingMismatch,

    /// Event sequence was not exactly current.last_dispute_sequence + 1.
    #[error("bond dispute event sequence mismatch: expected={expected} actual={actual}")]
    SequenceMismatch {
        /// Expected next sequence.
        expected: u64,
        /// Actual event sequence.
        actual: u64,
    },

    /// Current dispute is already terminal.
    #[error("bond dispute already terminal")]
    DisputeAlreadyTerminal,

    /// Event occurred outside the challenge window.
    #[error("bond dispute challenge window closed")]
    ChallengeWindowClosed,

    /// Event occurred outside the appeal window.
    #[error("bond dispute appeal window closed")]
    AppealWindowClosed,

    /// Event amount exceeds the disputed amount.
    #[error("bond dispute amount exceeds disputed bond")]
    AmountExceedsDisputedBond,

    /// Transition is not valid from the current status.
    #[error("invalid bond dispute transition: {0}")]
    InvalidTransition(&'static str),

    /// Checked arithmetic overflowed.
    #[error("bond dispute simulation arithmetic overflow")]
    ArithmeticOverflow,
}

/// Evaluate one disputed-bond event against a current simulation state.
///
/// This is pure and copy-on-write: the returned state is a new DTO. The current
/// state is never mutated, and no wallet/ledger balance is changed.
pub fn evaluate_bond_dispute_event_simulation(
    current: &QuickChainBondDisputeV1,
    event: &QuickChainBondDisputeEventV1,
) -> Result<QuickChainBondDisputeV1, QuickChainBondDisputeSimulationError> {
    current
        .validate()
        .map_err(|error| QuickChainBondDisputeSimulationError::InvalidDispute(error.to_string()))?;
    event.validate().map_err(|error| {
        QuickChainBondDisputeSimulationError::InvalidDisputeEvent(error.to_string())
    })?;

    ensure_event_binds_current(current, event)?;

    if current.is_terminal() {
        return Err(QuickChainBondDisputeSimulationError::DisputeAlreadyTerminal);
    }

    let expected = current
        .last_dispute_sequence
        .checked_add(1)
        .ok_or(QuickChainBondDisputeSimulationError::ArithmeticOverflow)?;

    if event.event_sequence != expected {
        return Err(QuickChainBondDisputeSimulationError::SequenceMismatch {
            expected,
            actual: event.event_sequence,
        });
    }

    let mut next = current.clone();
    next.epoch_id = event.epoch_id.clone();
    next.last_dispute_sequence = event.event_sequence;

    match event.event_kind {
        QuickChainBondDisputeEventKindV1::FreezePendingAppeal => {
            if current.status != QuickChainBondDisputeStatusV1::ChallengeOpen {
                return Err(QuickChainBondDisputeSimulationError::InvalidTransition(
                    "freeze requires challenge_open state",
                ));
            }

            if !current
                .challenge_window
                .contains_epoch(event.occurred_epoch)
            {
                return Err(QuickChainBondDisputeSimulationError::ChallengeWindowClosed);
            }

            let amount = event_amount(event)?;
            let disputed = parse_minor(&current.disputed_amount_minor)?;

            if amount > disputed {
                return Err(QuickChainBondDisputeSimulationError::AmountExceedsDisputedBond);
            }

            next.status = QuickChainBondDisputeStatusV1::FrozenPendingAppeal;
            next.frozen_amount_minor = amount.to_string();
            next.appeal_window = event.appeal_window.clone();
        }
        QuickChainBondDisputeEventKindV1::SubmitAppeal => {
            if current.status != QuickChainBondDisputeStatusV1::FrozenPendingAppeal {
                return Err(QuickChainBondDisputeSimulationError::InvalidTransition(
                    "appeal requires frozen_pending_appeal state",
                ));
            }

            let appeal_window = current.appeal_window.as_ref().ok_or(
                QuickChainBondDisputeSimulationError::InvalidTransition("appeal window required"),
            )?;

            if !appeal_window.contains_epoch(event.occurred_epoch) {
                return Err(QuickChainBondDisputeSimulationError::AppealWindowClosed);
            }

            next.status = QuickChainBondDisputeStatusV1::AppealOpen;
        }
        QuickChainBondDisputeEventKindV1::ResolveNoSlash => {
            next.status = QuickChainBondDisputeStatusV1::ResolvedNoSlash;
            next.frozen_amount_minor = "0".to_owned();
        }
        QuickChainBondDisputeEventKindV1::RejectIrreversibleSlash => {
            if event.rejection_code
                != Some(QuickChainBondDisputeRejectionCodeV1::OneStepIrreversibleSlashForbidden)
            {
                return Err(QuickChainBondDisputeSimulationError::InvalidTransition(
                    "irreversible slash rejection must carry forbidden code",
                ));
            }

            next.status = QuickChainBondDisputeStatusV1::ResolvedSlashRejected;
            next.frozen_amount_minor = "0".to_owned();
        }
        _ => {
            return Err(QuickChainBondDisputeSimulationError::InvalidTransition(
                "unsupported dispute event kind",
            ));
        }
    }

    next.validate()
        .map_err(|error| QuickChainBondDisputeSimulationError::InvalidDispute(error.to_string()))?;

    Ok(next)
}

/// Replay a deterministic sequence of disputed-bond events.
pub fn replay_bond_dispute_simulation(
    initial: &QuickChainBondDisputeV1,
    events: &[QuickChainBondDisputeEventV1],
) -> Result<QuickChainBondDisputeV1, QuickChainBondDisputeSimulationError> {
    initial
        .validate()
        .map_err(|error| QuickChainBondDisputeSimulationError::InvalidDispute(error.to_string()))?;

    let mut state = initial.clone();

    for event in events {
        state = evaluate_bond_dispute_event_simulation(&state, event)?;
    }

    Ok(state)
}

fn ensure_event_binds_current(
    current: &QuickChainBondDisputeV1,
    event: &QuickChainBondDisputeEventV1,
) -> Result<(), QuickChainBondDisputeSimulationError> {
    if current.chain_id != event.chain_id
        || current.dispute_id != event.dispute_id
        || current.bond_account_id != event.bond_account_id
        || current.validator_id != event.validator_id
    {
        return Err(QuickChainBondDisputeSimulationError::BindingMismatch);
    }

    Ok(())
}

fn event_amount(
    event: &QuickChainBondDisputeEventV1,
) -> Result<u128, QuickChainBondDisputeSimulationError> {
    let value = event.amount_minor.as_deref().ok_or_else(|| {
        QuickChainBondDisputeSimulationError::InvalidDisputeEvent("amount is required".to_owned())
    })?;

    parse_minor(value)
}

fn parse_minor(value: &str) -> Result<u128, QuickChainBondDisputeSimulationError> {
    value
        .parse::<u128>()
        .map_err(|error| QuickChainBondDisputeSimulationError::InvalidDispute(error.to_string()))
}
