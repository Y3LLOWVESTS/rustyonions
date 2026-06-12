//! RO:WHAT — Pure ordered index for QuickChain chain identity, operation identity, scoped idempotency, and ledger-assigned account-state sequences.
//! RO:WHY — ECON/RES: one state must belong to one chain while retries recover original evidence and duplicates reject deterministically.
//! RO:INTERACTS — types.rs, error.rs, ron-proto operation intents, accepted replay, and the atomic transition engine.
//! RO:INVARIANTS — one chain_id; BTreeMap ordering; each leaf-relevant account advances once; rejection never mutates.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — chain IDs are public domains; client idempotency keys are scoped hints, never anti-double-spend authority.
//! RO:TEST — quickchain_replay_index.rs, quickchain_accepted_replay.rs, and quickchain_chain_binding.rs.

use std::collections::BTreeMap;

use ron_proto::quickchain::{QuickChainOperationClassV1, QuickChainOperationIntentV1};

use super::{
    error::QuickChainReplayError,
    types::{
        QuickChainCommittedOperationRecord, QuickChainOperationFamily, QuickChainSubmissionDecision,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct IdempotencyScope {
    account_id: String,
    operation_family: QuickChainOperationFamily,
    idempotency_key: String,
}

impl IdempotencyScope {
    fn from_intent(intent: &QuickChainOperationIntentV1) -> Result<Self, QuickChainReplayError> {
        Ok(Self {
            account_id: intent.actor_account_id.clone(),
            operation_family: QuickChainOperationFamily::try_from_class(intent.op_class)?,
            idempotency_key: intent.idempotency_key.clone(),
        })
    }
}

/// Deterministic replay index for already committed QuickChain operations.
///
/// The first successful commit binds this index to one `chain_id`. Every later
/// submission and committed record must target that same chain.
///
/// This type does not execute economic operations. It records chain, identity,
/// idempotency, and sequence evidence only after corresponding economic changes
/// have succeeded in the surrounding atomic state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainReplayIndex {
    chain_id: Option<String>,
    operations: BTreeMap<String, QuickChainCommittedOperationRecord>,
    idempotency: BTreeMap<IdempotencyScope, String>,
    last_account_sequences: BTreeMap<String, u64>,
    next_ledger_sequence: u64,
}

impl Default for QuickChainReplayIndex {
    fn default() -> Self {
        Self {
            chain_id: None,
            operations: BTreeMap::new(),
            idempotency: BTreeMap::new(),
            last_account_sequences: BTreeMap::new(),
            next_ledger_sequence: 1,
        }
    }
}

impl QuickChainReplayIndex {
    /// Build an empty, not-yet-chain-bound replay index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Chain identity established by the first successful commit.
    ///
    /// An empty index returns `None`.
    #[must_use]
    pub fn chain_id(&self) -> Option<&str> {
        self.chain_id.as_deref()
    }

    /// Number of accepted operation records.
    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }

    /// Next primitive ledger sequence required by an appended committed record.
    #[must_use]
    pub const fn next_ledger_sequence(&self) -> u64 {
        self.next_ledger_sequence
    }

    /// Last committed account-state sequence, or zero for an unseen account.
    #[must_use]
    pub fn last_account_sequence(&self, account_id: &str) -> u64 {
        self.last_account_sequences
            .get(account_id)
            .copied()
            .unwrap_or(0)
    }

    /// Look up committed evidence by durable operation ID.
    #[must_use]
    pub fn committed(&self, operation_id: &str) -> Option<&QuickChainCommittedOperationRecord> {
        self.operations.get(operation_id)
    }

    /// Classify a submission without changing replay state.
    ///
    /// Exact DTO equality is intentionally temporary preflight behavior. A later
    /// reviewed batch may bind retries to a locked canonical operation hash
    /// without changing the outcomes established here.
    pub fn classify_submission(
        &self,
        intent: &QuickChainOperationIntentV1,
    ) -> Result<QuickChainSubmissionDecision, QuickChainReplayError> {
        validate_intent(intent)?;
        self.validate_chain_binding(intent)?;

        let scope = IdempotencyScope::from_intent(intent)?;

        if let Some(operation_id) = self.idempotency.get(&scope) {
            let original = self
                .operations
                .get(operation_id)
                .ok_or(QuickChainReplayError::CorruptIdentityIndex)?;

            if original.intent() == intent {
                return Ok(QuickChainSubmissionDecision::ReturnOriginal(Box::new(
                    original.clone(),
                )));
            }

            return Err(QuickChainReplayError::IdempotencyConflict);
        }

        if self.operations.contains_key(&intent.operation_id) {
            return Err(QuickChainReplayError::DuplicateOperationId);
        }

        Ok(QuickChainSubmissionDecision::Fresh)
    }

    /// Append evidence for one operation that already committed economically.
    ///
    /// The first accepted record binds the index to its `chain_id`. Later
    /// records must match that chain and begin at the next primitive ledger
    /// sequence. The record carries the actor's next sequence; every distinct
    /// account whose balance or owned-hold state changes advances exactly once.
    ///
    /// Every validation and next-value calculation occurs before mutation.
    pub fn record_committed(
        &mut self,
        record: QuickChainCommittedOperationRecord,
    ) -> Result<(), QuickChainReplayError> {
        match self.classify_submission(record.intent())? {
            QuickChainSubmissionDecision::Fresh => {}
            QuickChainSubmissionDecision::ReturnOriginal(_) => {
                return Err(QuickChainReplayError::DuplicateOperationId);
            }
        }

        let account_sequence_updates = self.account_sequence_updates(&record)?;

        if record.ledger_sequence_start() != self.next_ledger_sequence {
            return Err(QuickChainReplayError::LedgerSequenceMismatch {
                expected: self.next_ledger_sequence,
                actual: record.ledger_sequence_start(),
            });
        }

        let next_ledger_sequence = record
            .ledger_sequence_end()
            .checked_add(1)
            .ok_or(QuickChainReplayError::SequenceOverflow)?;

        let scope = IdempotencyScope::from_intent(record.intent())?;
        let operation_id = record.intent().operation_id.clone();
        let chain_id = self
            .chain_id
            .clone()
            .unwrap_or_else(|| record.intent().chain_id.clone());

        self.chain_id = Some(chain_id);
        for (account_id, account_sequence) in account_sequence_updates {
            self.last_account_sequences
                .insert(account_id, account_sequence);
        }
        self.next_ledger_sequence = next_ledger_sequence;
        self.idempotency.insert(scope, operation_id.clone());
        self.operations.insert(operation_id, record);

        Ok(())
    }

    fn account_sequence_updates(
        &self,
        record: &QuickChainCommittedOperationRecord,
    ) -> Result<BTreeMap<String, u64>, QuickChainReplayError> {
        let actor_account_id = record.intent().actor_account_id.clone();
        let expected_actor_sequence = self
            .last_account_sequence(&actor_account_id)
            .checked_add(1)
            .ok_or(QuickChainReplayError::SequenceOverflow)?;

        if record.account_sequence() != expected_actor_sequence {
            return Err(QuickChainReplayError::AccountSequenceMismatch {
                account_id: actor_account_id,
                expected: expected_actor_sequence,
                actual: record.account_sequence(),
            });
        }

        let mut updates = BTreeMap::new();
        updates.insert(
            record.intent().actor_account_id.clone(),
            expected_actor_sequence,
        );

        if matches!(
            record.intent().op_class,
            QuickChainOperationClassV1::Transfer | QuickChainOperationClassV1::HoldCapture
        ) {
            let counterparty_account_id = record
                .intent()
                .counterparty_account_id
                .as_ref()
                .ok_or_else(|| {
                    QuickChainReplayError::InvalidIntent(
                        "two-account value movement requires counterparty_account_id".to_string(),
                    )
                })?;

            if counterparty_account_id != &record.intent().actor_account_id {
                let counterparty_sequence = self
                    .last_account_sequence(counterparty_account_id)
                    .checked_add(1)
                    .ok_or(QuickChainReplayError::SequenceOverflow)?;

                updates.insert(counterparty_account_id.clone(), counterparty_sequence);
            }
        }

        Ok(updates)
    }

    fn validate_chain_binding(
        &self,
        intent: &QuickChainOperationIntentV1,
    ) -> Result<(), QuickChainReplayError> {
        let Some(expected) = self.chain_id.as_deref() else {
            return Ok(());
        };

        if expected == intent.chain_id.as_str() {
            return Ok(());
        }

        Err(QuickChainReplayError::ChainIdMismatch {
            expected: expected.to_string(),
            actual: intent.chain_id.clone(),
        })
    }
}

fn validate_intent(intent: &QuickChainOperationIntentV1) -> Result<(), QuickChainReplayError> {
    if intent.account_sequence.is_some() {
        return Err(QuickChainReplayError::ClientAssignedAccountSequence);
    }

    intent
        .validate()
        .map_err(|error| QuickChainReplayError::InvalidIntent(error.to_string()))
}
