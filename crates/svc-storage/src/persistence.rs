//! RO:WHAT — Canonical persistence-state metadata and policy-gated transitions.
//!
//! RO:WHY — `BUILD_PLAN_Z` Phase 11 keeps unvetted bytes amnesia-first until
//! canonical moderation, category, and review policy permit persistence.
//!
//! RO:INTERACTS — `ron-policy` persistence and moderation decisions,
//! `ron-proto::asset::AssetKind`, storage backends, and later operator controls.
//!
//! RO:INVARIANTS — exact B3 IDs; default ephemeral state; no direct approval or
//! pin bypass; moderation refusal always defeats durable eligibility.
//!
//! RO:SECURITY — metadata only; no disk I/O, serve override, provider mutation,
//! rewards, wallet authority, or ledger authority.
//!
//! RO:TEST — `tests/persistence_state.rs`.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use ron_policy::{
    B3Id, ModerationPolicy, PersistenceIntent, PersistencePolicy, PersistenceReasonCode,
    PersistenceReviewLevel,
};
use ron_proto::asset::AssetKind;
use serde::{Deserialize, Serialize};

/// Canonical lifecycle state for one exact stored object.
///
/// This state records persistence-policy posture. It does not prove that bytes
/// have been written to a durable backend. Only
/// [`PersistenceState::is_durable_storage_eligible`] answers whether a later
/// backend operation may be attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceState {
    /// Newly accepted bytes remain in an amnesia-first, evictable store.
    #[default]
    EphemeralUnvetted,
    /// The exact object is waiting for an explicit persistence review.
    PendingReview,
    /// Review completed and policy permits a later durable-storage operation.
    VerifiedPersistent,
    /// Operator policy refuses persistence for the exact object.
    OperatorBlocked,
    /// Authenticated global policy refuses persistence for the exact object.
    GlobalDenied,
    /// The object's owner has withdrawn the exact object.
    OwnerTombstoned,
    /// The exact object is isolated and cannot become durable pending review.
    Quarantined,
    /// A verified object has been explicitly selected for operator retention.
    PinnedByOperator,
}

impl PersistenceState {
    /// Return the canonical low-cardinality state label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EphemeralUnvetted => "ephemeral_unvetted",
            Self::PendingReview => "pending_review",
            Self::VerifiedPersistent => "verified_persistent",
            Self::OperatorBlocked => "operator_blocked",
            Self::GlobalDenied => "global_denied",
            Self::OwnerTombstoned => "owner_tombstoned",
            Self::Quarantined => "quarantined",
            Self::PinnedByOperator => "pinned_by_operator",
        }
    }

    /// Whether policy state permits a later durable-storage operation.
    ///
    /// This is eligibility only. It does not claim that durable bytes exist.
    #[must_use]
    pub const fn is_durable_storage_eligible(self) -> bool {
        matches!(self, Self::VerifiedPersistent | Self::PinnedByOperator)
    }
}

/// Explicit review action supported by the first Phase 11 state-machine slice.
///
/// Moderation-driven transitions are intentionally not represented here.
/// They will be derived from canonical `ron-policy` decisions in a later
/// Phase 11 slice rather than reimplementing moderation precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceReviewAction {
    /// Submit an ephemeral exact object for persistence review.
    SubmitForReview,
    /// Refuse persistence for an object already pending review.
    ///
    /// This is a local persistence decision. It does not add the object to
    /// canonical moderation policy or claim that serving has been disabled.
    Reject,
    /// Request approval of a pending object.
    ///
    /// This action cannot be applied directly. Approval must pass
    /// [`PersistenceRecord::apply_persistence_policy`].
    ApproveVerified,
    /// Request operator retention of an already verified object.
    ///
    /// This action cannot be applied directly. Pinning must pass
    /// [`PersistenceRecord::apply_persistence_policy`].
    Pin,
    /// Remove operator pinning while preserving verified eligibility.
    Unpin,
}

impl PersistenceReviewAction {
    /// Return the canonical low-cardinality action label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SubmitForReview => "submit_for_review",
            Self::Reject => "reject",
            Self::ApproveVerified => "approve_verified",
            Self::Pin => "pin",
            Self::Unpin => "unpin",
        }
    }
}

