//! RO:WHAT — Bounded user-node outbox and transport handoff for invalid-epoch challenges.
//! RO:WHY — Phase 17 requires ordinary user nodes to submit deterministic fraud challenges.
//! RO:INTERACTS — economic_audit challenge construction and ron-proto InvalidEpochChallengeV1.
//! RO:INVARIANTS — duplicate challenge IDs rejected; pending challenges are never silently evicted.
//! RO:SECURITY — transport acknowledgement is not challenge acceptance, finality, punishment, or value authority.
//! RO:TEST — tests/internal_roc_beta_phase17_epoch_replay.rs.

use std::{
    collections::{HashSet, VecDeque},
    fmt::Display,
};

use parking_lot::Mutex;
use ron_proto::InvalidEpochChallengeV1;
use serde::Serialize;
use thiserror::Error;

/// Default number of challenge records retained by one user-node process.
pub const DEFAULT_INVALID_EPOCH_CHALLENGE_OUTBOX_CAPACITY: usize = 128;

/// Maximum number of records returned by one local outbox read.
pub const MAX_INVALID_EPOCH_CHALLENGE_READ_ITEMS: usize = 64;

/// Local lifecycle of one challenge submission record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidEpochChallengeSubmissionStateV1 {
    /// Challenge is waiting for a transport handoff.
    Queued,
    /// One synchronous transport handoff is currently in progress.
    Dispatching,
    /// The configured transport acknowledged receipt of the challenge.
    Submitted,
}

/// Transport acknowledgement for one challenge handoff.
///
/// This acknowledgement proves only that the configured sink received the DTO.
/// It does not mean the challenge was accepted, adjudicated, finalized, or used
/// to change any economic state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvalidEpochChallengeSubmissionAckV1 {
    /// Transport-owned reference for the acknowledged handoff.
    pub submission_ref: String,
    /// Caller/sink-supplied acknowledgement timestamp.
    pub acknowledged_at_ms: u64,
}

/// Bounded local record for one challenge and its transport posture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvalidEpochChallengeSubmissionRecordV1 {
    /// Monotonic process-local outbox sequence.
    pub sequence: u64,
    /// Canonical ron-proto challenge evidence.
    pub challenge: InvalidEpochChallengeV1,
    /// Current local submission state.
    pub submission_state: InvalidEpochChallengeSubmissionStateV1,
    /// Number of transport attempts made for this record.
    pub submission_attempts: u32,
    /// Transport reference after acknowledgement.
    pub submission_ref: Option<String>,
    /// Transport acknowledgement timestamp.
    pub acknowledged_at_ms: Option<u64>,

    /// Challenges remain evidence only.
    pub evidence_only: bool,
    /// Transport acknowledgement never means challenge acceptance.
    pub challenge_accepted: bool,
    /// This record never claims consensus or finality.
    pub finality_claimed: bool,
    /// This record cannot mutate a wallet.
    pub wallet_mutation: bool,
    /// This record cannot mutate a ledger.
    pub ledger_mutation: bool,
}

/// Narrow transport boundary used to hand a canonical challenge to another
/// local or remote component.
///
/// Implementations return only transport acknowledgement. Challenge
/// adjudication and economic consequences are deliberately outside this trait.
pub trait InvalidEpochChallengeSubmissionSink {
    /// Sink-specific transport error.
    type Error: Display;

    /// Hand one canonical challenge to the configured transport.
    fn submit(
        &self,
        challenge: &InvalidEpochChallengeV1,
    ) -> Result<InvalidEpochChallengeSubmissionAckV1, Self::Error>;
}

/// Result of placing one challenge into the bounded outbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidEpochChallengeEnqueueOutcome {
    /// A new challenge was queued.
    Queued(Box<InvalidEpochChallengeSubmissionRecordV1>),
    /// The same deterministic challenge ID is already retained.
    Duplicate {
        /// Duplicate challenge identity.
        challenge_id: String,
    },
}

/// Result of attempting one queued transport handoff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidEpochChallengeSubmitOutcome {
    /// One transport handoff was acknowledged.
    Submitted(Box<InvalidEpochChallengeSubmissionRecordV1>),
    /// No queued challenge was available.
    Empty,
}

