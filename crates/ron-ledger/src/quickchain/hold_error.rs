//! RO:WHAT — Deterministic rejection taxonomy for QuickChain hold lifecycle transitions.
//! RO:WHY — ECON/RES: reservation, capture, release, and epoch-expiry failures must reject without partial economic mutation.
//! RO:INTERACTS — hold_state.rs, hold_transition.rs, execution_state.rs, and ron-proto QuickChain hold operation DTOs.
//! RO:INVARIANTS — one hold_id identifies one lifecycle; terminal holds cannot transition or reopen; expiry uses explicit epochs only.
//! RO:METRICS — future adapters may map variants to bounded hold-rejection counters.
//! RO:CONFIG — none.
//! RO:SECURITY — errors contain bounded public identifiers only; they grant no wallet, hold, or spend authority.
//! RO:TEST — tests/quickchain_hold_atomic_execution.rs.

use thiserror::Error;

/// Deterministic error returned by the QuickChain hold lifecycle layer.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum QuickChainHoldError {
    /// The submitted operation intent failed the frozen ron-proto contract.
    #[error("invalid hold operation intent: {0}")]
    InvalidIntent(String),

    /// This hold transition layer does not implement the submitted operation class.
    #[error("operation class is not supported by the hold lifecycle")]
    UnsupportedOperationClass,

    /// The supplied epoch input did not match the operation class.
    #[error("epoch input does not match the hold operation class")]
    EpochInputMismatch,

    /// An executable hold operation attempted to use a zero amount.
    #[error("hold operation amount must be greater than zero")]
    ZeroAmount,

    /// Hold creation used an invalid deterministic epoch range.
    #[error(
        "invalid hold epoch range: created_at_epoch={created_at_epoch}, \
         expires_at_epoch={expires_at_epoch}"
    )]
    InvalidEpochRange {
        /// Epoch in which the hold would be created.
        created_at_epoch: u64,

        /// First epoch in which deterministic expiry becomes eligible.
        expires_at_epoch: u64,
    },

    /// A terminal transition was assigned an epoch earlier than hold creation.
    #[error(
        "terminal epoch precedes hold creation for {hold_id}: \
         current_epoch={current_epoch}, created_at_epoch={created_at_epoch}"
    )]
    TerminalEpochBeforeOpen {
        /// Durable hold lifecycle identifier.
        hold_id: String,

        /// Explicit epoch supplied to the terminal operation.
        current_epoch: u64,

        /// Epoch in which the hold opened.
        created_at_epoch: u64,
    },

    /// Capture or release was attempted after deterministic expiry eligibility.
    #[error(
        "hold has reached its expiry epoch: {hold_id}, \
         current_epoch={current_epoch}, expires_at_epoch={expires_at_epoch}"
    )]
    HoldPastExpiry {
        /// Durable hold lifecycle identifier.
        hold_id: String,

        /// Explicit deterministic epoch supplied to the operation.
        current_epoch: u64,

        /// First epoch reserved for the expiry transition.
        expires_at_epoch: u64,
    },

    /// A hold ID already exists in active or terminal replay history.
    #[error("hold_id has already been used: {hold_id}")]
    HoldIdAlreadyUsed {
        /// Durable hold lifecycle identifier.
        hold_id: String,
    },

    /// The requested hold does not exist in active or terminal history.
    #[error("hold_id was not found: {hold_id}")]
    HoldNotFound {
        /// Durable hold lifecycle identifier.
        hold_id: String,
    },

    /// A distinct operation attempted to transition an already-terminal hold.
    #[error("hold lifecycle is already terminal: {hold_id}")]
    HoldAlreadyTerminal {
        /// Durable hold lifecycle identifier.
        hold_id: String,
    },

    /// The submitted actor account does not own the selected hold.
    #[error(
        "hold account mismatch for {hold_id}: expected={expected_account_id}, \
         actual={actual_account_id}"
    )]
    HoldAccountMismatch {
        /// Durable hold lifecycle identifier.
        hold_id: String,

        /// Account recorded when the hold opened.
        expected_account_id: String,

        /// Account supplied by the terminal operation.
        actual_account_id: String,
    },

    /// The submitted counterparty does not match the beneficiary fixed at opening.
    #[error(
        "hold counterparty mismatch for {hold_id}: \
         expected={expected_counterparty_account_id}, \
         actual={actual_counterparty_account_id}"
    )]
    HoldCounterpartyMismatch {
        /// Durable hold lifecycle identifier.
        hold_id: String,

        /// Counterparty recorded when the hold opened.
        expected_counterparty_account_id: String,

        /// Counterparty supplied by the capture operation.
        actual_counterparty_account_id: String,
    },

    /// Opening or spending would exceed the account's currently available value.
    #[error(
        "insufficient available funds for {account_id}: \
         available={available_minor}, required={required_minor}"
    )]
    InsufficientAvailableFunds {
        /// Account whose available balance could not satisfy the operation.
        account_id: String,

        /// Unreserved account value before the attempted operation.
        available_minor: u128,

        /// Amount the operation attempted to reserve or spend.
        required_minor: u128,
    },

    /// Capture requested more than the selected hold currently reserves.
    #[error(
        "hold operation exceeds reserved amount for {hold_id}: \
         held={held_minor}, requested={requested_minor}"
    )]
    RequestedAmountExceedsHeld {
        /// Durable hold lifecycle identifier.
        hold_id: String,

        /// Amount reserved by the selected open hold.
        held_minor: u128,

        /// Amount requested by the capture operation.
        requested_minor: u128,
    },

    /// Release or expiry did not identify the complete reservation.
    #[error(
        "terminal hold amount mismatch for {hold_id}: \
         expected={expected_minor}, actual={actual_minor}"
    )]
    TerminalAmountMismatch {
        /// Durable hold lifecycle identifier.
        hold_id: String,

        /// Complete amount reserved by the active hold.
        expected_minor: u128,

        /// Amount supplied by release or expiry.
        actual_minor: u128,
    },

    /// A deterministic expiry operation was submitted before its eligible epoch.
    #[error(
        "hold is not eligible for expiry: {hold_id}, \
         current_epoch={current_epoch}, expires_at_epoch={expires_at_epoch}"
    )]
    ExpiryNotEligible {
        /// Durable hold lifecycle identifier.
        hold_id: String,

        /// Explicit deterministic epoch supplied to the transition.
        current_epoch: u64,

        /// First epoch in which expiry is eligible.
        expires_at_epoch: u64,
    },

    /// Adding a reservation would overflow the account's held-value counter.
    #[error("held balance overflow for account {account_id}")]
    HeldBalanceOverflow {
        /// Account whose held-value counter would overflow.
        account_id: String,
    },

    /// Hold capture would underflow the holder's total balance.
    #[error(
        "hold capture would underflow balance for {account_id}: \
         balance={balance_minor}, captured={captured_minor}"
    )]
    CaptureBalanceUnderflow {
        /// Account whose total balance would become negative.
        account_id: String,

        /// Total account balance before capture.
        balance_minor: u128,

        /// Amount the capture attempted to remove.
        captured_minor: u128,
    },

    /// Hold capture would overflow the beneficiary's account balance.
    #[error("hold capture credit would overflow account {account_id}")]
    CaptureCreditOverflow {
        /// Beneficiary account whose resulting balance would overflow.
        account_id: String,
    },

    /// Hold bookkeeping contradicted balance, availability, or lifecycle invariants.
    #[error("QuickChain hold-state invariant violation")]
    StateInvariantViolation,
}