/// Rejected persistence-review transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidPersistenceTransition {
    state: PersistenceState,
    action: PersistenceReviewAction,
}

impl InvalidPersistenceTransition {
    const fn new(state: PersistenceState, action: PersistenceReviewAction) -> Self {
        Self { state, action }
    }

    /// State from which the transition was rejected.
    #[must_use]
    pub const fn state(self) -> PersistenceState {
        self.state
    }

    /// Action that was rejected.
    #[must_use]
    pub const fn action(self) -> PersistenceReviewAction {
        self.action
    }
}

impl fmt::Display for InvalidPersistenceTransition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "persistence action {} is invalid from state {}",
            self.action.as_str(),
            self.state.as_str(),
        )
    }
}

impl Error for InvalidPersistenceTransition {}

/// Rejected transition after evaluating canonical persistence policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistencePolicyTransitionError {
    /// The policy returned an internally inconsistent effect/reason pair.
    InvalidDecision {
        /// Whether the policy effect claimed to permit persistence.
        permitted: bool,
        /// Reason supplied by the policy decision.
        reason: PersistenceReasonCode,
    },
    /// Policy refused the requested persistence intent.
    Ineligible {
        /// State that remained active.
        state: PersistenceState,
        /// Requested policy intent.
        intent: PersistenceIntent,
        /// Canonical policy refusal reason.
        reason: PersistenceReasonCode,
    },
    /// Policy permitted the intent, but the current lifecycle state cannot
    /// legally perform it.
    InvalidState {
        /// State from which the transition was rejected.
        state: PersistenceState,
        /// Requested policy intent.
        intent: PersistenceIntent,
    },
}

impl fmt::Display for PersistencePolicyTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDecision { permitted, reason } => write!(
                formatter,
                "inconsistent persistence policy decision: permitted={permitted}, reason={}",
                reason.as_str(),
            ),
            Self::Ineligible {
                state,
                intent,
                reason,
            } => write!(
                formatter,
                "persistence policy refused {} from state {}: {}",
                intent.as_str(),
                state.as_str(),
                reason.as_str(),
            ),
            Self::InvalidState { state, intent } => write!(
                formatter,
                "persistence intent {} is invalid from state {}",
                intent.as_str(),
                state.as_str(),
            ),
        }
    }
}

impl Error for PersistencePolicyTransitionError {}

const fn moderation_refusal_state(reason: PersistenceReasonCode) -> Option<PersistenceState> {
    match reason {
        PersistenceReasonCode::GlobalDeny => Some(PersistenceState::GlobalDenied),
        PersistenceReasonCode::OwnerTombstone => Some(PersistenceState::OwnerTombstoned),
        PersistenceReasonCode::LocalBlock => Some(PersistenceState::OperatorBlocked),
        PersistenceReasonCode::Quarantined => Some(PersistenceState::Quarantined),
        PersistenceReasonCode::Eligible
        | PersistenceReasonCode::AssetKindNotAllowed
        | PersistenceReasonCode::ReviewThresholdNotMet
        | PersistenceReasonCode::OperatorPinningDisabled => None,
    }
}

/// Persistence-policy record for one exact canonical B3 object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistenceRecord {
    object: B3Id,
    #[serde(default)]
    state: PersistenceState,
}

impl PersistenceRecord {
    /// Create an amnesia-first record for one exact object.
    #[must_use]
    pub const fn new(object: B3Id) -> Self {
        Self {
            object,
            state: PersistenceState::EphemeralUnvetted,
        }
    }

    /// Exact canonical object associated with this record.
    #[must_use]
    pub const fn object(&self) -> &B3Id {
        &self.object
    }

    /// Current persistence-policy state.
    #[must_use]
    pub const fn state(&self) -> PersistenceState {
        self.state
    }

    /// Whether the current policy state permits a later durable write.
    ///
    /// Returning `true` does not claim that a durable write has occurred.
    #[must_use]
    pub const fn is_durable_storage_eligible(&self) -> bool {
        self.state.is_durable_storage_eligible()
    }

