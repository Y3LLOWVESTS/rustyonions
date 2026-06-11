//! RO:WHAT — QuickChain Phase 0 replay/idempotency scenario DTOs and semantic validation.
//! RO:WHY — ECON/RES: retries, idempotency conflicts, and duplicate operations must be explicit before transition or root code exists.
//! RO:INTERACTS — operation intents, backend receipt references, future ron-ledger replay, QC-0A locked-byte vectors.
//! RO:INVARIANTS — DTO-only; one accepted commit maximum; account_sequence is ledger-assigned; no mutation, persistence, hashing, or roots.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — idempotency keys are retry metadata, not authority; scenario acceptance does not create economic truth.
//! RO:TEST — tests/quickchain_replay_idempotency.rs and replay_idempotency_locked_bytes_v1.json.

use serde::{Deserialize, Serialize};

use super::{
    operation::{QuickChainOperationClassV1, QuickChainOperationIntentV1},
    validate_chain_id, validate_ref, validate_schema, validate_version, QuickChainResult,
    QuickChainValidationError,
};

/// Schema tag for replay/idempotency scenario DTOs.
pub const QUICKCHAIN_REPLAY_SCENARIO_SCHEMA: &str = "quickchain.replay-scenario.v1";

/// Replay/idempotency scenario classes frozen during Phase 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainReplayScenarioKindV1 {
    /// The same accepted intent is retried with the same idempotency key.
    IdenticalIdempotentRetry,

    /// The same idempotency key is reused for a different intent.
    ConflictingIdempotencyReuse,

    /// An already accepted durable operation id is submitted again.
    DuplicateOperationCommit,

    /// An accepted hold capture is retried and must not capture twice.
    RetryHoldCapture,

    /// An accepted hold release is retried and must not alter state twice.
    RetryHoldRelease,

    /// A client attempts to assign an account sequence that only the ledger may assign.
    LedgerAssignedAccountSequence,
}

/// Expected replay/idempotency disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainReplayOutcomeV1 {
    /// Return/reference the original backend-derived receipt; create no new commit.
    ReturnOriginalReceipt,

    /// Reject reuse of an idempotency key for a different intent.
    RejectIdempotencyConflict,

    /// Reject a second commit attempt for the same durable operation id.
    RejectDuplicateOperationCommit,

    /// Reject a client-supplied account sequence before acceptance.
    RejectClientAssignedAccountSequence,
}

/// Authority responsible for assigning account sequence values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainAccountSequenceSourceV1 {
    /// The wallet/ledger acceptance path assigns the sequence.
    LedgerAssigned,
}

/// Phase 0 replay/idempotency scenario.
///
/// This DTO records expected replay behavior only. It does not execute an
/// operation, look up a receipt, mutate balances, persist dedupe state, or
/// determine whether a backend receipt is authentic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainReplayScenarioV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub scenario_id: String,
    pub scenario_kind: QuickChainReplayScenarioKindV1,

    /// Previously accepted intent, when the scenario begins after one commit.
    #[serde(default)]
    pub original_intent: Option<QuickChainOperationIntentV1>,

    /// Intent presented by the retry or duplicate attempt.
    pub submitted_intent: QuickChainOperationIntentV1,

    /// Original backend receipt reference returned for accepted replay cases.
    #[serde(default)]
    pub original_receipt_txid: Option<String>,

    /// Client-proposed sequence captured only to prove it must be rejected.
    #[serde(default)]
    pub attempted_client_account_sequence: Option<u64>,

    pub expected_outcome: QuickChainReplayOutcomeV1,

    /// Total durable economic commits after evaluating the scenario.
    pub expected_economic_commit_count: u16,

    /// Total durable state transitions after evaluating the scenario.
    pub expected_state_transition_count: u16,

    pub account_sequence_source: QuickChainAccountSequenceSourceV1,
}

