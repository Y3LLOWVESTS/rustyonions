//! RO:WHAT — Focused tests for canonical Phase 11 persistence-state behavior.
//! RO:WHY — Prove amnesia-first defaults, policy-gated approval/pinning,
//! moderation-state projection, exact B3 binding, and strict wire labels.
//! RO:INTERACTS — svc_storage::persistence, ron_policy persistence/moderation,
//! and ron_proto::asset::AssetKind.
//! RO:INVARIANTS — no review skipping; no direct approval/pin bypass; canonical
//! moderation refusal defeats durable eligibility.
//! RO:SECURITY — state metadata makes no durable-backend, wallet, ledger,
//! reward, provider, or serve-authority claim.
//! RO:TEST — cargo test -p svc-storage --test persistence_state.

use ron_policy::{
    B3Id, ModerationPolicy, PersistenceIntent, PersistencePolicy, PersistenceReasonCode,
    PersistenceReviewLevel,
};
use ron_proto::asset::AssetKind;
use svc_storage::persistence::{
    PersistencePolicyTransitionError, PersistenceRecord, PersistenceReviewAction, PersistenceState,
};

const OBJECT: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

fn object() -> B3Id {
    OBJECT
        .parse()
        .expect("test object must be a canonical B3 identifier")
}

fn eligible_policy(operator_pinning: bool) -> PersistencePolicy {
    let mut policy = PersistencePolicy::default();

    assert!(policy.allow_asset_kind(AssetKind::Image));
    policy.set_operator_pinning_allowed(operator_pinning);

    policy
}

fn submit_for_review(record: &mut PersistenceRecord) {
    assert_eq!(
        record.apply_review_action(PersistenceReviewAction::SubmitForReview),
        Ok(true)
    );
}

fn approved_record(policy: &PersistencePolicy) -> PersistenceRecord {
    let mut record = PersistenceRecord::new(object());
    submit_for_review(&mut record);

    assert_eq!(
        record.apply_persistence_policy(
            policy,
            &ModerationPolicy::default(),
            AssetKind::Image,
            PersistenceReviewLevel::ModerationApproved,
            PersistenceIntent::Persist,
        ),
        Ok(true)
    );

    record
}

fn pinned_record(policy: &PersistencePolicy) -> PersistenceRecord {
    let mut record = approved_record(policy);

    assert_eq!(
        record.apply_persistence_policy(
            policy,
            &ModerationPolicy::default(),
            AssetKind::Image,
            PersistenceReviewLevel::ModerationApproved,
            PersistenceIntent::Pin,
        ),
        Ok(true)
    );

    record
}

fn moderation_for(reason: PersistenceReasonCode) -> ModerationPolicy {
    let mut moderation = ModerationPolicy::default();
    let inserted = match reason {
        PersistenceReasonCode::GlobalDeny => moderation.insert_global_deny(object()),
        PersistenceReasonCode::OwnerTombstone => moderation.insert_owner_tombstone(object()),
        PersistenceReasonCode::LocalBlock => moderation.insert_local_block(object()),
        PersistenceReasonCode::Quarantined => moderation.insert_quarantine(object()),
        other => panic!("test requires a moderation refusal reason, got {other:?}"),
    };

    assert!(inserted);
    moderation
}

#[test]
fn new_object_is_ephemeral_and_not_durable_storage_eligible() {
    let record = PersistenceRecord::new(object());

    assert_eq!(record.object().as_str(), OBJECT);
    assert_eq!(record.state(), PersistenceState::EphemeralUnvetted);
    assert!(!record.is_durable_storage_eligible());
}

