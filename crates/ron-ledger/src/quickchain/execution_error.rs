//! RO:WHAT — Unified deterministic error boundary for atomic QuickChain preflight execution.
//! RO:WHY — ECON/RES: callers need one typed result while replay, arithmetic, and hold logic retain precise rejection taxonomies.
//! RO:INTERACTS — execution_state.rs, error.rs, transition_error.rs, hold_error.rs.
//! RO:INVARIANTS — wrapping an error never changes its source category or exposes secrets.
//! RO:METRICS — future adapters may map nested variants to stable counters.
//! RO:CONFIG — none.
//! RO:SECURITY — no capabilities, secrets, stack traces, or raw service errors.
//! RO:TEST — quickchain_atomic_execution.rs and quickchain_hold_atomic_execution.rs.

use thiserror::Error;

use super::{
    error::QuickChainReplayError, hold_error::QuickChainHoldError,
    transition_error::QuickChainTransitionError,
};

/// Error returned by the atomic QuickChain preflight execution boundary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum QuickChainExecutionError {
    /// Operation identity, idempotency, receipt identity, or sequence rejection.
    #[error(transparent)]
    Replay(#[from] QuickChainReplayError),

    /// Basic balance arithmetic, authorization, or state-invariant rejection.
    #[error(transparent)]
    Transition(#[from] QuickChainTransitionError),

    /// Hold lifecycle, epoch, reservation, or available-balance rejection.
    #[error(transparent)]
    Hold(#[from] QuickChainHoldError),
}
