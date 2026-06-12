//! RO:WHAT — Typed accepted-operation history and deterministic QuickChain state reconstruction.
//! RO:WHY — ECON/RES: live acceptance and replay must invoke the same transition code and reproduce identical committed evidence.
//! RO:INTERACTS — execution_state.rs, types.rs, error.rs, hold_transition.rs, and ron-proto operation intents.
//! RO:INVARIANTS — accepted records are replayed in supplied order; duplicates and sequence/evidence mismatches reject; no roots, IO, clocks, or persistence.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — supply decisions and epoch inputs are explicit replay inputs, not capabilities or proof of authority.
//! RO:TEST — tests/quickchain_accepted_replay.rs.

use super::{
    error::QuickChainReplayError, execution_error::QuickChainExecutionError,
    execution_state::QuickChainAtomicState, hold_transition::QuickChainHoldEpochInput,
    transition::QuickChainSupplyDecision, types::QuickChainCommittedOperationRecord,
};

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
    },

    /// Accepted hold open, capture, release, or expiry operation.
    Hold {
        /// Original committed operation evidence.
        record: Box<QuickChainCommittedOperationRecord>,

        /// Explicit deterministic epoch input used by the original transition.
        epoch_input: QuickChainHoldEpochInput,
    },
}

impl QuickChainAcceptedOperation {
    /// Construct accepted balance-operation replay input.
    #[must_use]
    pub fn balance(
        record: QuickChainCommittedOperationRecord,
        supply_decision: QuickChainSupplyDecision,
    ) -> Self {
        Self::Balance {
            record: Box::new(record),
            supply_decision,
        }
    }

    /// Construct accepted hold-operation replay input.
    #[must_use]
    pub fn hold(
        record: QuickChainCommittedOperationRecord,
        epoch_input: QuickChainHoldEpochInput,
    ) -> Self {
        Self::Hold {
            record: Box::new(record),
            epoch_input,
        }
    }

    /// Return the original committed record expected from replay.
    #[must_use]
    pub fn record(&self) -> &QuickChainCommittedOperationRecord {
        match self {
            Self::Balance { record, .. } | Self::Hold { record, .. } => record.as_ref(),
        }
    }
}

impl QuickChainAtomicState {
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
                } => {
                    let outcome = rebuilt.execute_balance_operation(
                        record.intent(),
                        *supply_decision,
                        record.receipt_txid().to_string(),
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
                } => {
                    let outcome = rebuilt.execute_hold_operation(
                        record.intent(),
                        *epoch_input,
                        record.receipt_txid().to_string(),
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