#[test]
fn approval_and_pinning_require_canonical_policy_and_remain_idempotent() {
    let policy = eligible_policy(true);
    let moderation = ModerationPolicy::default();
    let mut record = PersistenceRecord::new(object());

    submit_for_review(&mut record);
    assert_eq!(record.state(), PersistenceState::PendingReview);

    assert_eq!(
        record.apply_review_action(PersistenceReviewAction::SubmitForReview),
        Ok(false),
        "repeated review submission must be idempotent"
    );

    let direct_approval = record
        .apply_review_action(PersistenceReviewAction::ApproveVerified)
        .expect_err("direct approval must not bypass canonical policy");

    assert_eq!(direct_approval.state(), PersistenceState::PendingReview);
    assert_eq!(
        direct_approval.action(),
        PersistenceReviewAction::ApproveVerified
    );

    assert_eq!(
        record.apply_persistence_policy(
            &policy,
            &moderation,
            AssetKind::Image,
            PersistenceReviewLevel::ModerationApproved,
            PersistenceIntent::Persist,
        ),
        Ok(true)
    );
    assert_eq!(record.state(), PersistenceState::VerifiedPersistent);
    assert!(record.is_durable_storage_eligible());

    assert_eq!(
        record.apply_persistence_policy(
            &policy,
            &moderation,
            AssetKind::Image,
            PersistenceReviewLevel::ModerationApproved,
            PersistenceIntent::Persist,
        ),
        Ok(false),
        "repeated policy approval must be idempotent"
    );

    let direct_pin = record
        .apply_review_action(PersistenceReviewAction::Pin)
        .expect_err("direct pinning must not bypass canonical policy");

    assert_eq!(direct_pin.state(), PersistenceState::VerifiedPersistent);
    assert_eq!(direct_pin.action(), PersistenceReviewAction::Pin);

    assert_eq!(
        record.apply_persistence_policy(
            &policy,
            &moderation,
            AssetKind::Image,
            PersistenceReviewLevel::ModerationApproved,
            PersistenceIntent::Pin,
        ),
        Ok(true)
    );
    assert_eq!(record.state(), PersistenceState::PinnedByOperator);

    assert_eq!(
        record.apply_persistence_policy(
            &policy,
            &moderation,
            AssetKind::Image,
            PersistenceReviewLevel::ModerationApproved,
            PersistenceIntent::Pin,
        ),
        Ok(false),
        "repeated policy pin must be idempotent"
    );

    assert_eq!(
        record.apply_review_action(PersistenceReviewAction::Unpin),
        Ok(true)
    );
    assert_eq!(record.state(), PersistenceState::VerifiedPersistent);

    assert_eq!(
        record.apply_review_action(PersistenceReviewAction::Unpin),
        Ok(false),
        "repeated unpin must be idempotent"
    );
}

#[test]
fn approval_and_pinning_cannot_skip_required_review_state() {
    let policy = eligible_policy(true);
    let moderation = ModerationPolicy::default();
    let mut record = PersistenceRecord::new(object());

    let approval_error = record
        .apply_persistence_policy(
            &policy,
            &moderation,
            AssetKind::Image,
            PersistenceReviewLevel::ModerationApproved,
            PersistenceIntent::Persist,
        )
        .expect_err("policy approval must not skip pending review");

    assert_eq!(
        approval_error,
        PersistencePolicyTransitionError::InvalidState {
            state: PersistenceState::EphemeralUnvetted,
            intent: PersistenceIntent::Persist,
        }
    );
    assert_eq!(record.state(), PersistenceState::EphemeralUnvetted);

    let pin_error = record
        .apply_persistence_policy(
            &policy,
            &moderation,
            AssetKind::Image,
            PersistenceReviewLevel::ModerationApproved,
            PersistenceIntent::Pin,
        )
        .expect_err("policy pin must not skip verification");

    assert_eq!(
        pin_error,
        PersistencePolicyTransitionError::InvalidState {
            state: PersistenceState::EphemeralUnvetted,
            intent: PersistenceIntent::Pin,
        }
    );

    submit_for_review(&mut record);

    let pending_pin_error = record
        .apply_persistence_policy(
            &policy,
            &moderation,
            AssetKind::Image,
            PersistenceReviewLevel::ModerationApproved,
            PersistenceIntent::Pin,
        )
        .expect_err("pending review must not pin before approval");

    assert_eq!(
        pending_pin_error,
        PersistencePolicyTransitionError::InvalidState {
            state: PersistenceState::PendingReview,
            intent: PersistenceIntent::Pin,
        }
    );
    assert_eq!(record.state(), PersistenceState::PendingReview);
}

#[test]
fn moderation_refusals_project_to_non_durable_storage_states() {
    let policy = eligible_policy(true);

    for (reason, expected_state) in [
        (
            PersistenceReasonCode::GlobalDeny,
            PersistenceState::GlobalDenied,
        ),
        (
            PersistenceReasonCode::OwnerTombstone,
            PersistenceState::OwnerTombstoned,
        ),
        (
            PersistenceReasonCode::LocalBlock,
            PersistenceState::OperatorBlocked,
        ),
        (
            PersistenceReasonCode::Quarantined,
            PersistenceState::Quarantined,
        ),
    ] {
        let moderation = moderation_for(reason);
        let mut record = pinned_record(&policy);

        assert!(record.is_durable_storage_eligible());

        assert_eq!(
            record.apply_persistence_policy(
                &policy,
                &moderation,
                AssetKind::Image,
                PersistenceReviewLevel::ModerationApproved,
                PersistenceIntent::Pin,
            ),
            Ok(true)
        );

        assert_eq!(record.state(), expected_state);
        assert!(
            !record.is_durable_storage_eligible(),
            "{} must revoke durable-storage eligibility",
            reason.as_str()
        );

        assert_eq!(
            record.apply_persistence_policy(
                &policy,
                &moderation,
                AssetKind::Image,
                PersistenceReviewLevel::ModerationApproved,
                PersistenceIntent::Pin,
            ),
            Ok(false),
            "reapplying the same canonical refusal must be idempotent"
        );
    }
}

