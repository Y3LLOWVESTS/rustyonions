//! RO:WHAT — Phase 10A exact-b3 moderation model tests.
//! RO:WHY — Lock deterministic block/allow/tombstone/quarantine precedence before runtime wiring.
//! RO:INTERACTS — ron_policy::moderation.
//! RO:INVARIANTS — exact full-hash matching; local allow cannot override stronger refusal states.
//! RO:SECURITY — policy decision only; no storage, provider, reward, wallet, or ledger mutation.
//! RO:TEST — cargo test -p ron-policy --test moderation_policy.

use ron_policy::{
    B3Id, ModerationEffect, ModerationPolicy, ModerationPolicyCompositionError,
    ModerationReasonCode,
};

const OBJECT_A: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OBJECT_B: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab";
const OBJECT_C: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const OBJECT_D: &str = "b3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const OBJECT_E: &str = "b3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

fn id(value: &str) -> B3Id {
    value.parse().expect("test b3 identifier must parse")
}

#[test]
fn strict_b3_identifier_rejects_noncanonical_values() {
    assert!(OBJECT_A.parse::<B3Id>().is_ok());

    for invalid in [
        "",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "b3:abc",
        "b3:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "b3:gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg",
        "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ] {
        assert!(
            invalid.parse::<B3Id>().is_err(),
            "noncanonical identifier should reject: {invalid}"
        );
    }
}

#[test]
fn no_rule_and_local_allow_permit_serving() {
    let mut policy = ModerationPolicy::default();
    let object = id(OBJECT_A);

    let no_rule = policy.evaluate(&object);
    assert_eq!(no_rule.effect, ModerationEffect::Serve);
    assert_eq!(no_rule.reason, ModerationReasonCode::NoRule);
    assert!(no_rule.permits_serve());

    assert!(policy.insert_local_allow(object.clone()));

    let allowed = policy.evaluate(&object);
    assert_eq!(allowed.effect, ModerationEffect::Serve);
    assert_eq!(allowed.reason, ModerationReasonCode::LocalAllow);
    assert!(allowed.permits_serve());
}

#[test]
fn refusal_states_have_fixed_precedence_over_local_allow() {
    let mut policy = ModerationPolicy::default();

    let global = id(OBJECT_A);
    let tombstoned = id(OBJECT_B);
    let blocked = id(OBJECT_C);
    let quarantined = id(OBJECT_D);

    assert!(policy.insert_local_allow(global.clone()));
    assert!(policy.insert_global_deny(global.clone()));

    assert!(policy.insert_local_allow(tombstoned.clone()));
    assert!(policy.insert_owner_tombstone(tombstoned.clone()));

    assert!(policy.insert_local_allow(blocked.clone()));
    assert!(policy.insert_local_block(blocked.clone()));

    assert!(policy.insert_local_allow(quarantined.clone()));
    assert!(policy.insert_quarantine(quarantined.clone()));

    let global_decision = policy.evaluate(&global);
    assert_eq!(global_decision.reason, ModerationReasonCode::GlobalDeny);
    assert_eq!(global_decision.effect, ModerationEffect::Refuse);

    let tombstone_decision = policy.evaluate(&tombstoned);
    assert_eq!(
        tombstone_decision.reason,
        ModerationReasonCode::OwnerTombstone
    );
    assert_eq!(tombstone_decision.effect, ModerationEffect::Refuse);

    let block_decision = policy.evaluate(&blocked);
    assert_eq!(block_decision.reason, ModerationReasonCode::LocalBlock);
    assert_eq!(block_decision.effect, ModerationEffect::Refuse);

    let quarantine_decision = policy.evaluate(&quarantined);
    assert_eq!(
        quarantine_decision.reason,
        ModerationReasonCode::Quarantined
    );
    assert_eq!(quarantine_decision.effect, ModerationEffect::Quarantine);

    assert!(!global_decision.permits_serve());
    assert!(!tombstone_decision.permits_serve());
    assert!(!block_decision.permits_serve());
    assert!(!quarantine_decision.permits_serve());
}

#[test]
fn strongest_matching_reason_wins_deterministically() {
    let object = id(OBJECT_A);
    let mut policy = ModerationPolicy::default();

    assert!(policy.insert_quarantine(object.clone()));
    assert!(policy.insert_local_block(object.clone()));
    assert!(policy.insert_owner_tombstone(object.clone()));
    assert!(policy.insert_global_deny(object.clone()));

    let decision = policy.evaluate(&object);

    assert_eq!(decision.effect, ModerationEffect::Refuse);
    assert_eq!(decision.reason, ModerationReasonCode::GlobalDeny);
}

#[test]
fn policy_false_positive_requires_exact_lookup() {
    let blocked = id(OBJECT_A);
    let almost_same = id(OBJECT_B);
    let unrelated = id(OBJECT_E);

    let mut policy = ModerationPolicy::default();
    assert!(policy.insert_local_block(blocked.clone()));

    assert_eq!(
        policy.evaluate(&blocked).reason,
        ModerationReasonCode::LocalBlock
    );

    for object in [&almost_same, &unrelated] {
        let decision = policy.evaluate(object);

        assert_eq!(decision.effect, ModerationEffect::Serve);
        assert_eq!(decision.reason, ModerationReasonCode::NoRule);
    }
}

