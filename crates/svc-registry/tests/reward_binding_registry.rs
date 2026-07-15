use ron_proto::{
    ContentId, NodeRewardRecipientStateV1, RewardBindingSignatureRefV1,
    RewardRecipientResolutionStateV1, ServiceNodeRewardBindingRotationV1,
    ServiceNodeRewardBindingV1, SERVICE_NODE_REWARD_BINDING_ROTATION_VERSION,
    SERVICE_NODE_REWARD_BINDING_VERSION,
};
use svc_registry::rewards::{RewardBindingRegistry, RewardBindingRegistryError};

fn cid(byte: char) -> ContentId {
    let raw = format!("b3:{}", byte.to_string().repeat(64));
    ContentId::parse(&raw).expect("test cid should parse")
}

fn signature(label: &str) -> RewardBindingSignatureRefV1 {
    RewardBindingSignatureRefV1 {
        alg: "ed25519".to_string(),
        public_key_ref: format!("key:{label}"),
        signature: format!("sig-{label}-bytes"),
    }
}

fn binding_for(node_id: &str, binding_id: &str, nonce: &str) -> ServiceNodeRewardBindingV1 {
    ServiceNodeRewardBindingV1 {
        version: SERVICE_NODE_REWARD_BINDING_VERSION,
        binding_id: binding_id.to_string(),
        service_node_id: node_id.to_string(),
        operator_account_id: "acct_stevan".to_string(),
        operator_display_address: "@stevan".to_string(),
        operator_passport_id: Some("passport:main:stevan".to_string()),
        reward_recipient_account_id: "acct_stevan".to_string(),
        reward_recipient_display_address: "@stevan".to_string(),
        node_public_key: "node_pk_b3c9".to_string(),
        operator_signature: signature("operator"),
        node_signature: signature("node"),
        created_at_ms: 1_000,
        effective_epoch: 7,
        expires_at_ms: Some(9_999),
        rotation_nonce: nonce.to_string(),
        policy_hash: cid('a'),
    }
}

fn rotation_for(
    node_id: &str,
    old_binding_id: &str,
    rotation_id: &str,
    nonce: &str,
) -> ServiceNodeRewardBindingRotationV1 {
    ServiceNodeRewardBindingRotationV1 {
        version: SERVICE_NODE_REWARD_BINDING_ROTATION_VERSION,
        rotation_id: rotation_id.to_string(),
        service_node_id: node_id.to_string(),
        old_binding_id: old_binding_id.to_string(),
        new_reward_recipient_account_id: "acct_new_operator".to_string(),
        new_reward_recipient_display_address: "@new-operator".to_string(),
        requested_at_ms: 2_000,
        requested_epoch: 10,
        effective_epoch: 12,
        rotation_nonce: nonce.to_string(),
        policy_hash: cid('b'),
        operator_signature: signature("new_operator"),
        node_signature: signature("node_rotation"),
        old_operator_signature: Some(signature("old_operator")),
    }
}

fn registry() -> RewardBindingRegistry {
    RewardBindingRegistry::new(cid('c'), cid('d'))
}

#[test]
fn registry_resolves_service_node_to_bound_reward_recipient() {
    let mut registry = registry();
    registry
        .insert_binding(binding_for("node_b3c9", "binding:node_b3c9:1", "nonce_001"))
        .expect("valid binding should insert");

    let resolution = registry.resolve("node_b3c9", 7, 1_500);
    resolution
        .validate()
        .expect("resolved recipient should validate");

    assert_eq!(resolution.state, RewardRecipientResolutionStateV1::Resolved);
    assert_eq!(
        resolution.reward_recipient_account_id.as_deref(),
        Some("acct_stevan")
    );
    assert_eq!(
        resolution.reward_recipient_display_address.as_deref(),
        Some("@stevan")
    );
}

#[test]
fn registry_returns_unbound_without_smuggling_recipient_fields() {
    let registry = registry();

    let resolution = registry.resolve("node_missing", 7, 1_500);
    resolution
        .validate()
        .expect("unbound resolution should validate");

    assert_eq!(resolution.state, RewardRecipientResolutionStateV1::Unbound);
    assert!(resolution.reward_recipient_account_id.is_none());
    assert!(resolution.reward_recipient_display_address.is_none());
}

#[test]
fn registry_rejects_duplicate_binding_id_and_duplicate_nonce() {
    let mut registry = registry();
    registry
        .insert_binding(binding_for("node_one", "binding:shared:1", "nonce_001"))
        .expect("first binding should insert");

    let duplicate_id =
        registry.insert_binding(binding_for("node_two", "binding:shared:1", "nonce_002"));

    assert!(matches!(
        duplicate_id,
        Err(RewardBindingRegistryError::DuplicateBindingId { .. })
    ));

    let duplicate_nonce = registry.insert_binding(binding_for(
        "node_three",
        "binding:node_three:1",
        "nonce_001",
    ));

    assert!(matches!(
        duplicate_nonce,
        Err(RewardBindingRegistryError::DuplicateRotationNonce { .. })
    ));
}

