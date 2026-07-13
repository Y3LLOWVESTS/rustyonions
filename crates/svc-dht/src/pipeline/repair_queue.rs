//! RO:WHAT — Bounded repair-request queue and policy-revalidating local worker.
//! RO:WHY — Phase 12 needs backpressured repair scheduling and truthful churn recovery.
//! RO:INTERACTS — pipeline::repair, provider::Store, ron-policy moderation, Tokio mpsc.
//! RO:INVARIANTS — bounded queue; no duplicate pending key; revalidate before execution.
//! RO:SECURITY — refused content never executes; requests contain no raw transport address.
//! RO:TEST — tests/provider_repair_queue.rs.

use crate::{
    pipeline::repair::{review_repair_need, RepairCandidate, RepairReview, RepairReviewError},
    provider::Store,
    types::B3Cid,
};
use parking_lot::Mutex;
use ron_policy::{ModerationPolicy, ModerationReasonCode};
use std::{collections::HashSet, fmt, future::Future, sync::Arc};
use tokio::sync::mpsc;

/// One bounded repair request.
///
/// The request intentionally stores only the canonical CID and desired target.
/// Provider inventory and moderation state are reviewed again by the worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairRequest {
    cid: B3Cid,
    target_provider_count: usize,
}

impl RepairRequest {
    pub fn cid(&self) -> &B3Cid {
        &self.cid
    }

    pub fn target_provider_count(&self) -> usize {
        self.target_provider_count
    }

    fn key(&self) -> RepairRequestKey {
        RepairRequestKey {
            cid: self.cid.clone(),
            target_provider_count: self.target_provider_count,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RepairRequestKey {
    cid: B3Cid,
    target_provider_count: usize,
}

/// Invalid bounded-queue configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairQueueConfigError {
    ZeroCapacity,
}

impl RepairQueueConfigError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ZeroCapacity => "repair_queue_capacity_must_be_nonzero",
        }
    }
}

impl fmt::Display for RepairQueueConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for RepairQueueConfigError {}

/// Truthful result of reviewing and attempting to enqueue repair work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairEnqueueOutcome {
    /// One new bounded queue entry was accepted.
    Enqueued,

    /// The same CID and target are already pending.
    DuplicatePending,

    /// Provider count already meets the target.
    TargetSatisfied,

    /// Canonical moderation policy forbids repair.
    Suppressed { reason: ModerationReasonCode },
}

/// Failure while reviewing or entering the bounded queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairEnqueueError {
    Review(RepairReviewError),
    QueueFull,
    QueueClosed,
}

impl RepairEnqueueError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Review(error) => error.as_str(),
            Self::QueueFull => "repair_queue_full",
            Self::QueueClosed => "repair_queue_closed",
        }
    }
}

impl fmt::Display for RepairEnqueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for RepairEnqueueError {}

/// Cloneable producer and pending-request registry.
#[derive(Clone)]
pub struct RepairQueue {
    tx: mpsc::Sender<RepairRequest>,
    pending: Arc<Mutex<HashSet<RepairRequestKey>>>,
}

/// Single-consumer repair queue receiver.
pub struct RepairQueueReceiver {
    rx: mpsc::Receiver<RepairRequest>,
}

/// Construct one bounded repair queue.
pub fn bounded_repair_queue(
    capacity: usize,
) -> Result<(RepairQueue, RepairQueueReceiver), RepairQueueConfigError> {
    if capacity == 0 {
        return Err(RepairQueueConfigError::ZeroCapacity);
    }

    let (tx, rx) = mpsc::channel(capacity);

    Ok((
        RepairQueue { tx, pending: Arc::new(Mutex::new(HashSet::new())) },
        RepairQueueReceiver { rx },
    ))
}

impl RepairQueue {
    /// Reevaluate moderation and provider count before entering the queue.
    ///
    /// `try_send` is intentionally used so a saturated repair path never
    /// applies unbounded producer backpressure or silently buffers work.
    pub fn review_and_enqueue(
        &self,
        store: &Store,
        cid: &B3Cid,
        target_provider_count: usize,
        moderation_policy: &ModerationPolicy,
    ) -> Result<RepairEnqueueOutcome, RepairEnqueueError> {
        match review_repair_need(store, cid, target_provider_count, moderation_policy)
            .map_err(RepairEnqueueError::Review)?
        {
            RepairReview::Suppressed { reason } => Ok(RepairEnqueueOutcome::Suppressed { reason }),
            RepairReview::TargetSatisfied { .. } => Ok(RepairEnqueueOutcome::TargetSatisfied),
            RepairReview::Candidate(_) => {
                let request = RepairRequest { cid: cid.clone(), target_provider_count };
                let key = request.key();

                {
                    let mut pending = self.pending.lock();

                    if !pending.insert(key.clone()) {
                        return Ok(RepairEnqueueOutcome::DuplicatePending);
                    }
                }

                match self.tx.try_send(request) {
                    Ok(()) => Ok(RepairEnqueueOutcome::Enqueued),
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        self.pending.lock().remove(&key);
                        Err(RepairEnqueueError::QueueFull)
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        self.pending.lock().remove(&key);
                        Err(RepairEnqueueError::QueueClosed)
                    }
                }
            }
        }
    }

    fn finish(&self, request: &RepairRequest) {
        self.pending.lock().remove(&request.key());
    }
}

impl RepairQueueReceiver {
    async fn recv(&mut self) -> Option<RepairRequest> {
        self.rx.recv().await
    }
}

/// Result of processing one bounded repair request.
#[derive(Debug, PartialEq, Eq)]
pub enum RepairWorkerOutcome<E> {
    /// Every producer was dropped and no work remained.
    QueueClosed,

    /// Policy changed after enqueue and now suppresses repair.
    Suppressed { cid: B3Cid, reason: ModerationReasonCode },

    /// Provider count recovered before execution began.
    TargetSatisfied { cid: B3Cid },

    /// Revalidation failed closed.
    ReviewFailed { cid: B3Cid, error: RepairReviewError },

    /// A currently valid candidate was passed to the injected executor.
    Attempted { cid: B3Cid, result: Result<(), E> },
}

/// Receive and process one repair request.
///
/// Moderation and provider counts are revalidated after dequeue. This prevents
/// stale queued work from repairing newly tombstoned content or performing
/// unnecessary work after another provider has already restored redundancy.
///
/// The pending key is released on every terminal outcome, including executor
/// failure, so a later planner pass may truthfully retry the request.
pub async fn run_repair_worker_once<F, Fut, E>(
    receiver: &mut RepairQueueReceiver,
    queue: &RepairQueue,
    store: &Store,
    moderation_policy: &ModerationPolicy,
    execute: F,
) -> RepairWorkerOutcome<E>
where
    F: FnOnce(RepairCandidate) -> Fut,
    Fut: Future<Output = Result<(), E>>,
{
    let Some(request) = receiver.recv().await else {
        return RepairWorkerOutcome::QueueClosed;
    };

    let cid = request.cid.clone();

    let outcome = match review_repair_need(
        store,
        &request.cid,
        request.target_provider_count,
        moderation_policy,
    ) {
        Ok(RepairReview::Suppressed { reason }) => RepairWorkerOutcome::Suppressed { cid, reason },
        Ok(RepairReview::TargetSatisfied { .. }) => RepairWorkerOutcome::TargetSatisfied { cid },
        Ok(RepairReview::Candidate(candidate)) => {
            let result = execute(candidate).await;

            RepairWorkerOutcome::Attempted { cid, result }
        }
        Err(error) => RepairWorkerOutcome::ReviewFailed { cid, error },
    };

    queue.finish(&request);
    outcome
}
