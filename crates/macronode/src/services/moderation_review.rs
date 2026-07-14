//! RO:WHAT — Bounded process-local queue for explicit Service Node moderation review.
//! RO:WHY — BUILD_PLAN_Z Phase 21 requires operator review without silently mutating canonical moderation policy.
//! RO:INTERACTS — macronode moderation-review HTTP handlers, RuntimeStatus, and the immutable effective moderation snapshot.
//! RO:INVARIANTS — exact B3 objects; bounded items; deterministic ordering; pending-only transitions; duplicate submission does not reset review.
//! RO:SECURITY — review decisions are metadata only and cannot mutate policy, storage, providers, rewards, wallets, or ledgers.
//! RO:TEST — focused unit tests beside this behavior.

#![forbid(unsafe_code)]

use parking_lot::Mutex;
use ron_policy::B3Id;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

pub const DEFAULT_MODERATION_REVIEW_CAPACITY: usize = 1_024;
pub const MAX_MODERATION_REVIEW_READ_ITEMS: usize = 256;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModerationReviewSourceV1 {
    PolicyRefusalEvidence,
    ModerationActionEvidence,
    OperatorReport,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModerationReviewReasonV1 {
    PolicyRefusal,
    AbuseReport,
    MalwareSuspicion,
    CopyrightClaim,
    OperatorInspection,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModerationReviewStateV1 {
    PendingReview,
    ApprovedForEscalation,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModerationReviewDecisionV1 {
    Approve,
    Reject,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModerationReviewItemV1 {
    pub sequence: u64,
    pub object: String,
    pub source: ModerationReviewSourceV1,
    pub reason: ModerationReviewReasonV1,
    pub effective_policy_reason: String,
    pub currently_permits_serve: bool,
    pub state: ModerationReviewStateV1,
    pub submitted_at_ms: u64,
    pub reviewed_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModerationReviewCountsV1 {
    pub total: usize,
    pub pending_review: usize,
    pub approved_for_escalation: usize,
    pub rejected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModerationReviewMutationV1 {
    changed: bool,
    item: ModerationReviewItemV1,
}

impl ModerationReviewMutationV1 {
    #[must_use]
    pub fn changed(&self) -> bool {
        self.changed
    }

    #[must_use]
    pub fn item(&self) -> &ModerationReviewItemV1 {
        &self.item
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModerationReviewError {
    #[error("moderation review capacity must be greater than zero")]
    ZeroCapacity,

    #[error("moderation review read limit must be within 1..={maximum}; actual={actual}")]
    InvalidReadLimit { actual: usize, maximum: usize },

    #[error("moderation review queue is full; capacity={capacity}")]
    CapacityReached { capacity: usize },

    #[error("moderation review sequence exhausted")]
    SequenceExhausted,

    #[error("moderation review item was not found; sequence={sequence}")]
    NotFound { sequence: u64 },

    #[error(
        "object already has a pending moderation review with different metadata; object={object}"
    )]
    ConflictingPendingReview { object: String },

    #[error(
        "moderation review transition conflicts with existing state; sequence={sequence}; state={state:?}"
    )]
    TransitionConflict {
        sequence: u64,
        state: ModerationReviewStateV1,
    },
}

#[derive(Debug, Default)]
struct ModerationReviewCatalogState {
    next_sequence: u64,
    items: BTreeMap<u64, ModerationReviewItemV1>,
}

#[derive(Debug)]
pub struct ModerationReviewCatalog {
    capacity: usize,
    state: Mutex<ModerationReviewCatalogState>,
}

impl Default for ModerationReviewCatalog {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_MODERATION_REVIEW_CAPACITY)
            .expect("default moderation review capacity must be valid")
    }
}

impl ModerationReviewCatalog {
    pub fn with_capacity(capacity: usize) -> Result<Self, ModerationReviewError> {
        if capacity == 0 {
            return Err(ModerationReviewError::ZeroCapacity);
        }

        Ok(Self {
            capacity,
            state: Mutex::new(ModerationReviewCatalogState::default()),
        })
    }

    pub fn submit(
        &self,
        object: B3Id,
        source: ModerationReviewSourceV1,
        reason: ModerationReviewReasonV1,
        effective_policy_reason: String,
        currently_permits_serve: bool,
        submitted_at_ms: u64,
    ) -> Result<ModerationReviewMutationV1, ModerationReviewError> {
        let object = object.as_str().to_string();
        let mut state = self.state.lock();

        if let Some(existing) = state.items.values().find(|item| {
            item.object == object && item.state == ModerationReviewStateV1::PendingReview
        }) {
            if existing.source == source
                && existing.reason == reason
                && existing.effective_policy_reason == effective_policy_reason
                && existing.currently_permits_serve == currently_permits_serve
            {
                return Ok(ModerationReviewMutationV1 {
                    changed: false,
                    item: existing.clone(),
                });
            }

            return Err(ModerationReviewError::ConflictingPendingReview { object });
        }

        if state.items.len() >= self.capacity {
            return Err(ModerationReviewError::CapacityReached {
                capacity: self.capacity,
            });
        }

        let sequence = state
            .next_sequence
            .checked_add(1)
            .ok_or(ModerationReviewError::SequenceExhausted)?;

        state.next_sequence = sequence;

        let item = ModerationReviewItemV1 {
            sequence,
            object,
            source,
            reason,
            effective_policy_reason,
            currently_permits_serve,
            state: ModerationReviewStateV1::PendingReview,
            submitted_at_ms,
            reviewed_at_ms: None,
        };

        state.items.insert(sequence, item.clone());

        Ok(ModerationReviewMutationV1 {
            changed: true,
            item,
        })
    }

    pub fn status(&self, sequence: u64) -> Option<ModerationReviewItemV1> {
        self.state.lock().items.get(&sequence).cloned()
    }

    pub fn list_pending(
        &self,
        limit: usize,
    ) -> Result<Vec<ModerationReviewItemV1>, ModerationReviewError> {
        if limit == 0 || limit > MAX_MODERATION_REVIEW_READ_ITEMS {
            return Err(ModerationReviewError::InvalidReadLimit {
                actual: limit,
                maximum: MAX_MODERATION_REVIEW_READ_ITEMS,
            });
        }

        Ok(self
            .state
            .lock()
            .items
            .values()
            .filter(|item| item.state == ModerationReviewStateV1::PendingReview)
            .take(limit)
            .cloned()
            .collect())
    }

    pub fn review(
        &self,
        sequence: u64,
        decision: ModerationReviewDecisionV1,
        reviewed_at_ms: u64,
    ) -> Result<ModerationReviewMutationV1, ModerationReviewError> {
        let mut state = self.state.lock();

        let item = state
            .items
            .get_mut(&sequence)
            .ok_or(ModerationReviewError::NotFound { sequence })?;

        let target = match decision {
            ModerationReviewDecisionV1::Approve => ModerationReviewStateV1::ApprovedForEscalation,
            ModerationReviewDecisionV1::Reject => ModerationReviewStateV1::Rejected,
        };

        if item.state == target {
            return Ok(ModerationReviewMutationV1 {
                changed: false,
                item: item.clone(),
            });
        }

        if item.state != ModerationReviewStateV1::PendingReview {
            return Err(ModerationReviewError::TransitionConflict {
                sequence,
                state: item.state,
            });
        }

        item.state = target;
        item.reviewed_at_ms = Some(reviewed_at_ms);

        Ok(ModerationReviewMutationV1 {
            changed: true,
            item: item.clone(),
        })
    }

    #[must_use]
    pub fn counts(&self) -> ModerationReviewCountsV1 {
        let state = self.state.lock();
        let mut counts = ModerationReviewCountsV1 {
            total: state.items.len(),
            ..ModerationReviewCountsV1::default()
        };

        for item in state.items.values() {
            match item.state {
                ModerationReviewStateV1::PendingReview => {
                    counts.pending_review += 1;
                }
                ModerationReviewStateV1::ApprovedForEscalation => {
                    counts.approved_for_escalation += 1;
                }
                ModerationReviewStateV1::Rejected => {
                    counts.rejected += 1;
                }
            }
        }

        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBJECT_A: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const OBJECT_B: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn object(value: &str) -> B3Id {
        value.parse().expect("test object must be canonical")
    }

    fn submit(catalog: &ModerationReviewCatalog, value: &str) -> ModerationReviewMutationV1 {
        catalog
            .submit(
                object(value),
                ModerationReviewSourceV1::OperatorReport,
                ModerationReviewReasonV1::AbuseReport,
                "no_rule".to_string(),
                true,
                1_000,
            )
            .expect("review submission")
    }

    #[test]
    fn duplicate_pending_submission_is_idempotent() {
        let catalog = ModerationReviewCatalog::default();

        let first = submit(&catalog, OBJECT_A);
        let duplicate = submit(&catalog, OBJECT_A);

        assert!(first.changed());
        assert!(!duplicate.changed());
        assert_eq!(first.item().sequence, duplicate.item().sequence,);
        assert_eq!(catalog.counts().pending_review, 1);
    }

    #[test]
    fn review_decisions_are_explicit_and_do_not_reset() {
        let catalog = ModerationReviewCatalog::default();

        let sequence = submit(&catalog, OBJECT_A).item().sequence;

        let approved = catalog
            .review(sequence, ModerationReviewDecisionV1::Approve, 2_000)
            .expect("approval");

        assert!(approved.changed());
        assert_eq!(
            approved.item().state,
            ModerationReviewStateV1::ApprovedForEscalation,
        );

        let duplicate = catalog
            .review(sequence, ModerationReviewDecisionV1::Approve, 3_000)
            .expect("idempotent approval");

        assert!(!duplicate.changed());

        assert!(matches!(
            catalog.review(sequence, ModerationReviewDecisionV1::Reject, 4_000,),
            Err(ModerationReviewError::TransitionConflict { .. }),
        ));
    }

    #[test]
    fn capacity_and_read_limits_are_bounded() {
        let catalog = ModerationReviewCatalog::with_capacity(1).expect("bounded catalog");

        submit(&catalog, OBJECT_A);

        // A second distinct object must fail once capacity is full.
        let error = catalog
            .submit(
                object(OBJECT_B),
                ModerationReviewSourceV1::PolicyRefusalEvidence,
                ModerationReviewReasonV1::PolicyRefusal,
                "local_block".to_string(),
                false,
                2_000,
            )
            .expect_err("capacity must reject");

        assert!(matches!(
            error,
            ModerationReviewError::CapacityReached { capacity: 1 },
        ));

        assert!(matches!(
            catalog.list_pending(0),
            Err(ModerationReviewError::InvalidReadLimit { .. }),
        ));
    }
}
