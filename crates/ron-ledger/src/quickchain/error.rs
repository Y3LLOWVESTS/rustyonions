//! RO:WHAT — Deterministic rejection taxonomy for the QuickChain replay index.
//! RO:WHY — ECON/RES: duplicate identities, cross-chain submissions, conflicting retries, and sequence corruption must fail explicitly.
//! RO:INTERACTS — replay_index.rs, types.rs, accepted_replay.rs, and QuickChain replay tests.
//! RO:INVARIANTS — errors are deterministic; no raw capability material, secrets, roots, or stack traces.
//! RO:METRICS — future adapters may map variants to bounded counters.
//! RO:CONFIG — none.
//! RO:SECURITY — messages contain only bounded public identifiers when needed.
//! RO:TEST — quickchain_replay_index.rs, quickchain_accepted_replay.rs, and quickchain_chain_binding.rs.

use thiserror::Error;

/// Deterministic QuickChain replay-index rejection.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum QuickChainReplayError {
    /// The submitted operation intent failed the frozen ron-proto validation contract.
    #[error("invalid operation intent: {0}")]
    InvalidIntent(String),

    /// A client attempted to choose a sequence owned by the ledger.
    #[error("account_sequence must be absent from an operation intent")]
    ClientAssignedAccountSequence,

    /// The linked ron-proto version exposed an operation class this index does not support.
    #[error("unsupported QuickChain operation class")]
    UnsupportedOperationClass,

    /// A receipt reference is empty, overlong, or contains unsupported bytes.
    #[error("invalid receipt transaction reference")]
    InvalidReceiptTxid,

    /// The committed record did not carry a positive ledger-assigned account sequence.
    #[error("committed account_sequence must be greater than zero")]
    InvalidCommittedAccountSequence,

    /// The committed record carried an invalid primitive ledger sequence range.
    #[error("invalid committed ledger sequence range")]
    InvalidLedgerSequenceRange,

    /// The same scoped idempotency key is already bound to a different intent.
    #[error("idempotency key is already bound to a different accepted intent")]
    IdempotencyConflict,

    /// A distinct submission reused an already committed durable operation id.
    #[error("operation_id has already committed")]
    DuplicateOperationId,

    /// Accepted replay history contained the same committed operation more than once.
    #[error("accepted replay history contains duplicate operation: {operation_id}")]
    DuplicateAcceptedOperation {
        /// Durable operation identifier duplicated in accepted history.
        operation_id: String,
    },

    /// Re-execution did not reproduce the supplied committed evidence exactly.
    #[error("accepted replay record does not match deterministic execution: {operation_id}")]
    AcceptedRecordMismatch {
        /// Durable operation identifier whose evidence did not reproduce.
        operation_id: String,
    },

    /// A validated operation targeted a different QuickChain identity.
    #[error("chain_id mismatch: expected={expected}, actual={actual}")]
    ChainIdMismatch {
        /// Chain identity established by the first committed operation.
        expected: String,

        /// Chain identity supplied by the rejected operation.
        actual: String,
    },

    /// Internal operation and idempotency indexes disagree.
    #[error("operation identity index is internally inconsistent")]
    CorruptIdentityIndex,

    /// The committed record skipped or repeated the actor account sequence.
    #[error("account sequence mismatch for {account_id}: expected={expected}, actual={actual}")]
    AccountSequenceMismatch {
        /// Actor account whose sequence did not advance exactly once.
        account_id: String,

        /// Sequence required by deterministic replay.
        expected: u64,

        /// Sequence supplied by the committed record.
        actual: u64,
    },

    /// The committed record did not begin at the next primitive ledger sequence.
    #[error("ledger sequence mismatch: expected start={expected}, actual start={actual}")]
    LedgerSequenceMismatch {
        /// Next sequence required by deterministic replay.
        expected: u64,

        /// Submitted first sequence.
        actual: u64,
    },

    /// A ledger-owned sequence counter could not advance.
    #[error("sequence overflow")]
    SequenceOverflow,
}
