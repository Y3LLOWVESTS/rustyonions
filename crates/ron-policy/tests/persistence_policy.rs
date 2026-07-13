//! RO:WHAT — Focused tests for declarative Phase 11 persistence eligibility.
//! RO:WHY — Prove fail-closed defaults, canonical moderation precedence,
//! asset-category policy, review thresholds, and operator-pin gating.
//! RO:INTERACTS — ron_policy persistence/moderation and ron_proto AssetKind.
//! RO:INVARIANTS — refusal states defeat approval; policy performs no storage mutation.
//! RO:TEST — cargo test -p ron-policy --test persistence_policy.

use ron_policy::{
    B3Id, ModerationPolicy, PersistenceEffect, PersistenceIntent, PersistencePolicy,
    PersistenceReasonCode, PersistenceReviewLevel,
};
use ron_proto::asset::AssetKind;

const OBJECT: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

fn object() -> B3Id {
    OBJECT
        .parse()
        .expect("test object must be a canonical B3 identifier")
}

fn image_policy() -> PersistencePolicy {
    let mut policy = PersistencePolicy::default();

    assert!(
        policy.allow_asset_kind(AssetKind::Image),
        "first category insertion must change policy"
    );

    policy
}

fn evaluate(
    policy: &PersistencePolicy,
    moderation: &ModerationPolicy,
    review: PersistenceReviewLevel,
    intent: PersistenceIntent,
) -> ron_policy::PersistenceDecision {
    policy.evaluate(moderation, &object(), AssetKind::Image, review, intent)
}

#[test]
fn default_policy_is_fail_closed() {
    let policy = PersistencePolicy::default();
    let moderation = ModerationPolicy::default();

    assert_eq!(
        policy.required_review(),
        PersistenceReviewLevel::ModerationApproved
    );
    assert_eq!(policy.allowed_asset_kind_count(), 0);
    assert!(!policy.operator_pinning_allowed());

    let decision = evaluate(
        &policy,
        &moderation,
        PersistenceReviewLevel::ModerationApproved,
        PersistenceIntent::Persist,
    );

    assert_eq!(decision.effect, PersistenceEffect::Ineligible);
    assert_eq!(decision.reason, PersistenceReasonCode::AssetKindNotAllowed);
    assert!(!decision.permits_persistence());
}

#[test]
fn enabled_asset_requires_configured_review_threshold() {
    let policy = image_policy();
    let moderation = ModerationPolicy::default();

    let insufficient = evaluate(
        &policy,
        &moderation,
        PersistenceReviewLevel::IntegrityVerified,
        PersistenceIntent::Persist,
    );

    assert_eq!(insufficient.effect, PersistenceEffect::Ineligible);
    assert_eq!(
        insufficient.reason,
        PersistenceReasonCode::ReviewThresholdNotMet
    );

    let approved = evaluate(
        &policy,
        &moderation,
        PersistenceReviewLevel::ModerationApproved,
        PersistenceIntent::Persist,
    );

    assert_eq!(approved.effect, PersistenceEffect::Eligible);
    assert_eq!(approved.reason, PersistenceReasonCode::Eligible);
    assert!(approved.permits_persistence());
}

#[test]
fn review_threshold_is_declarative_and_configurable() {
    let mut policy = image_policy();
    let moderation = ModerationPolicy::default();

    policy.set_required_review(PersistenceReviewLevel::IntegrityVerified);

    assert_eq!(
        policy.required_review(),
        PersistenceReviewLevel::IntegrityVerified
    );

    let verified = evaluate(
        &policy,
        &moderation,
        PersistenceReviewLevel::IntegrityVerified,
        PersistenceIntent::Persist,
    );

    assert!(verified.permits_persistence());

    let unreviewed = evaluate(
        &policy,
        &moderation,
        PersistenceReviewLevel::Unreviewed,
        PersistenceIntent::Persist,
    );

    assert_eq!(
        unreviewed.reason,
        PersistenceReasonCode::ReviewThresholdNotMet
    );
    assert!(!unreviewed.permits_persistence());
}

#[test]
fn moderation_refusal_states_defeat_persistence_approval() {
    let policy = image_policy();
    let object = object();

    let mut global_deny = ModerationPolicy::default();
    assert!(global_deny.insert_global_deny(object.clone()));

    let global_decision = policy.evaluate(
        &global_deny,
        &object,
        AssetKind::Image,
        PersistenceReviewLevel::ModerationApproved,
        PersistenceIntent::Persist,
    );

    assert_eq!(global_decision.reason, PersistenceReasonCode::GlobalDeny);

    let mut tombstone = ModerationPolicy::default();
    assert!(tombstone.insert_owner_tombstone(object.clone()));

    let tombstone_decision = policy.evaluate(
        &tombstone,
        &object,
        AssetKind::Image,
        PersistenceReviewLevel::ModerationApproved,
        PersistenceIntent::Persist,
    );

    assert_eq!(
        tombstone_decision.reason,
        PersistenceReasonCode::OwnerTombstone
    );

    let mut local_block = ModerationPolicy::default();
    assert!(local_block.insert_local_block(object.clone()));

    let block_decision = policy.evaluate(
        &local_block,
        &object,
        AssetKind::Image,
        PersistenceReviewLevel::ModerationApproved,
        PersistenceIntent::Persist,
    );

    assert_eq!(block_decision.reason, PersistenceReasonCode::LocalBlock);

    let mut quarantine = ModerationPolicy::default();
    assert!(quarantine.insert_quarantine(object.clone()));

    let quarantine_decision = policy.evaluate(
        &quarantine,
        &object,
        AssetKind::Image,
        PersistenceReviewLevel::ModerationApproved,
        PersistenceIntent::Persist,
    );

    assert_eq!(
        quarantine_decision.reason,
        PersistenceReasonCode::Quarantined
    );

    for decision in [
        global_decision,
        tombstone_decision,
        block_decision,
        quarantine_decision,
    ] {
        assert_eq!(decision.effect, PersistenceEffect::Ineligible);
        assert!(!decision.permits_persistence());
    }
}

