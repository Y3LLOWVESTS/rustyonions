//! RO:WHAT — Focused tests for the Phase 11 operator persistence catalog.
//! RO:WHY — Operator API and CLI adapters require one real shared state owner.
//! RO:INTERACTS — PersistenceCatalog, PersistenceRecord, ron-policy, moderation,
//! and canonical AssetKind.
//! RO:INVARIANTS — deterministic bounded lists; no duplicate reset; policy
//! gates approval/pinning; failed mutations do not partially commit.
//! RO:SECURITY — metadata only; no durable-byte, provider, reward, wallet, or
//! ledger mutation.
//! RO:TEST — cargo test -p svc-storage --test persistence_catalog.

use ron_policy::{
    B3Id, ModerationPolicy, PersistenceIntent, PersistencePolicy, PersistenceReasonCode,
    PersistenceReviewLevel,
};
use ron_proto::asset::AssetKind;
use svc_storage::{
    persistence::{PersistencePolicyTransitionError, PersistenceState},
    persistence_catalog::{
        PersistenceCatalog, PersistenceCatalogError, MAX_PERSISTENCE_REVIEW_ITEMS,
    },
};

fn object(fill: char) -> B3Id {
    format!("b3:{}", fill.to_string().repeat(64))
        .parse()
        .expect("test object must be an exact canonical B3 identifier")
}

fn eligible_policy(operator_pinning: bool) -> PersistencePolicy {
    let mut policy = PersistencePolicy::default();

    assert!(policy.allow_asset_kind(AssetKind::Image));
    policy.set_operator_pinning_allowed(operator_pinning);

    policy
}

#[test]
fn registration_is_ephemeral_idempotent_and_does_not_reset_category() {
    let catalog = PersistenceCatalog::new();
    let object = object('a');

    assert!(catalog.register(object.clone(), AssetKind::Image));
    assert!(!catalog.register(object.clone(), AssetKind::Video));
    assert_eq!(catalog.candidate_count(), 1);

    let candidate = catalog.status(&object).expect("candidate should exist");

    assert_eq!(candidate.object(), &object);
    assert_eq!(candidate.asset_kind(), AssetKind::Image);
    assert_eq!(candidate.state(), PersistenceState::EphemeralUnvetted);
    assert!(!candidate.is_durable_storage_eligible());
}

#[test]
fn pending_list_is_deterministic_bounded_and_excludes_decided_objects() {
    let catalog = PersistenceCatalog::new();
    let first = object('a');
    let second = object('b');
    let third = object('c');

    assert!(catalog.register(third.clone(), AssetKind::Image));
    assert!(catalog.register(first.clone(), AssetKind::Image));
    assert!(catalog.register(second.clone(), AssetKind::Image));

    catalog
        .submit_for_review(&second)
        .expect("submission should succeed");

    let pending = catalog
        .list_pending(2)
        .expect("bounded list should succeed");

    assert_eq!(pending.len(), 2);
    assert_eq!(pending[0].object(), &first);
    assert_eq!(pending[1].object(), &second);
    assert_eq!(pending[1].state(), PersistenceState::PendingReview);

    catalog.reject(&first).expect("rejection should succeed");

    let remaining = catalog
        .list_pending(MAX_PERSISTENCE_REVIEW_ITEMS)
        .expect("full bounded list should succeed");

    assert_eq!(remaining.len(), 2);
    assert_eq!(remaining[0].object(), &second);
    assert_eq!(remaining[1].object(), &third);
}

#[test]
fn approval_uses_canonical_policy_and_remains_idempotent() {
    let catalog = PersistenceCatalog::new();
    let object = object('a');
    let policy = eligible_policy(true);
    let moderation = ModerationPolicy::default();

    assert!(catalog.register(object.clone(), AssetKind::Image));

    let approved = catalog
        .approve(
            &object,
            &policy,
            &moderation,
            PersistenceReviewLevel::ModerationApproved,
        )
        .expect("eligible object should approve");

    assert!(approved.changed());
    assert_eq!(
        approved.candidate().state(),
        PersistenceState::VerifiedPersistent
    );
    assert!(approved.candidate().is_durable_storage_eligible());

    let repeated = catalog
        .approve(
            &object,
            &policy,
            &moderation,
            PersistenceReviewLevel::ModerationApproved,
        )
        .expect("repeated approval should remain valid");

    assert!(!repeated.changed());
    assert_eq!(
        repeated.candidate().state(),
        PersistenceState::VerifiedPersistent
    );
}

