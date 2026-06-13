//! RO:WHAT — Typed accepted-operation history and deterministic QuickChain state reconstruction.
//! RO:WHY — ECON/RES: live acceptance and replay must invoke the same transition code and reproduce identical committed evidence.
//! RO:INTERACTS — execution_state.rs, types.rs, error.rs, hold_transition.rs, and ron-proto operation intents.
//! RO:INVARIANTS — accepted records are replayed in supplied order; duplicates and sequence/evidence/boundary mismatches reject; no roots, IO, clocks, or persistence.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — supply decisions, epoch inputs, receipt refs, chain IDs, and replay boundaries are explicit inputs, not capabilities or authority.
//! RO:TEST — tests/quickchain_accepted_replay.rs and accepted-replay boundary/tamper suites.

use super::{
    error::QuickChainReplayError, execution_error::QuickChainExecutionError,
    execution_state::QuickChainAtomicState, hold_transition::QuickChainHoldEpochInput,
    transition::QuickChainSupplyDecision, types::QuickChainCommittedOperationRecord,
};

/// Caller-supplied boundary for validating an accepted-history reconstruction.
///
/// This is not a root, checkpoint, signature, persistence DTO, consensus field,
/// or settlement artifact. It is a pre-root adapter seam that lets future
/// durable storage restore a replay slice and prove the rebuilt in-memory state
/// ends at the expected chain/operation/ledger boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainAcceptedReplayBoundary {
    operation_count: usize,
    next_ledger_sequence: u64,
    chain_id: Option<String>,
}

impl QuickChainAcceptedReplayBoundary {
    /// Construct an accepted-history replay boundary without a chain assertion.
    ///
    /// Prefer `from_state` or `with_chain_id` when the durable history is
    /// expected to be bound to a concrete chain. This constructor remains useful
    /// for empty-state tests and legacy numeric-boundary adapters.
    #[must_use]
    pub const fn new(operation_count: usize, next_ledger_sequence: u64) -> Self {
        Self {
            operation_count,
            next_ledger_sequence,
            chain_id: None,
        }
    }

    /// Construct an accepted-history replay boundary with a chain assertion.
    #[must_use]
    pub fn with_chain_id(
        operation_count: usize,
        next_ledger_sequence: u64,
        chain_id: impl Into<String>,
    ) -> Self {
        Self {
            operation_count,
            next_ledger_sequence,
            chain_id: Some(chain_id.into()),
        }
    }

    /// Empty-state replay boundary.
    #[must_use]
    pub const fn empty() -> Self {
        Self::new(0, 1)
    }

    /// Capture the replay boundary currently implied by an atomic state.
    #[must_use]
    pub fn from_state(state: &QuickChainAtomicState) -> Self {
        let operation_count = state.operation_count();
        let next_ledger_sequence = state.next_ledger_sequence();

        match state.replay_index().chain_id() {
            Some(chain_id) => Self::with_chain_id(operation_count, next_ledger_sequence, chain_id),
            None => Self::new(operation_count, next_ledger_sequence),
        }
    }

    /// Number of accepted operations expected by this boundary.
    #[must_use]
    pub const fn operation_count(&self) -> usize {
        self.operation_count
    }

    /// Next primitive ledger sequence expected after rebuilding this boundary.
    #[must_use]
    pub const fn next_ledger_sequence(&self) -> u64 {
        self.next_ledger_sequence
    }

    /// Expected chain identity, when this boundary is chain-bound.
    #[must_use]
    pub fn chain_id(&self) -> Option<&str> {
        self.chain_id.as_deref()
    }
}