#[test]
fn local_operator_states_can_be_removed_without_touching_stronger_sets() {
    let blocked = id(OBJECT_A);
    let allowed = id(OBJECT_B);
    let quarantined = id(OBJECT_C);

    let mut policy = ModerationPolicy::default();

    assert!(policy.insert_local_block(blocked.clone()));
    assert!(policy.insert_local_allow(allowed.clone()));
    assert!(policy.insert_quarantine(quarantined.clone()));

    assert!(policy.remove_local_block(&blocked));
    assert!(policy.remove_local_allow(&allowed));
    assert!(policy.remove_quarantine(&quarantined));

    assert_eq!(
        policy.evaluate(&blocked).reason,
        ModerationReasonCode::NoRule
    );
    assert_eq!(
        policy.evaluate(&allowed).reason,
        ModerationReasonCode::NoRule
    );
    assert_eq!(
        policy.evaluate(&quarantined).reason,
        ModerationReasonCode::NoRule
    );
}

#[test]
fn policy_json_is_strict_and_deterministic() {
    let mut policy = ModerationPolicy::default();

    assert!(policy.insert_local_block(id(OBJECT_C)));
    assert!(policy.insert_global_deny(id(OBJECT_A)));
    assert!(policy.insert_local_allow(id(OBJECT_E)));

    let first = serde_json::to_string(&policy).expect("serialize policy");
    let second = serde_json::to_string(&policy).expect("serialize policy again");

    assert_eq!(first, second);

    let decoded: ModerationPolicy = serde_json::from_str(&first).expect("deserialize policy");

    assert_eq!(decoded, policy);

    let invalid = format!(
        r#"{{
            "global_deny": ["{OBJECT_A}"],
            "local_block": [],
            "local_allow": [],
            "owner_tombstone": [],
            "quarantine": [],
            "unexpected": true
        }}"#
    );

    assert!(
        serde_json::from_str::<ModerationPolicy>(&invalid).is_err(),
        "unknown moderation policy fields must reject"
    );
}

#[test]
fn reason_codes_have_stable_wire_labels() {
    let values = [
        (ModerationReasonCode::NoRule, r#""no_rule""#),
        (ModerationReasonCode::LocalAllow, r#""local_allow""#),
        (ModerationReasonCode::GlobalDeny, r#""global_deny""#),
        (ModerationReasonCode::OwnerTombstone, r#""owner_tombstone""#),
        (ModerationReasonCode::LocalBlock, r#""local_block""#),
        (ModerationReasonCode::Quarantined, r#""quarantined""#),
    ];

    for (reason, expected) in values {
        assert_eq!(
            serde_json::to_string(&reason).expect("serialize reason code"),
            expected
        );
        assert_eq!(reason.as_str(), expected.trim_matches('"'));
    }

    assert_eq!(ModerationReasonCode::NoRule.refusal_label(), None);
    assert_eq!(ModerationReasonCode::LocalAllow.refusal_label(), None);

    for reason in [
        ModerationReasonCode::GlobalDeny,
        ModerationReasonCode::OwnerTombstone,
        ModerationReasonCode::LocalBlock,
        ModerationReasonCode::Quarantined,
    ] {
        assert_eq!(reason.refusal_label(), Some(reason.as_str()));
    }
}

#[test]
fn signed_global_and_local_policy_compose_without_forking_precedence() {
    let globally_denied = id(OBJECT_A);
    let tombstoned = id(OBJECT_B);
    let locally_blocked = id(OBJECT_C);
    let locally_allowed = id(OBJECT_D);
    let quarantined = id(OBJECT_E);

    let mut signed_global = ModerationPolicy::default();
    assert!(signed_global.insert_global_deny(globally_denied.clone()));
    assert!(signed_global.insert_owner_tombstone(tombstoned.clone()));

    let mut local = ModerationPolicy::default();
    assert!(local.insert_local_block(locally_blocked.clone()));
    assert!(local.insert_local_allow(locally_allowed.clone()));
    assert!(local.insert_quarantine(quarantined.clone()));
    assert!(local.insert_local_allow(globally_denied.clone()));

    let composed = ModerationPolicy::compose_signed_global_with_local(&signed_global, &local)
        .expect("separated global and local policy should compose");

    assert_eq!(
        composed.evaluate(&globally_denied).reason,
        ModerationReasonCode::GlobalDeny
    );
    assert_eq!(
        composed.evaluate(&tombstoned).reason,
        ModerationReasonCode::OwnerTombstone
    );
    assert_eq!(
        composed.evaluate(&locally_blocked).reason,
        ModerationReasonCode::LocalBlock
    );
    assert_eq!(
        composed.evaluate(&locally_allowed).reason,
        ModerationReasonCode::LocalAllow
    );
    assert_eq!(
        composed.evaluate(&quarantined).reason,
        ModerationReasonCode::Quarantined
    );

    assert_eq!(composed.global_deny_count(), 1);
    assert_eq!(composed.owner_tombstone_count(), 1);
    assert_eq!(composed.local_block_count(), 1);
    assert_eq!(composed.local_allow_count(), 2);
    assert_eq!(composed.quarantine_count(), 1);
}

#[test]
fn signed_global_and_local_composition_rejects_crossed_authority() {
    let object = id(OBJECT_A);

    let mut invalid_global = ModerationPolicy::default();
    assert!(invalid_global.insert_local_block(object.clone()));

    let error = ModerationPolicy::compose_signed_global_with_local(
        &invalid_global,
        &ModerationPolicy::default(),
    )
    .expect_err("signed global input must not carry local state");

    assert_eq!(
        error,
        ModerationPolicyCompositionError::SignedGlobalContainsOperatorState
    );

    let mut invalid_local = ModerationPolicy::default();
    assert!(invalid_local.insert_global_deny(object));

    let error = ModerationPolicy::compose_signed_global_with_local(
        &ModerationPolicy::default(),
        &invalid_local,
    )
    .expect_err("local input must not carry global state");

    assert_eq!(
        error,
        ModerationPolicyCompositionError::LocalPolicyContainsGlobalState
    );
}
