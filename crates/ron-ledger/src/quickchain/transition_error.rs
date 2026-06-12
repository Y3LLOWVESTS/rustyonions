//! RO:WHAT — Deterministic rejection taxonomy for pure QuickChain balance transitions.
//! RO:WHY — ECON/RES: checked arithmetic and supply authorization failures must reject without partial mutation.
//! RO:INTERACTS — balance_state.rs, transition.rs, ron-proto QuickChain operation intents.
//! RO:INVARIANTS — ron-proto owns DTO-shape rejection; no saturation; no negative balances; no silent supply changes.
//! RO:METRICS — future adapters may map variants to bounded rejection counters.
//! RO:CONFIG — none.
//! RO:SECURITY — approval failures are explicit; this type carries no capabilities or secrets.
//! RO:TEST — tests/quickchain_balance_transition.rs.

use thiserror::Error;

/// Deterministic error returned by the pure QuickChain balance-transition layer.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum QuickChainTransitionError {
    /// The submitted operation intent failed the frozen ron-proto contract.
    ///
    /// This includes missing class-required fields, non-canonical money strings,
    /// and money values outside the supported `u128` range.
    #[error("invalid operation intent: {0}")]
    InvalidIntent(String),

    /// Economic mutation operations must move a positive amount.
    ///
    /// Canonical `"0"` is a valid non-negative money encoding at the DTO layer,
    /// but it is not a valid executable economic mutation.
    #[error("operation amount must be greater than zero")]
    ZeroAmount,

    /// This transition slice does not yet implement the submitted operation class.
    #[error("operation class is not supported by the balance-transition slice")]
    UnsupportedOperationClass,

    /// Supply issue was requested without an approved supply decision.
    #[error("issue operation is not authorized")]
    UnauthorizedIssue,

    /// Supply burn was requested without an approved supply decision.
    #[error("burn operation is not authorized")]
    UnauthorizedBurn,

    /// A debit would make an account balance negative.
    #[error(
        "insufficient funds for {account_id}: available={available_minor}, required={required_minor}"
    )]
    InsufficientFunds {
        /// Account whose balance could not satisfy the debit.
        account_id: String,

        /// Current account balance in ROC minor units.
        available_minor: u128,

        /// Required debit in ROC minor units.
        required_minor: u128,
    },

    /// Crediting an account would exceed `u128::MAX`.
    #[error("balance overflow for account {account_id}")]
    BalanceOverflow {
        /// Account whose resulting balance would overflow.
        account_id: String,
    },

    /// Supply issue accounting would exceed `u128::MAX`.
    #[error("ROC supply overflow")]
    SupplyOverflow,

    /// Supply accounting would become negative or internally inconsistent.
    #[error("ROC supply underflow")]
    SupplyUnderflow,

    /// A value contradicted an invariant already guaranteed by validated ron-proto input.
    #[error("QuickChain balance-state invariant violation")]
    StateInvariantViolation,
}