/// One operation previously accepted by the deterministic ledger boundary.
///
/// This is an internal replay value, not a wire DTO, receipt, checkpoint,
/// signature, proof, or persistence format.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QuickChainAcceptedOperation {
    /// Accepted issue, transfer, or burn operation.
    Balance {
        /// Original committed operation evidence.
        record: Box<QuickChainCommittedOperationRecord>,

        /// Explicit supply-policy result used by the original transition.
        supply_decision: QuickChainSupplyDecision,

        /// Receipt transaction reference supplied back into deterministic execution.
        ///
        /// This is intentionally stored separately from `record.receipt_txid()`
        /// so replay can detect split-field durable-history corruption before
        /// roots, signatures, or persistence formats exist.
        trusted_receipt_txid: String,
    },

    /// Accepted hold open, capture, release, or expiry operation.
    Hold {
        /// Original committed operation evidence.
        record: Box<QuickChainCommittedOperationRecord>,

        /// Explicit deterministic epoch input used by the original transition.
        epoch_input: QuickChainHoldEpochInput,

        /// Receipt transaction reference supplied back into deterministic execution.
        ///
        /// This is intentionally stored separately from `record.receipt_txid()`
        /// so replay can detect split-field durable-history corruption before
        /// roots, signatures, or persistence formats exist.
        trusted_receipt_txid: String,
    },
}

impl QuickChainAcceptedOperation {
    /// Construct accepted balance-operation replay input.
    #[must_use]
    pub fn balance(
        record: QuickChainCommittedOperationRecord,
        supply_decision: QuickChainSupplyDecision,
    ) -> Self {
        let trusted_receipt_txid = record.receipt_txid().to_string();

        Self::Balance {
            record: Box::new(record),
            supply_decision,
            trusted_receipt_txid,
        }
    }

    /// Construct accepted balance-operation replay input from split durable fields.
    ///
    /// This is still not a persistence DTO, signature, proof, or receipt. It is
    /// a preflight adapter boundary for tests and future storage adapters that
    /// restore the committed record and the execution receipt reference from
    /// separate durable fields. If those fields disagree, accepted replay must
    /// reject with `AcceptedRecordMismatch`.
    #[must_use]
    pub fn balance_with_replay_receipt_txid(
        record: QuickChainCommittedOperationRecord,
        supply_decision: QuickChainSupplyDecision,
        trusted_receipt_txid: impl Into<String>,
    ) -> Self {
        Self::Balance {
            record: Box::new(record),
            supply_decision,
            trusted_receipt_txid: trusted_receipt_txid.into(),
        }
    }

    /// Construct accepted hold-operation replay input.
    #[must_use]
    pub fn hold(
        record: QuickChainCommittedOperationRecord,
        epoch_input: QuickChainHoldEpochInput,
    ) -> Self {
        let trusted_receipt_txid = record.receipt_txid().to_string();

        Self::Hold {
            record: Box::new(record),
            epoch_input,
            trusted_receipt_txid,
        }
    }

    /// Construct accepted hold-operation replay input from split durable fields.
    ///
    /// This is still not a persistence DTO, signature, proof, or receipt. It is
    /// a preflight adapter boundary for tests and future storage adapters that
    /// restore the committed record and the execution receipt reference from
    /// separate durable fields. If those fields disagree, accepted replay must
    /// reject with `AcceptedRecordMismatch`.
    #[must_use]
    pub fn hold_with_replay_receipt_txid(
        record: QuickChainCommittedOperationRecord,
        epoch_input: QuickChainHoldEpochInput,
        trusted_receipt_txid: impl Into<String>,
    ) -> Self {
        Self::Hold {
            record: Box::new(record),
            epoch_input,
            trusted_receipt_txid: trusted_receipt_txid.into(),
        }
    }

    /// Return the original committed record expected from replay.
    #[must_use]
    pub fn record(&self) -> &QuickChainCommittedOperationRecord {
        match self {
            Self::Balance { record, .. } | Self::Hold { record, .. } => record.as_ref(),
        }
    }

    /// Return the receipt transaction reference that replay will feed back into
    /// deterministic execution.
    #[must_use]
    pub fn trusted_receipt_txid(&self) -> &str {
        match self {
            Self::Balance {
                trusted_receipt_txid,
                ..
            }
            | Self::Hold {
                trusted_receipt_txid,
                ..
            } => trusted_receipt_txid,
        }
    }
}

impl QuickChainAtomicState {
    /// Return the accepted-history replay boundary implied by this state.
    #[must_use]
    pub fn accepted_replay_boundary(&self) -> QuickChainAcceptedReplayBoundary {
        QuickChainAcceptedReplayBoundary::from_state(self)
    }

