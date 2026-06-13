//! RO:WHAT — Gated QuickChain replay, checked balances, holds, atomic execution, deterministic snapshots, and pure payload projection for ron-ledger.
//! RO:WHY — ECON/RES: economic arithmetic, reservations, retry identity, ledger-owned sequences, and reviewed immutable projections must agree exactly.
//! RO:INTERACTS — balance_state, hold_state, execution_state, replay_index, state_snapshot, leaf_projection, hash_payload_projection, and ron-proto DTOs.
//! RO:INVARIANTS — checked u128 arithmetic; BTreeMap ordering; explicit commitments/epochs/policy/receipt context; no hashes, roots, IO, clocks, or service mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none; the entire module is disabled unless quickchain-preflight is enabled.
//! RO:SECURITY — idempotency keys, receipt references, supplied roots, projection context, snapshots, and supply decisions grant no authority by themselves.
//! RO:TEST — replay, balance, hold, chain-binding, accepted-replay, snapshot, leaf-projection, operation-hash, and receipt-hash projection suites.

mod accepted_replay;
mod balance_state;
mod error;
mod execution_error;
mod execution_state;
mod hash_payload_projection;
mod hold_error;
mod hold_state;
mod hold_transition;
mod leaf_projection;
mod replay_index;
mod state_snapshot;
mod transition;
mod transition_error;
mod types;

pub use accepted_replay::{QuickChainAcceptedOperation, QuickChainAcceptedReplayBoundary};
pub use balance_state::QuickChainBalanceState;
pub use error::QuickChainReplayError;
pub use execution_error::QuickChainExecutionError;
pub use execution_state::{
    QuickChainAtomicState, QuickChainBalanceExecutionOutcome, QuickChainExecutionDisposition,
    QuickChainHoldExecutionOutcome,
};
pub use hash_payload_projection::{
    project_operation_hash_payload, project_receipt_hash_payload,
    QuickChainHashPayloadProjectionError, QuickChainOperationHashProjectionContext,
    QuickChainReceiptHashProjectionContext,
};
pub use hold_error::QuickChainHoldError;
pub use hold_state::{
    QuickChainHoldState, QuickChainHoldTerminalStatus, QuickChainOpenHoldRecord,
    QuickChainTerminalHoldRecord,
};
pub use hold_transition::{
    QuickChainHoldEpochInput, QuickChainHoldTransition, QuickChainHoldTransitionKind,
};
pub use leaf_projection::{
    QuickChainAccountLeafProjectionContext, QuickChainActiveHoldLeafProjectionContext,
    QuickChainEpochBinding, QuickChainLeafProjectionError,
};
pub use replay_index::QuickChainReplayIndex;
pub use state_snapshot::{
    QuickChainAccountSnapshot, QuickChainActiveHoldSnapshot, QuickChainStateSnapshot,
    QuickChainStateSnapshotError,
};
pub use transition::{QuickChainBalanceTransition, QuickChainSupplyDecision};
pub use transition_error::QuickChainTransitionError;
pub use types::{QuickChainCommittedOperationRecord, QuickChainSubmissionDecision};