#[test]
fn registry_rejects_second_current_binding_for_same_service_node() {
    let mut registry = registry();
    registry
        .insert_binding(binding_for("node_b3c9", "binding:node_b3c9:1", "nonce_001"))
        .expect("first binding should insert");

    let second =
        registry.insert_binding(binding_for("node_b3c9", "binding:node_b3c9:2", "nonce_002"));

    assert!(matches!(
        second,
        Err(RewardBindingRegistryError::ServiceNodeAlreadyBound { .. })
    ));
}

#[test]
fn registry_reports_expired_binding_without_recipient_account() {
    let mut registry = registry();
    registry
        .insert_binding(binding_for("node_b3c9", "binding:node_b3c9:1", "nonce_001"))
        .expect("valid binding should insert");

    let resolution = registry.resolve("node_b3c9", 7, 10_000);
    resolution
        .validate()
        .expect("expired resolution should validate");

    assert_eq!(resolution.state, RewardRecipientResolutionStateV1::Expired);
    assert!(resolution.reward_recipient_account_id.is_none());
    assert!(resolution.reward_recipient_display_address.is_none());

    let status = registry.status("node_b3c9", 12, 10_000);
    status.validate().expect("expired status should validate");
    assert_eq!(status.state, NodeRewardRecipientStateV1::Expired);
}

#[test]
fn registry_schedules_pending_rotation_with_nonce_replay_rejection() {
    let mut registry = registry();
    registry
        .insert_binding(binding_for("node_b3c9", "binding:node_b3c9:1", "nonce_001"))
        .expect("valid binding should insert");

    registry
        .schedule_rotation(rotation_for(
            "node_b3c9",
            "binding:node_b3c9:1",
            "rotation:node_b3c9:2",
            "nonce_002",
        ))
        .expect("valid rotation should schedule");

    let status = registry.status("node_b3c9", 11, 1_500);
    status
        .validate()
        .expect("pending rotation status should validate");

    assert_eq!(status.state, NodeRewardRecipientStateV1::PendingRotation);
    assert!(status.current_binding.is_some());
    assert!(status.pending_rotation.is_some());

    let replay = registry.schedule_rotation(rotation_for(
        "node_b3c9",
        "binding:node_b3c9:1",
        "rotation:node_b3c9:3",
        "nonce_002",
    ));

    assert!(matches!(
        replay,
        Err(RewardBindingRegistryError::DuplicateRotationNonce { .. })
    ));
}

#[test]
fn phase23_mid_epoch_rotation_replacement_fails_closed_without_redirecting_recipient() {
    let mut registry = registry();

    registry
        .insert_binding(binding_for("node_b3c9", "binding:node_b3c9:1", "nonce_001"))
        .expect("initial reward binding should insert");

    registry
        .schedule_rotation(rotation_for(
            "node_b3c9",
            "binding:node_b3c9:1",
            "rotation:node_b3c9:2",
            "nonce_002",
        ))
        .expect("first future-epoch rotation should schedule");

    let replacement = registry.schedule_rotation(rotation_for(
        "node_b3c9",
        "binding:node_b3c9:1",
        "rotation:node_b3c9:attacker-replacement",
        "nonce_003",
    ));

    assert!(matches!(
        replacement,
        Err(RewardBindingRegistryError::PendingRotationAlreadyScheduled { .. })
    ));

    let mid_epoch_status = registry.status("node_b3c9", 11, 2_500);

    mid_epoch_status
        .validate()
        .expect("mid-epoch pending status should remain valid");

    assert_eq!(
        mid_epoch_status.state,
        NodeRewardRecipientStateV1::PendingRotation,
    );

    let retained_rotation = mid_epoch_status
        .pending_rotation
        .as_ref()
        .expect("original pending rotation must remain retained");

    assert_eq!(
        retained_rotation.rotation_id, "rotation:node_b3c9:2",
        "rejected replacement must not overwrite the accepted rotation",
    );

    assert_eq!(retained_rotation.rotation_nonce, "nonce_002");

    let mid_epoch_resolution = registry.resolve("node_b3c9", 11, 2_500);

    mid_epoch_resolution
        .validate()
        .expect("mid-epoch recipient resolution should validate");

    assert_eq!(
        mid_epoch_resolution.state,
        RewardRecipientResolutionStateV1::Resolved,
    );

    assert_eq!(
        mid_epoch_resolution.reward_recipient_account_id.as_deref(),
        Some("acct_stevan"),
        "pending rotation must not redirect the current epoch recipient",
    );

    assert_eq!(
        mid_epoch_resolution
            .reward_recipient_display_address
            .as_deref(),
        Some("@stevan"),
    );

    let premature = registry
        .apply_due_rotation("node_b3c9", 11, 2_500)
        .expect("not-due rotation should remain a safe no-op");

    assert!(!premature);

    let applied = registry
        .apply_due_rotation("node_b3c9", 12, 3_000)
        .expect("accepted rotation should apply at its effective epoch");

    assert!(applied);

    let effective_resolution = registry.resolve("node_b3c9", 12, 3_000);

    effective_resolution
        .validate()
        .expect("effective-epoch resolution should validate");

    assert_eq!(
        effective_resolution.reward_recipient_account_id.as_deref(),
        Some("acct_new_operator"),
    );

    let rotated_binding_id = "binding:node_b3c9:1:rotated:rotation:node_b3c9:2";

    let mut later_rotation = rotation_for(
        "node_b3c9",
        rotated_binding_id,
        "rotation:node_b3c9:3",
        "nonce_003",
    );

    later_rotation.requested_at_ms = 3_100;
    later_rotation.requested_epoch = 12;
    later_rotation.effective_epoch = 14;

    registry
        .schedule_rotation(later_rotation)
        .expect("nonce from rejected replacement must not be consumed");

    let later_status = registry.status("node_b3c9", 12, 3_100);

    later_status
        .validate()
        .expect("later pending rotation status should validate");

    assert_eq!(
        later_status
            .pending_rotation
            .as_ref()
            .expect("later rotation must be pending")
            .rotation_nonce,
        "nonce_003",
    );

    println!(
        "Phase 23E passed: a second mid-epoch reward-recipient rotation \
         was rejected without overwriting the accepted rotation or \
         consuming its nonce, the current epoch retained the original \
         recipient, the accepted rotation applied only at its effective \
         epoch, and no wallet, ledger, payout, receipt, confirmed-ROC, \
         or finality authority was created."
    );
}