#[test]
fn local_block_projects_pending_review_to_operator_blocked() {
    let policy = eligible_policy(true);
    let moderation = moderation_for(PersistenceReasonCode::LocalBlock);
    let mut record = PersistenceRecord::new(object());

    submit_for_review(&mut record);

    assert_eq!(
        record.apply_persistence_policy(
            &policy,
            &moderation,
            AssetKind::Image,
            PersistenceReviewLevel::ModerationApproved,
            PersistenceIntent::Persist,
        ),
        Ok(true)
    );
    assert_eq!(record.state(), PersistenceState::OperatorBlocked);
    assert!(!record.is_durable_storage_eligible());
}

#[test]
fn canonical_moderation_precedence_projects_global_deny() {
    let policy = eligible_policy(true);
    let mut moderation = ModerationPolicy::default();
    let mut record = pinned_record(&policy);

    assert!(moderation.insert_local_allow(object()));
    assert!(moderation.insert_quarantine(object()));
    assert!(moderation.insert_local_block(object()));
    assert!(moderation.insert_owner_tombstone(object()));
    assert!(moderation.insert_global_deny(object()));

    assert_eq!(
        record.apply_persistence_policy(
            &policy,
            &moderation,
            AssetKind::Image,
            PersistenceReviewLevel::ModerationApproved,
            PersistenceIntent::Pin,
        ),
        Ok(true)
    );

    assert_eq!(record.state(), PersistenceState::GlobalDenied);
    assert!(!record.is_durable_storage_eligible());
}

#[test]
fn category_review_and_pin_refusals_leave_state_unchanged() {
    let moderation = ModerationPolicy::default();

    let mut category_record = PersistenceRecord::new(object());
    submit_for_review(&mut category_record);

    let category_error = category_record
        .apply_persistence_policy(
            &PersistencePolicy::default(),
            &moderation,
            AssetKind::Image,
            PersistenceReviewLevel::ModerationApproved,
            PersistenceIntent::Persist,
        )
        .expect_err("an unconfigured asset category must fail closed");

    assert_eq!(
        category_error,
        PersistencePolicyTransitionError::Ineligible {
            state: PersistenceState::PendingReview,
            intent: PersistenceIntent::Persist,
            reason: PersistenceReasonCode::AssetKindNotAllowed,
        }
    );
    assert_eq!(category_record.state(), PersistenceState::PendingReview);

    let policy = eligible_policy(false);
    let mut review_record = PersistenceRecord::new(object());
    submit_for_review(&mut review_record);

    let review_error = review_record
        .apply_persistence_policy(
            &policy,
            &moderation,
            AssetKind::Image,
            PersistenceReviewLevel::IntegrityVerified,
            PersistenceIntent::Persist,
        )
        .expect_err("insufficient review strength must fail closed");

    assert_eq!(
        review_error,
        PersistencePolicyTransitionError::Ineligible {
            state: PersistenceState::PendingReview,
            intent: PersistenceIntent::Persist,
            reason: PersistenceReasonCode::ReviewThresholdNotMet,
        }
    );
    assert_eq!(review_record.state(), PersistenceState::PendingReview);

    let mut pin_record = approved_record(&policy);

    let pin_error = pin_record
        .apply_persistence_policy(
            &policy,
            &moderation,
            AssetKind::Image,
            PersistenceReviewLevel::ModerationApproved,
            PersistenceIntent::Pin,
        )
        .expect_err("disabled operator pinning must fail closed");

    assert_eq!(
        pin_error,
        PersistencePolicyTransitionError::Ineligible {
            state: PersistenceState::VerifiedPersistent,
            intent: PersistenceIntent::Pin,
            reason: PersistenceReasonCode::OperatorPinningDisabled,
        }
    );
    assert_eq!(pin_record.state(), PersistenceState::VerifiedPersistent);
}