impl QuickChainReplayScenarioV1 {
    /// Validate the Phase 0 replay/idempotency scenario contract.
    ///
    /// Validation is structural and relational only. It performs no lookup,
    /// deduplication, receipt verification, wallet mutation, or ledger replay.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainReplayScenarioV1.schema",
            &self.schema,
            QUICKCHAIN_REPLAY_SCENARIO_SCHEMA,
        )?;
        validate_version("QuickChainReplayScenarioV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_ref("scenario_id", &self.scenario_id)?;

        self.submitted_intent.validate()?;
        self.validate_intent_chain("submitted_intent.chain_id", &self.submitted_intent)?;

        if let Some(original_intent) = &self.original_intent {
            original_intent.validate()?;
            self.validate_intent_chain("original_intent.chain_id", original_intent)?;
        }

        if let Some(txid) = &self.original_receipt_txid {
            validate_ref("original_receipt_txid", txid)?;
        }

        if self.attempted_client_account_sequence == Some(0) {
            return Err(QuickChainValidationError::InvalidField {
                field: "attempted_client_account_sequence",
                reason: "must be greater than zero when present",
            });
        }

        match self.account_sequence_source {
            QuickChainAccountSequenceSourceV1::LedgerAssigned => {}
        }

        match self.scenario_kind {
            QuickChainReplayScenarioKindV1::IdenticalIdempotentRetry => {
                self.validate_identical_retry(None)
            }
            QuickChainReplayScenarioKindV1::ConflictingIdempotencyReuse => {
                self.validate_conflicting_idempotency_reuse()
            }
            QuickChainReplayScenarioKindV1::DuplicateOperationCommit => {
                self.validate_duplicate_operation_commit()
            }
            QuickChainReplayScenarioKindV1::RetryHoldCapture => {
                self.validate_identical_retry(Some(QuickChainOperationClassV1::HoldCapture))
            }
            QuickChainReplayScenarioKindV1::RetryHoldRelease => {
                self.validate_identical_retry(Some(QuickChainOperationClassV1::HoldRelease))
            }
            QuickChainReplayScenarioKindV1::LedgerAssignedAccountSequence => {
                self.validate_ledger_assigned_account_sequence()
            }
        }
    }

    fn validate_intent_chain(
        &self,
        field: &'static str,
        intent: &QuickChainOperationIntentV1,
    ) -> QuickChainResult<()> {
        if intent.chain_id == self.chain_id {
            return Ok(());
        }

        Err(QuickChainValidationError::InvalidField {
            field,
            reason: "must match replay scenario chain_id",
        })
    }

    fn validate_identical_retry(
        &self,
        required_class: Option<QuickChainOperationClassV1>,
    ) -> QuickChainResult<()> {
        let original = self.require_original_intent()?;
        self.require_original_receipt()?;
        self.forbid_attempted_client_sequence()?;

        if original != &self.submitted_intent {
            return Err(QuickChainValidationError::InvalidField {
                field: "submitted_intent",
                reason: "must exactly match original_intent for an identical retry",
            });
        }

        if let Some(required_class) = required_class {
            if self.submitted_intent.op_class != required_class {
                return Err(QuickChainValidationError::InvalidField {
                    field: "submitted_intent.op_class",
                    reason: "does not match the replay scenario operation class",
                });
            }
        }

        self.require_outcome(QuickChainReplayOutcomeV1::ReturnOriginalReceipt)?;
        self.require_counts(1, 1)
    }

    fn validate_conflicting_idempotency_reuse(&self) -> QuickChainResult<()> {
        let original = self.require_original_intent()?;
        self.require_original_receipt()?;
        self.forbid_attempted_client_sequence()?;

        if original.idempotency_key != self.submitted_intent.idempotency_key {
            return Err(QuickChainValidationError::InvalidField {
                field: "submitted_intent.idempotency_key",
                reason: "must equal original idempotency_key for conflict-reuse scenarios",
            });
        }

        if original == &self.submitted_intent {
            return Err(QuickChainValidationError::InvalidField {
                field: "submitted_intent",
                reason: "must differ from original_intent for conflicting idempotency reuse",
            });
        }

        self.require_outcome(QuickChainReplayOutcomeV1::RejectIdempotencyConflict)?;
        self.require_counts(1, 1)
    }

    fn validate_duplicate_operation_commit(&self) -> QuickChainResult<()> {
        let original = self.require_original_intent()?;
        self.require_original_receipt()?;
        self.forbid_attempted_client_sequence()?;

        if original.operation_id != self.submitted_intent.operation_id {
            return Err(QuickChainValidationError::InvalidField {
                field: "submitted_intent.operation_id",
                reason: "must equal original operation_id for duplicate-commit scenarios",
            });
        }

        if original.idempotency_key == self.submitted_intent.idempotency_key {
            return Err(QuickChainValidationError::InvalidField {
                field: "submitted_intent.idempotency_key",
                reason: "must differ so duplicate operation identity is tested independently",
            });
        }

        if original == &self.submitted_intent {
            return Err(QuickChainValidationError::InvalidField {
                field: "submitted_intent",
                reason: "must be a distinct submission for duplicate-commit scenarios",
            });
        }

        self.require_outcome(QuickChainReplayOutcomeV1::RejectDuplicateOperationCommit)?;
        self.require_counts(1, 1)
    }

    fn validate_ledger_assigned_account_sequence(&self) -> QuickChainResult<()> {
        if self.original_intent.is_some() {
            return Err(QuickChainValidationError::InvalidField {
                field: "original_intent",
                reason: "must be absent for a pre-acceptance sequence rejection",
            });
        }

        if self.original_receipt_txid.is_some() {
            return Err(QuickChainValidationError::InvalidField {
                field: "original_receipt_txid",
                reason: "must be absent because no operation was accepted",
            });
        }

        if self.attempted_client_account_sequence.is_none() {
            return Err(QuickChainValidationError::InvalidField {
                field: "attempted_client_account_sequence",
                reason: "required for the client-assigned sequence rejection scenario",
            });
        }

        self.require_outcome(QuickChainReplayOutcomeV1::RejectClientAssignedAccountSequence)?;
        self.require_counts(0, 0)
    }

    fn require_original_intent(&self) -> QuickChainResult<&QuickChainOperationIntentV1> {
        self.original_intent
            .as_ref()
            .ok_or(QuickChainValidationError::InvalidField {
                field: "original_intent",
                reason: "required after an accepted original operation",
            })
    }

    fn require_original_receipt(&self) -> QuickChainResult<()> {
        if self.original_receipt_txid.is_some() {
            return Ok(());
        }

        Err(QuickChainValidationError::InvalidField {
            field: "original_receipt_txid",
            reason: "required after an accepted original operation",
        })
    }

    fn forbid_attempted_client_sequence(&self) -> QuickChainResult<()> {
        if self.attempted_client_account_sequence.is_none() {
            return Ok(());
        }

        Err(QuickChainValidationError::InvalidField {
            field: "attempted_client_account_sequence",
            reason: "must be absent outside the ledger-assigned sequence scenario",
        })
    }

    fn require_outcome(&self, expected: QuickChainReplayOutcomeV1) -> QuickChainResult<()> {
        if self.expected_outcome == expected {
            return Ok(());
        }

        Err(QuickChainValidationError::InvalidField {
            field: "expected_outcome",
            reason: "does not match replay scenario semantics",
        })
    }

    fn require_counts(
        &self,
        expected_economic_commit_count: u16,
        expected_state_transition_count: u16,
    ) -> QuickChainResult<()> {
        if self.expected_economic_commit_count != expected_economic_commit_count {
            return Err(QuickChainValidationError::InvalidField {
                field: "expected_economic_commit_count",
                reason: "does not match replay scenario semantics",
            });
        }

        if self.expected_state_transition_count != expected_state_transition_count {
            return Err(QuickChainValidationError::InvalidField {
                field: "expected_state_transition_count",
                reason: "does not match replay scenario semantics",
            });
        }

        Ok(())
    }
}
