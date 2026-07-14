//! RO:WHAT — Thread-safe runtime catalog for exact-B3 persistence candidates.
//!
//! RO:WHY — BUILD_PLAN_Z Phase 11 operator controls need one real state owner
//! that status, list, approve, reject, pin, and unpin operations can share.
//!
//! RO:INTERACTS — persistence records, canonical ron-policy decisions,
//! moderation snapshots, asset kinds, and later macronode operator adapters.
//!
//! RO:INVARIANTS — duplicate registration never resets state; failed mutations
//! do not partially commit; review listings are deterministic and bounded.
//!
//! RO:SECURITY — metadata and eligibility only; no durable byte write, serve
//! override, provider mutation, reward, wallet, or ledger authority.
//!
//! RO:TEST — tests/persistence_catalog.rs.

#![forbid(unsafe_code)]

use std::collections::{btree_map::Entry, BTreeMap};

use parking_lot::RwLock;
use ron_policy::{
    B3Id, ModerationPolicy, PersistenceIntent, PersistencePolicy, PersistenceReviewLevel,
};
use ron_proto::asset::AssetKind;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::persistence::{
    InvalidPersistenceTransition, PersistencePolicyTransitionError, PersistenceRecord,
    PersistenceReviewAction, PersistenceState,
};

/// Maximum number of candidates returned by one operator-list request.
pub const MAX_PERSISTENCE_REVIEW_ITEMS: usize = 1_024;

/// One exact object and its canonical persistence-review metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistenceCandidate {
    record: PersistenceRecord,
    asset_kind: AssetKind,
}

impl PersistenceCandidate {
    fn new(object: B3Id, asset_kind: AssetKind) -> Self {
        Self {
            record: PersistenceRecord::new(object),
            asset_kind,
        }
    }

    /// Exact canonical object identifier.
    #[must_use]
    pub const fn object(&self) -> &B3Id {
        self.record.object()
    }

    /// Canonical asset category supplied when the object was registered.
    #[must_use]
    pub const fn asset_kind(&self) -> AssetKind {
        self.asset_kind
    }

    /// Current persistence lifecycle state.
    #[must_use]
    pub const fn state(&self) -> PersistenceState {
        self.record.state()
    }

    /// Whether policy state permits a later durable-backend operation.
    ///
    /// This does not claim that such an operation has occurred.
    #[must_use]
    pub const fn is_durable_storage_eligible(&self) -> bool {
        self.record.is_durable_storage_eligible()
    }
}

/// Result returned after one successful catalog mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PersistenceMutation {
    candidate: PersistenceCandidate,
    changed: bool,
}

impl PersistenceMutation {
    fn new(candidate: PersistenceCandidate, changed: bool) -> Self {
        Self { candidate, changed }
    }

    /// Updated candidate snapshot.
    #[must_use]
    pub const fn candidate(&self) -> &PersistenceCandidate {
        &self.candidate
    }

    /// Whether this request changed canonical runtime metadata.
    #[must_use]
    pub const fn changed(&self) -> bool {
        self.changed
    }
}

/// Low-cardinality persistence workflow counts safe for status projection.
///
/// These values describe process-local eligibility metadata only. They do not
/// prove that durable bytes were written or that serving, rewards, wallets, or
/// ledgers were mutated.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PersistenceCatalogCounts {
    pub total: usize,
    pub ephemeral_unvetted: usize,
    pub pending_review: usize,
    pub verified_persistent: usize,
    pub operator_blocked: usize,
    pub global_denied: usize,
    pub owner_tombstoned: usize,
    pub quarantined: usize,
    pub pinned_by_operator: usize,
    pub durable_storage_eligible: usize,
}