#[test]
fn failed_approval_does_not_partially_commit_pending_review() {
    let catalog = PersistenceCatalog::new();
    let object = object('a');

    assert!(catalog.register(object.clone(), AssetKind::Image));

    let error = catalog
        .approve(
            &object,
            &PersistencePolicy::default(),
            &ModerationPolicy::default(),
            PersistenceReviewLevel::ModerationApproved,
        )
        .expect_err("unconfigured category must fail closed");

    assert_eq!(
        error,
        PersistenceCatalogError::Policy(PersistencePolicyTransitionError::Ineligible {
            state: PersistenceState::PendingReview,
            intent: PersistenceIntent::Persist,
            reason: PersistenceReasonCode::AssetKindNotAllowed,
        })
    );

    assert_eq!(
        catalog.status(&object).unwrap().state(),
        PersistenceState::EphemeralUnvetted,
        "failed transactional approval must not commit its intermediate state"
    );
}

#[test]
fn rejection_moves_review_candidate_to_operator_blocked() {
    let catalog = PersistenceCatalog::new();
    let object = object('a');

    assert!(catalog.register(object.clone(), AssetKind::Image));

    let rejected = catalog.reject(&object).expect("rejection should succeed");

    assert!(rejected.changed());
    assert_eq!(
        rejected.candidate().state(),
        PersistenceState::OperatorBlocked
    );
    assert!(!rejected.candidate().is_durable_storage_eligible());

    let repeated = catalog
        .reject(&object)
        .expect("repeated rejection should be idempotent");

    assert!(!repeated.changed());
}

#[test]
fn pin_requires_approval_and_unpin_preserves_verified_eligibility() {
    let catalog = PersistenceCatalog::new();
    let object = object('a');
    let policy = eligible_policy(true);
    let moderation = ModerationPolicy::default();

    assert!(catalog.register(object.clone(), AssetKind::Image));

    let early_pin = catalog
        .pin(
            &object,
            &policy,
            &moderation,
            PersistenceReviewLevel::ModerationApproved,
        )
        .expect_err("ephemeral object must not pin");

    assert_eq!(
        early_pin,
        PersistenceCatalogError::Policy(PersistencePolicyTransitionError::InvalidState {
            state: PersistenceState::EphemeralUnvetted,
            intent: PersistenceIntent::Pin,
        })
    );

    catalog
        .approve(
            &object,
            &policy,
            &moderation,
            PersistenceReviewLevel::ModerationApproved,
        )
        .expect("approval should succeed");

    let pinned = catalog
        .pin(
            &object,
            &policy,
            &moderation,
            PersistenceReviewLevel::ModerationApproved,
        )
        .expect("verified object should pin");

    assert!(pinned.changed());
    assert_eq!(
        pinned.candidate().state(),
        PersistenceState::PinnedByOperator
    );

    let unpinned = catalog.unpin(&object).expect("unpin should succeed");

    assert!(unpinned.changed());
    assert_eq!(
        unpinned.candidate().state(),
        PersistenceState::VerifiedPersistent
    );
    assert!(unpinned.candidate().is_durable_storage_eligible());
}

#[test]
fn canonical_moderation_refusal_overrides_approval() {
    let catalog = PersistenceCatalog::new();
    let object = object('a');
    let policy = eligible_policy(true);
    let mut moderation = ModerationPolicy::default();

    assert!(catalog.register(object.clone(), AssetKind::Image));
    assert!(moderation.insert_global_deny(object.clone()));

    let outcome = catalog
        .approve(
            &object,
            &policy,
            &moderation,
            PersistenceReviewLevel::ModerationApproved,
        )
        .expect("moderation refusal should project into canonical state");

    assert!(outcome.changed());
    assert_eq!(outcome.candidate().state(), PersistenceState::GlobalDenied);
    assert!(!outcome.candidate().is_durable_storage_eligible());
}

#[test]
fn missing_objects_and_invalid_list_limits_fail_closed() {
    let catalog = PersistenceCatalog::new();
    let missing = object('a');

    assert!(catalog.status(&missing).is_none());

    assert_eq!(
        catalog.reject(&missing),
        Err(PersistenceCatalogError::NotFound {
            object: missing.clone(),
        })
    );

    assert_eq!(
        catalog.list_pending(0),
        Err(PersistenceCatalogError::InvalidLimit {
            limit: 0,
            max: MAX_PERSISTENCE_REVIEW_ITEMS,
        })
    );

    assert_eq!(
        catalog.list_pending(MAX_PERSISTENCE_REVIEW_ITEMS + 1),
        Err(PersistenceCatalogError::InvalidLimit {
            limit: MAX_PERSISTENCE_REVIEW_ITEMS + 1,
            max: MAX_PERSISTENCE_REVIEW_ITEMS,
        })
    );
}