/// Deterministic challenge-outbox failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InvalidEpochChallengeOutboxError {
    /// Outbox capacity must be nonzero.
    #[error("invalid-epoch challenge outbox capacity must be greater than zero")]
    ZeroCapacity,

    /// A local read requested an unsupported item count.
    #[error("invalid-epoch challenge read limit must be within 1..={maximum}; actual={actual}")]
    InvalidReadLimit {
        /// Requested item count.
        actual: usize,
        /// Maximum supported item count.
        maximum: usize,
    },

    /// The canonical challenge DTO failed validation.
    #[error("invalid-epoch challenge was rejected before queueing: {0}")]
    InvalidChallenge(String),

    /// Every retained record is still pending or being dispatched.
    #[error("invalid-epoch challenge outbox is full; capacity={capacity}")]
    OutboxFull {
        /// Configured record capacity.
        capacity: usize,
    },

    /// The process-local sequence counter overflowed.
    #[error("invalid-epoch challenge outbox sequence exhausted")]
    SequenceExhausted,

    /// The per-record submission-attempt counter overflowed.
    #[error("invalid-epoch challenge submission attempt counter exhausted")]
    AttemptCounterExhausted,

    /// The configured transport rejected or failed the handoff.
    #[error("invalid-epoch challenge transport failed: {0}")]
    Sink(String),

    /// The transport returned malformed acknowledgement data.
    #[error("invalid-epoch challenge transport acknowledgement is invalid: {field}")]
    InvalidAcknowledgement {
        /// Invalid acknowledgement field.
        field: &'static str,
    },

    /// Internal record state changed unexpectedly during a handoff.
    #[error("invalid-epoch challenge outbox record state changed unexpectedly")]
    UnexpectedRecordState,
}

#[derive(Debug)]
struct ChallengeOutboxState {
    next_sequence: u64,
    seen: HashSet<String>,
    records: VecDeque<InvalidEpochChallengeSubmissionRecordV1>,
}

impl ChallengeOutboxState {
    fn new(capacity: usize) -> Self {
        Self {
            next_sequence: 1,
            seen: HashSet::with_capacity(capacity),
            records: VecDeque::with_capacity(capacity),
        }
    }
}

/// Bounded process-local invalid-epoch challenge outbox.
///
/// Pending records are never silently discarded. When capacity is reached, the
/// oldest transport-acknowledged record may be replaced, but an outbox
/// containing only pending work fails closed with `OutboxFull`.
#[derive(Debug)]
pub struct InvalidEpochChallengeOutbox {
    capacity: usize,
    state: Mutex<ChallengeOutboxState>,
}

impl InvalidEpochChallengeOutbox {
    /// Construct a bounded challenge outbox.
    ///
    /// # Errors
    ///
    /// Returns `ZeroCapacity` when `capacity` is zero.
    pub fn new(capacity: usize) -> Result<Self, InvalidEpochChallengeOutboxError> {
        if capacity == 0 {
            return Err(InvalidEpochChallengeOutboxError::ZeroCapacity);
        }

        Ok(Self { capacity, state: Mutex::new(ChallengeOutboxState::new(capacity)) })
    }

    /// Validate and queue one canonical challenge.
    ///
    /// Duplicate deterministic challenge IDs are reported without creating a
    /// second record. Pending records are never evicted to make room.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid challenge, a full pending outbox, or
    /// sequence exhaustion.
    pub fn enqueue(
        &self,
        challenge: InvalidEpochChallengeV1,
    ) -> Result<InvalidEpochChallengeEnqueueOutcome, InvalidEpochChallengeOutboxError> {
        challenge.validate().map_err(|error| {
            InvalidEpochChallengeOutboxError::InvalidChallenge(error.to_string())
        })?;

        let mut state = self.state.lock();

        if state.seen.contains(&challenge.challenge_id) {
            return Ok(InvalidEpochChallengeEnqueueOutcome::Duplicate {
                challenge_id: challenge.challenge_id,
            });
        }

        if state.records.len() == self.capacity {
            let removable_index = state.records.iter().position(|record| {
                record.submission_state == InvalidEpochChallengeSubmissionStateV1::Submitted
            });

            let Some(removable_index) = removable_index else {
                return Err(InvalidEpochChallengeOutboxError::OutboxFull {
                    capacity: self.capacity,
                });
            };

            let removed = state
                .records
                .remove(removable_index)
                .ok_or(InvalidEpochChallengeOutboxError::UnexpectedRecordState)?;
            state.seen.remove(&removed.challenge.challenge_id);
        }

        let sequence = state.next_sequence;
        state.next_sequence = state
            .next_sequence
            .checked_add(1)
            .ok_or(InvalidEpochChallengeOutboxError::SequenceExhausted)?;

        let record = InvalidEpochChallengeSubmissionRecordV1 {
            sequence,
            challenge,
            submission_state: InvalidEpochChallengeSubmissionStateV1::Queued,
            submission_attempts: 0,
            submission_ref: None,
            acknowledged_at_ms: None,
            evidence_only: true,
            challenge_accepted: false,
            finality_claimed: false,
            wallet_mutation: false,
            ledger_mutation: false,
        };

        state.seen.insert(record.challenge.challenge_id.clone());
        state.records.push_back(record.clone());

        Ok(InvalidEpochChallengeEnqueueOutcome::Queued(Box::new(record)))
    }