    /// Rebuild complete state from ordered accepted-operation history, then
    /// verify the caller-supplied durable-history boundary.
    ///
    /// This catches valid-but-truncated, valid-but-overextended, or wrong-chain
    /// histories before roots, checkpoints, signatures, or persistence formats
    /// are introduced. It does not produce a root or decide finality.
    pub fn rebuild_from_accepted_operations_with_boundary(
        operations: &[QuickChainAcceptedOperation],
        boundary: QuickChainAcceptedReplayBoundary,
    ) -> Result<Self, QuickChainExecutionError> {
        let rebuilt = Self::rebuild_from_accepted_operations(operations)?;

        let actual_operation_count = rebuilt.operation_count();
        if actual_operation_count != boundary.operation_count() {
            return Err(
                QuickChainReplayError::AcceptedHistoryOperationCountMismatch {
                    expected: boundary.operation_count(),
                    actual: actual_operation_count,
                }
                .into(),
            );
        }

        let actual_next_ledger_sequence = rebuilt.next_ledger_sequence();
        if actual_next_ledger_sequence != boundary.next_ledger_sequence() {
            return Err(
                QuickChainReplayError::AcceptedHistoryNextLedgerSequenceMismatch {
                    expected: boundary.next_ledger_sequence(),
                    actual: actual_next_ledger_sequence,
                }
                .into(),
            );
        }

        if let Some(expected_chain_id) = boundary.chain_id() {
            let actual_chain_id = rebuilt.replay_index().chain_id();

            if actual_chain_id != Some(expected_chain_id) {
                return Err(QuickChainReplayError::AcceptedHistoryChainIdMismatch {
                    expected: expected_chain_id.to_string(),
                    actual: actual_chain_id.map(str::to_string),
                }
                .into());
            }
        }

        Ok(rebuilt)
    }

    /// Rebuild complete state from ordered accepted-operation history.
    ///
    /// Every operation is executed through the same atomic methods used by a
    /// fresh live submission. The generated committed record must exactly match
    /// the supplied accepted record, including:
    ///
    /// - validated operation intent
    /// - trusted receipt transaction reference
    /// - actor account sequence
    /// - primitive ledger sequence start
    /// - primitive ledger sequence end
    ///
    /// Any duplicate, arithmetic contradiction, hold-lifecycle contradiction,
    /// epoch mismatch, or committed-evidence mismatch rejects the reconstruction.
    ///
    /// The caller receives no partially rebuilt state when replay fails.
    pub fn rebuild_from_accepted_operations(
        operations: &[QuickChainAcceptedOperation],
    ) -> Result<Self, QuickChainExecutionError> {
        let mut rebuilt = Self::new();

        for operation in operations {
            match operation {
                QuickChainAcceptedOperation::Balance {
                    record,
                    supply_decision,
                    trusted_receipt_txid,
                } => {
                    let outcome = rebuilt.execute_balance_operation(
                        record.intent(),
                        *supply_decision,
                        trusted_receipt_txid.clone(),
                    )?;

                    if outcome.is_retry() {
                        return Err(QuickChainReplayError::DuplicateAcceptedOperation {
                            operation_id: record.intent().operation_id.clone(),
                        }
                        .into());
                    }

                    verify_replayed_record(record, outcome.record())?;
                }

                QuickChainAcceptedOperation::Hold {
                    record,
                    epoch_input,
                    trusted_receipt_txid,
                } => {
                    let outcome = rebuilt.execute_hold_operation(
                        record.intent(),
                        *epoch_input,
                        trusted_receipt_txid.clone(),
                    )?;

                    if outcome.is_retry() {
                        return Err(QuickChainReplayError::DuplicateAcceptedOperation {
                            operation_id: record.intent().operation_id.clone(),
                        }
                        .into());
                    }

                    verify_replayed_record(record, outcome.record())?;
                }
            }
        }

        Ok(rebuilt)
    }
}

fn verify_replayed_record(
    expected: &QuickChainCommittedOperationRecord,
    actual: &QuickChainCommittedOperationRecord,
) -> Result<(), QuickChainExecutionError> {
    if expected != actual {
        return Err(QuickChainReplayError::AcceptedRecordMismatch {
            operation_id: expected.intent().operation_id.clone(),
        }
        .into());
    }

    Ok(())
}