    /// Evaluate and apply canonical persistence policy for this exact object.
    ///
    /// Moderation, asset-category, review-threshold, and operator-pinning
    /// decisions remain owned by `ron-policy`. This method only projects an
    /// accepted decision into the storage lifecycle state.
    ///
    /// Canonical moderation refusal reasons immediately move the record into
    /// the corresponding non-durable state. Other policy refusals leave the
    /// current state unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`PersistencePolicyTransitionError::Ineligible`] when category,
    /// review, or pin policy refuses the request. Returns
    /// [`PersistencePolicyTransitionError::InvalidState`] when policy permits
    /// an operation that would skip required lifecycle ordering.
    pub fn apply_persistence_policy(
        &mut self,
        policy: &PersistencePolicy,
        moderation: &ModerationPolicy,
        asset_kind: AssetKind,
        review: PersistenceReviewLevel,
        intent: PersistenceIntent,
    ) -> Result<bool, PersistencePolicyTransitionError> {
        let decision = policy.evaluate(moderation, &self.object, asset_kind, review, intent);
        let permitted = decision.permits_persistence();

        if !permitted {
            if let Some(next) = moderation_refusal_state(decision.reason) {
                let changed = self.state != next;
                self.state = next;
                return Ok(changed);
            }

            if matches!(
                decision.reason,
                PersistenceReasonCode::AssetKindNotAllowed
                    | PersistenceReasonCode::ReviewThresholdNotMet
                    | PersistenceReasonCode::OperatorPinningDisabled
            ) {
                return Err(PersistencePolicyTransitionError::Ineligible {
                    state: self.state,
                    intent,
                    reason: decision.reason,
                });
            }

            return Err(PersistencePolicyTransitionError::InvalidDecision {
                permitted,
                reason: decision.reason,
            });
        }

        if decision.reason != PersistenceReasonCode::Eligible {
            return Err(PersistencePolicyTransitionError::InvalidDecision {
                permitted,
                reason: decision.reason,
            });
        }

        use PersistenceIntent::{Persist, Pin};
        use PersistenceState::{PendingReview, PinnedByOperator, VerifiedPersistent};

        let next = match (self.state, intent) {
            (PendingReview, Persist) => VerifiedPersistent,
            (state @ (VerifiedPersistent | PinnedByOperator), Persist) => state,

            (VerifiedPersistent, Pin) => PinnedByOperator,
            (PinnedByOperator, Pin) => PinnedByOperator,

            (state, rejected_intent) => {
                return Err(PersistencePolicyTransitionError::InvalidState {
                    state,
                    intent: rejected_intent,
                });
            }
        };

        let changed = self.state != next;
        self.state = next;

        Ok(changed)
    }

    /// Apply a local persistence-review workflow action.
    ///
    /// Submission for review, local rejection, and removal of an existing pin
    /// are local lifecycle operations. Approval and pin requests are deliberately
    /// rejected here;
    /// callers must use [`Self::apply_persistence_policy`] so canonical policy
    /// cannot be bypassed.
    ///
    /// Repeating an already-active local action is idempotent and returns
    /// `Ok(false)`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidPersistenceTransition`] when an action would skip
    /// required review, directly approve or pin without policy, or alter a
    /// moderation-owned refusal state.
    pub fn apply_review_action(
        &mut self,
        action: PersistenceReviewAction,
    ) -> Result<bool, InvalidPersistenceTransition> {
        use PersistenceReviewAction::{Reject, SubmitForReview, Unpin};
        use PersistenceState::{
            EphemeralUnvetted, OperatorBlocked, PendingReview, PinnedByOperator, VerifiedPersistent,
        };

        let next = match (self.state, action) {
            (EphemeralUnvetted, SubmitForReview) => PendingReview,
            (PendingReview, SubmitForReview) => PendingReview,

            (PendingReview, Reject) => OperatorBlocked,
            (OperatorBlocked, Reject) => OperatorBlocked,

            (PinnedByOperator, Unpin) => VerifiedPersistent,
            (VerifiedPersistent, Unpin) => VerifiedPersistent,

            (state, rejected_action) => {
                return Err(InvalidPersistenceTransition::new(state, rejected_action));
            }
        };

        let changed = self.state != next;
        self.state = next;

        Ok(changed)
    }
}