#[test]
fn registry_rejects_rotation_for_missing_or_mismatched_binding() {
    let mut registry = registry();

    let missing = registry.schedule_rotation(rotation_for(
        "node_b3c9",
        "binding:node_b3c9:missing",
        "rotation:node_b3c9:2",
        "nonce_002",
    ));

    assert!(matches!(
        missing,
        Err(RewardBindingRegistryError::BindingNotFound { .. })
    ));

    registry
        .insert_binding(binding_for(
            "node_original",
            "binding:node_original:1",
            "nonce_001",
        ))
        .expect("valid binding should insert");

    let mismatch = registry.schedule_rotation(rotation_for(
        "node_other",
        "binding:node_original:1",
        "rotation:node_other:2",
        "nonce_002",
    ));

    assert!(matches!(
        mismatch,
        Err(RewardBindingRegistryError::ServiceNodeMismatch)
    ));
}

#[test]
fn registry_applies_pending_rotation_only_after_effective_epoch() {
    let mut registry = registry();
    registry
        .insert_binding(binding_for("node_b3c9", "binding:node_b3c9:1", "nonce_001"))
        .expect("valid binding should insert");

    registry
        .schedule_rotation(rotation_for(
            "node_b3c9",
            "binding:node_b3c9:1",
            "rotation:node_b3c9:2",
            "nonce_002",
        ))
        .expect("valid rotation should schedule");

    let premature = registry
        .apply_due_rotation("node_b3c9", 11, 2_500)
        .expect("not-due rotation should not error");
    assert!(!premature, "rotation must not apply before effective epoch");

    let before_due = registry.resolve("node_b3c9", 11, 2_500);
    before_due
        .validate()
        .expect("pre-rotation resolution should validate");
    assert_eq!(
        before_due.reward_recipient_account_id.as_deref(),
        Some("acct_stevan")
    );

    let pending_status = registry.status("node_b3c9", 11, 2_500);
    pending_status
        .validate()
        .expect("pending status should validate");
    assert_eq!(
        pending_status.state,
        NodeRewardRecipientStateV1::PendingRotation
    );

    let applied = registry
        .apply_due_rotation("node_b3c9", 12, 3_000)
        .expect("due rotation should apply");
    assert!(applied, "rotation should apply at effective epoch");

    let after_due = registry.resolve("node_b3c9", 12, 3_000);
    after_due
        .validate()
        .expect("post-rotation resolution should validate");
    assert_eq!(
        after_due.reward_recipient_account_id.as_deref(),
        Some("acct_new_operator")
    );
    assert_eq!(
        after_due.reward_recipient_display_address.as_deref(),
        Some("@new-operator")
    );

    let bound_status = registry.status("node_b3c9", 12, 3_000);
    bound_status
        .validate()
        .expect("post-rotation status should validate");
    assert_eq!(bound_status.state, NodeRewardRecipientStateV1::Bound);
    assert!(bound_status.pending_rotation.is_none());
}

#[test]
fn registry_apply_rotation_is_noop_without_pending_rotation() {
    let mut registry = registry();
    registry
        .insert_binding(binding_for("node_b3c9", "binding:node_b3c9:1", "nonce_001"))
        .expect("valid binding should insert");

    let applied = registry
        .apply_due_rotation("node_b3c9", 12, 3_000)
        .expect("no pending rotation should not error");

    assert!(!applied);
}