#[test]
fn strongest_moderation_state_keeps_canonical_precedence() {
    let mut policy = image_policy();
    policy.set_operator_pinning_allowed(true);

    let object = object();
    let mut moderation = ModerationPolicy::default();

    assert!(moderation.insert_local_allow(object.clone()));
    assert!(moderation.insert_quarantine(object.clone()));
    assert!(moderation.insert_local_block(object.clone()));
    assert!(moderation.insert_owner_tombstone(object.clone()));
    assert!(moderation.insert_global_deny(object.clone()));

    let decision = policy.evaluate(
        &moderation,
        &object,
        AssetKind::Image,
        PersistenceReviewLevel::ModerationApproved,
        PersistenceIntent::Pin,
    );

    assert_eq!(decision.effect, PersistenceEffect::Ineligible);
    assert_eq!(decision.reason, PersistenceReasonCode::GlobalDeny);
}

#[test]
fn asset_category_policy_is_exact() {
    let mut policy = image_policy();
    let moderation = ModerationPolicy::default();

    assert!(policy.permits_asset_kind(AssetKind::Image));
    assert!(!policy.permits_asset_kind(AssetKind::Video));

    let video_decision = policy.evaluate(
        &moderation,
        &object(),
        AssetKind::Video,
        PersistenceReviewLevel::ModerationApproved,
        PersistenceIntent::Persist,
    );

    assert_eq!(
        video_decision.reason,
        PersistenceReasonCode::AssetKindNotAllowed
    );

    assert!(policy.disallow_asset_kind(&AssetKind::Image));
    assert!(!policy.permits_asset_kind(AssetKind::Image));
    assert_eq!(policy.allowed_asset_kind_count(), 0);
}

#[test]
fn operator_pinning_requires_explicit_policy_enablement() {
    let mut policy = image_policy();
    let moderation = ModerationPolicy::default();

    let disabled = evaluate(
        &policy,
        &moderation,
        PersistenceReviewLevel::ModerationApproved,
        PersistenceIntent::Pin,
    );

    assert_eq!(
        disabled.reason,
        PersistenceReasonCode::OperatorPinningDisabled
    );
    assert!(!disabled.permits_persistence());

    policy.set_operator_pinning_allowed(true);

    let enabled = evaluate(
        &policy,
        &moderation,
        PersistenceReviewLevel::ModerationApproved,
        PersistenceIntent::Pin,
    );

    assert_eq!(enabled.reason, PersistenceReasonCode::Eligible);
    assert!(enabled.permits_persistence());
}

#[test]
fn persistence_policy_wire_shape_is_strict_and_stable() {
    let mut policy = image_policy();
    policy.set_operator_pinning_allowed(true);

    let encoded = serde_json::to_string(&policy).expect("policy should serialize");

    assert_eq!(
        encoded,
        r#"{"required_review":"moderation_approved","allowed_asset_kinds":["image"],"allow_operator_pinning":true}"#
    );

    let decoded: PersistencePolicy =
        serde_json::from_str(&encoded).expect("policy should deserialize");

    assert_eq!(decoded, policy);

    let defaulted: PersistencePolicy =
        serde_json::from_str("{}").expect("empty policy should default");

    assert_eq!(
        defaulted.required_review(),
        PersistenceReviewLevel::ModerationApproved
    );
    assert_eq!(defaulted.allowed_asset_kind_count(), 0);
    assert!(!defaulted.operator_pinning_allowed());

    let unknown_field = r#"{
        "required_review":"moderation_approved",
        "allowed_asset_kinds":["image"],
        "allow_operator_pinning":false,
        "unexpected":true
    }"#;

    assert!(
        serde_json::from_str::<PersistencePolicy>(unknown_field).is_err(),
        "unknown persistence-policy fields must reject"
    );

    let labels = [
        (PersistenceReviewLevel::Unreviewed.as_str(), "unreviewed"),
        (
            PersistenceReviewLevel::IntegrityVerified.as_str(),
            "integrity_verified",
        ),
        (
            PersistenceReviewLevel::ModerationApproved.as_str(),
            "moderation_approved",
        ),
        (PersistenceIntent::Persist.as_str(), "persist"),
        (PersistenceIntent::Pin.as_str(), "pin"),
        (
            PersistenceReasonCode::AssetKindNotAllowed.as_str(),
            "asset_kind_not_allowed",
        ),
        (
            PersistenceReasonCode::ReviewThresholdNotMet.as_str(),
            "review_threshold_not_met",
        ),
        (
            PersistenceReasonCode::OperatorPinningDisabled.as_str(),
            "operator_pinning_disabled",
        ),
    ];

    for (actual, expected) in labels {
        assert_eq!(actual, expected);
    }
}