    /// Submit the oldest queued challenge through the configured sink.
    ///
    /// A sink failure returns the record to `Queued` so a later retry can use
    /// the same deterministic challenge identity. A valid acknowledgement marks
    /// only transport submission; it does not claim challenge acceptance.
    ///
    /// # Errors
    ///
    /// Returns an error for attempt overflow, sink failure, malformed
    /// acknowledgement, or an unexpected concurrent record-state change.
    pub fn submit_next<S>(
        &self,
        sink: &S,
    ) -> Result<InvalidEpochChallengeSubmitOutcome, InvalidEpochChallengeOutboxError>
    where
        S: InvalidEpochChallengeSubmissionSink,
    {
        let (sequence, challenge) = {
            let mut state = self.state.lock();

            let Some(record) = state.records.iter_mut().find(|record| {
                record.submission_state == InvalidEpochChallengeSubmissionStateV1::Queued
            }) else {
                return Ok(InvalidEpochChallengeSubmitOutcome::Empty);
            };

            record.submission_attempts = record
                .submission_attempts
                .checked_add(1)
                .ok_or(InvalidEpochChallengeOutboxError::AttemptCounterExhausted)?;
            record.submission_state = InvalidEpochChallengeSubmissionStateV1::Dispatching;

            (record.sequence, record.challenge.clone())
        };

        let acknowledgement = match sink.submit(&challenge) {
            Ok(acknowledgement) => acknowledgement,
            Err(error) => {
                self.restore_queued(sequence)?;
                return Err(InvalidEpochChallengeOutboxError::Sink(error.to_string()));
            }
        };

        if !valid_submission_ref(&acknowledgement.submission_ref) {
            self.restore_queued(sequence)?;
            return Err(InvalidEpochChallengeOutboxError::InvalidAcknowledgement {
                field: "submission_ref",
            });
        }

        if acknowledgement.acknowledged_at_ms == 0
            || acknowledgement.acknowledged_at_ms < challenge.submitted_at_ms
        {
            self.restore_queued(sequence)?;
            return Err(InvalidEpochChallengeOutboxError::InvalidAcknowledgement {
                field: "acknowledged_at_ms",
            });
        }

        let mut state = self.state.lock();
        let record = state
            .records
            .iter_mut()
            .find(|record| record.sequence == sequence)
            .ok_or(InvalidEpochChallengeOutboxError::UnexpectedRecordState)?;

        if record.submission_state != InvalidEpochChallengeSubmissionStateV1::Dispatching
            || record.challenge.challenge_id != challenge.challenge_id
        {
            return Err(InvalidEpochChallengeOutboxError::UnexpectedRecordState);
        }

        record.submission_state = InvalidEpochChallengeSubmissionStateV1::Submitted;
        record.submission_ref = Some(acknowledgement.submission_ref);
        record.acknowledged_at_ms = Some(acknowledgement.acknowledged_at_ms);

        Ok(InvalidEpochChallengeSubmitOutcome::Submitted(Box::new(record.clone())))
    }

    /// Return the newest retained challenge records first.
    ///
    /// # Errors
    ///
    /// Returns `InvalidReadLimit` when `limit` is zero or exceeds the bounded
    /// local read maximum.
    pub fn recent(
        &self,
        limit: usize,
    ) -> Result<Vec<InvalidEpochChallengeSubmissionRecordV1>, InvalidEpochChallengeOutboxError>
    {
        if limit == 0 || limit > MAX_INVALID_EPOCH_CHALLENGE_READ_ITEMS {
            return Err(InvalidEpochChallengeOutboxError::InvalidReadLimit {
                actual: limit,
                maximum: MAX_INVALID_EPOCH_CHALLENGE_READ_ITEMS,
            });
        }

        let state = self.state.lock();

        Ok(state.records.iter().rev().take(limit).cloned().collect())
    }

    /// Number of retained queued, dispatching, and submitted records.
    pub fn len(&self) -> usize {
        self.state.lock().records.len()
    }

    /// True when no challenge records are retained.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Number of challenges that still require transport acknowledgement.
    pub fn pending_len(&self) -> usize {
        self.state
            .lock()
            .records
            .iter()
            .filter(|record| {
                record.submission_state != InvalidEpochChallengeSubmissionStateV1::Submitted
            })
            .count()
    }

    fn restore_queued(&self, sequence: u64) -> Result<(), InvalidEpochChallengeOutboxError> {
        let mut state = self.state.lock();
        let record = state
            .records
            .iter_mut()
            .find(|record| record.sequence == sequence)
            .ok_or(InvalidEpochChallengeOutboxError::UnexpectedRecordState)?;

        if record.submission_state != InvalidEpochChallengeSubmissionStateV1::Dispatching {
            return Err(InvalidEpochChallengeOutboxError::UnexpectedRecordState);
        }

        record.submission_state = InvalidEpochChallengeSubmissionStateV1::Queued;
        Ok(())
    }
}

impl Default for InvalidEpochChallengeOutbox {
    fn default() -> Self {
        Self::new(DEFAULT_INVALID_EPOCH_CHALLENGE_OUTBOX_CAPACITY)
            .expect("default invalid-epoch challenge capacity is nonzero")
    }
}

fn valid_submission_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/' | b'@')
        })
}