/// Runtime-catalog refusal.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PersistenceCatalogError {
    /// The requested exact object is not registered.
    #[error("persistence object is not registered: {object}")]
    NotFound {
        /// Missing exact object.
        object: B3Id,
    },

    /// A pending-list request exceeded the closed bound.
    #[error("invalid persistence review-list limit: {limit}; expected 1..={max}")]
    InvalidLimit {
        /// Rejected requested limit.
        limit: usize,
        /// Maximum accepted limit.
        max: usize,
    },

    /// Local lifecycle ordering rejected the request.
    #[error(transparent)]
    Transition(#[from] InvalidPersistenceTransition),

    /// Canonical persistence policy rejected the request.
    #[error(transparent)]
    Policy(#[from] PersistencePolicyTransitionError),
}

/// Process-local owner of persistence-review metadata.
///
/// The catalog is intentionally independent from byte residency. A candidate
/// may be eligible for durable storage while the current backend remains
/// entirely ephemeral.
#[derive(Debug, Default)]
pub struct PersistenceCatalog {
    candidates: RwLock<BTreeMap<B3Id, PersistenceCandidate>>,
}

impl PersistenceCatalog {
    /// Build an empty runtime catalog.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one exact object in the amnesia-first state.
    ///
    /// Duplicate registration is idempotent and never resets an existing
    /// candidate's category or lifecycle state.
    #[must_use]
    pub fn register(&self, object: B3Id, asset_kind: AssetKind) -> bool {
        let mut candidates = self.candidates.write();

        match candidates.entry(object.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(PersistenceCandidate::new(object, asset_kind));
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    /// Number of currently registered exact objects.
    #[must_use]
    pub fn candidate_count(&self) -> usize {
        self.candidates.read().len()
    }

    /// Return a consistent low-cardinality snapshot of the current workflow.
    ///
    /// The catalog read lock is held for the complete count so all fields
    /// describe the same process-local state.
    #[must_use]
    pub fn counts(&self) -> PersistenceCatalogCounts {
        let candidates = self.candidates.read();
        let mut counts = PersistenceCatalogCounts {
            total: candidates.len(),
            ..PersistenceCatalogCounts::default()
        };

        for candidate in candidates.values() {
            match candidate.state() {
                PersistenceState::EphemeralUnvetted => {
                    counts.ephemeral_unvetted += 1;
                }
                PersistenceState::PendingReview => {
                    counts.pending_review += 1;
                }
                PersistenceState::VerifiedPersistent => {
                    counts.verified_persistent += 1;
                }
                PersistenceState::OperatorBlocked => {
                    counts.operator_blocked += 1;
                }
                PersistenceState::GlobalDenied => {
                    counts.global_denied += 1;
                }
                PersistenceState::OwnerTombstoned => {
                    counts.owner_tombstoned += 1;
                }
                PersistenceState::Quarantined => {
                    counts.quarantined += 1;
                }
                PersistenceState::PinnedByOperator => {
                    counts.pinned_by_operator += 1;
                }
            }

            if candidate.is_durable_storage_eligible() {
                counts.durable_storage_eligible += 1;
            }
        }

        counts
    }

    /// Return one exact candidate snapshot.
    #[must_use]
    pub fn status(&self, object: &B3Id) -> Option<PersistenceCandidate> {
        self.candidates.read().get(object).cloned()
    }

    /// Return a deterministic bounded list of objects awaiting a decision.
    ///
    /// Both `ephemeral_unvetted` and `pending_review` are included. This lets
    /// an operator discover newly registered amnesia-first objects without
    /// inventing a separate automatic approval path.
    pub fn list_pending(
        &self,
        limit: usize,
    ) -> Result<Vec<PersistenceCandidate>, PersistenceCatalogError> {
        if limit == 0 || limit > MAX_PERSISTENCE_REVIEW_ITEMS {
            return Err(PersistenceCatalogError::InvalidLimit {
                limit,
                max: MAX_PERSISTENCE_REVIEW_ITEMS,
            });
        }

        let candidates = self.candidates.read();

        Ok(candidates
            .values()
            .filter(|candidate| {
                matches!(
                    candidate.state(),
                    PersistenceState::EphemeralUnvetted | PersistenceState::PendingReview
                )
            })
            .take(limit)
            .cloned()
            .collect())
    }

    /// Move one exact amnesia-first object into `pending_review`.
    pub fn submit_for_review(
        &self,
        object: &B3Id,
    ) -> Result<PersistenceMutation, PersistenceCatalogError> {
        self.mutate(object, |candidate| {
            candidate
                .record
                .apply_review_action(PersistenceReviewAction::SubmitForReview)
                .map_err(Into::into)
        })
    }

    /// Apply canonical persistence approval.
    ///
    /// An ephemeral candidate traverses `pending_review` transactionally before
    /// policy evaluation. If policy rejects, neither transition is committed.
    pub fn approve(
        &self,
        object: &B3Id,
        policy: &PersistencePolicy,
        moderation: &ModerationPolicy,
        review: PersistenceReviewLevel,
    ) -> Result<PersistenceMutation, PersistenceCatalogError> {
        self.mutate(object, |candidate| {
            let mut changed = false;

            if candidate.state() == PersistenceState::EphemeralUnvetted {
                changed |= candidate
                    .record
                    .apply_review_action(PersistenceReviewAction::SubmitForReview)?;
            }

            changed |= candidate.record.apply_persistence_policy(
                policy,
                moderation,
                candidate.asset_kind,
                review,
                PersistenceIntent::Persist,
            )?;

            Ok(changed)
        })
    }

    /// Refuse persistence for one exact review candidate.
    ///
    /// An ephemeral candidate traverses `pending_review` transactionally before
    /// becoming `operator_blocked`. This does not alter moderation policy.
    pub fn reject(&self, object: &B3Id) -> Result<PersistenceMutation, PersistenceCatalogError> {
        self.mutate(object, |candidate| {
            let mut changed = false;

            if candidate.state() == PersistenceState::EphemeralUnvetted {
                changed |= candidate
                    .record
                    .apply_review_action(PersistenceReviewAction::SubmitForReview)?;
            }

            changed |= candidate
                .record
                .apply_review_action(PersistenceReviewAction::Reject)?;

            Ok(changed)
        })
    }

    /// Apply canonical operator-pinning policy.
    pub fn pin(
        &self,
        object: &B3Id,
        policy: &PersistencePolicy,
        moderation: &ModerationPolicy,
        review: PersistenceReviewLevel,
    ) -> Result<PersistenceMutation, PersistenceCatalogError> {
        self.mutate(object, |candidate| {
            candidate
                .record
                .apply_persistence_policy(
                    policy,
                    moderation,
                    candidate.asset_kind,
                    review,
                    PersistenceIntent::Pin,
                )
                .map_err(Into::into)
        })
    }

    /// Remove operator pinning while preserving verified eligibility.
    pub fn unpin(&self, object: &B3Id) -> Result<PersistenceMutation, PersistenceCatalogError> {
        self.mutate(object, |candidate| {
            candidate
                .record
                .apply_review_action(PersistenceReviewAction::Unpin)
                .map_err(Into::into)
        })
    }

    fn mutate<F>(
        &self,
        object: &B3Id,
        operation: F,
    ) -> Result<PersistenceMutation, PersistenceCatalogError>
    where
        F: FnOnce(&mut PersistenceCandidate) -> Result<bool, PersistenceCatalogError>,
    {
        let mut candidates = self.candidates.write();

        let mut candidate =
            candidates
                .get(object)
                .cloned()
                .ok_or_else(|| PersistenceCatalogError::NotFound {
                    object: object.clone(),
                })?;

        // Work on a clone so failed policy or lifecycle checks never leave a
        // partially applied transition in the shared catalog.
        let changed = operation(&mut candidate)?;

        if changed {
            candidates.insert(object.clone(), candidate.clone());
        }

        Ok(PersistenceMutation::new(candidate, changed))
    }
}