#[test]
fn operator_reject_requires_review_and_is_idempotent() {
    let mut record = PersistenceRecord::new(object());

    let direct_reject = record
        .apply_review_action(PersistenceReviewAction::Reject)
        .expect_err("ephemeral object must enter pending review before rejection");

    assert_eq!(direct_reject.state(), PersistenceState::EphemeralUnvetted);
    assert_eq!(direct_reject.action(), PersistenceReviewAction::Reject);

    submit_for_review(&mut record);

    assert_eq!(
        record.apply_review_action(PersistenceReviewAction::Reject),
        Ok(true)
    );
    assert_eq!(record.state(), PersistenceState::OperatorBlocked);
    assert!(!record.is_durable_storage_eligible());

    assert_eq!(
        record.apply_review_action(PersistenceReviewAction::Reject),
        Ok(false),
        "repeated operator rejection must be idempotent"
    );
}

#[test]
fn only_verified_and_pinned_states_are_durable_storage_eligible() {
    for state in [
        PersistenceState::EphemeralUnvetted,
        PersistenceState::PendingReview,
        PersistenceState::OperatorBlocked,
        PersistenceState::GlobalDenied,
        PersistenceState::OwnerTombstoned,
        PersistenceState::Quarantined,
    ] {
        assert!(
            !state.is_durable_storage_eligible(),
            "{} must not permit durable storage",
            state.as_str()
        );
    }

    for state in [
        PersistenceState::VerifiedPersistent,
        PersistenceState::PinnedByOperator,
    ] {
        assert!(
            state.is_durable_storage_eligible(),
            "{} must be eligible for a later durable write",
            state.as_str()
        );
    }
}

#[test]
fn persistence_state_and_action_labels_are_stable() {
    let states = [
        (PersistenceState::EphemeralUnvetted, "ephemeral_unvetted"),
        (PersistenceState::PendingReview, "pending_review"),
        (PersistenceState::VerifiedPersistent, "verified_persistent"),
        (PersistenceState::OperatorBlocked, "operator_blocked"),
        (PersistenceState::GlobalDenied, "global_denied"),
        (PersistenceState::OwnerTombstoned, "owner_tombstoned"),
        (PersistenceState::Quarantined, "quarantined"),
        (PersistenceState::PinnedByOperator, "pinned_by_operator"),
    ];

    for (state, expected) in states {
        assert_eq!(state.as_str(), expected);
        assert_eq!(
            serde_json::to_string(&state).expect("state should serialize"),
            format!("\"{expected}\"")
        );

        let decoded: PersistenceState = serde_json::from_str(&format!("\"{expected}\""))
            .expect("canonical state label should deserialize");

        assert_eq!(decoded, state);
    }

    let actions = [
        (
            PersistenceReviewAction::SubmitForReview,
            "submit_for_review",
        ),
        (PersistenceReviewAction::Reject, "reject"),
        (PersistenceReviewAction::ApproveVerified, "approve_verified"),
        (PersistenceReviewAction::Pin, "pin"),
        (PersistenceReviewAction::Unpin, "unpin"),
    ];

    for (action, expected) in actions {
        assert_eq!(action.as_str(), expected);
        assert_eq!(
            serde_json::to_string(&action).expect("action should serialize"),
            format!("\"{expected}\"")
        );
    }
}

#[test]
fn persistence_record_json_is_strict_and_defaults_to_ephemeral() {
    let record = PersistenceRecord::new(object());
    let encoded = serde_json::to_string(&record).expect("record should serialize");

    assert_eq!(
        encoded,
        format!(r#"{{"object":"{OBJECT}","state":"ephemeral_unvetted"}}"#)
    );

    let decoded: PersistenceRecord =
        serde_json::from_str(&encoded).expect("record should deserialize");

    assert_eq!(decoded, record);

    let state_omitted = format!(r#"{{"object":"{OBJECT}"}}"#);
    let defaulted: PersistenceRecord = serde_json::from_str(&state_omitted)
        .expect("missing state should use the amnesia-first default");

    assert_eq!(defaulted.state(), PersistenceState::EphemeralUnvetted);
    assert!(!defaulted.is_durable_storage_eligible());

    let unknown_field = format!(
        r#"{{
            "object":"{OBJECT}",
            "state":"ephemeral_unvetted",
            "unexpected":true
        }}"#
    );

    assert!(
        serde_json::from_str::<PersistenceRecord>(&unknown_field).is_err(),
        "unknown persistence record fields must reject"
    );

    let invalid_object = r#"{
        "object":"b3:NOT_CANONICAL",
        "state":"ephemeral_unvetted"
    }"#;

    assert!(
        serde_json::from_str::<PersistenceRecord>(invalid_object).is_err(),
        "non-canonical B3 object identifiers must reject"
    );
}
