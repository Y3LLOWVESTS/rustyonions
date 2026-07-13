use ron_proto::{
    ContentId, NodeRewardRecipientStateV1, NodeRewardRecipientStatusV1,
    RewardBindingSignatureRefV1, RewardRecipientResolutionStateV1, RewardRecipientResolutionV1,
    ServiceNodeRewardBindingRotationV1, ServiceNodeRewardBindingV1,
    ServiceNodeRewardBindingValidationError, NODE_REWARD_RECIPIENT_STATUS_VERSION,
    REWARD_RECIPIENT_RESOLUTION_VERSION, SERVICE_NODE_REWARD_BINDING_ROTATION_VERSION,
    SERVICE_NODE_REWARD_BINDING_VERSION,
};
use serde_json::json;

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

fn valid_binding() -> ServiceNodeRewardBindingV1 {
    ServiceNodeRewardBindingV1 {
        version: SERVICE_NODE_REWARD_BINDING_VERSION,
        binding_id: "binding:node_b3c9:1".to_string(),
        service_node_id: "node_b3c9".to_string(),
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
        rotation_nonce: "rotation_nonce_001".to_string(),
        policy_hash: cid('a'),
    }
}

fn valid_rotation() -> ServiceNodeRewardBindingRotationV1 {
    ServiceNodeRewardBindingRotationV1 {
        version: SERVICE_NODE_REWARD_BINDING_ROTATION_VERSION,
        rotation_id: "rotation:node_b3c9:2".to_string(),
        service_node_id: "node_b3c9".to_string(),
        old_binding_id: "binding:node_b3c9:1".to_string(),
        new_reward_recipient_account_id: "acct_new_operator".to_string(),
        new_reward_recipient_display_address: "@new-operator".to_string(),
        requested_at_ms: 2_000,
        requested_epoch: 10,
        effective_epoch: 12,
        rotation_nonce: "rotation_nonce_002".to_string(),
        policy_hash: cid('b'),
        operator_signature: signature("new_operator"),
        node_signature: signature("node_rotation"),
        old_operator_signature: Some(signature("old_operator")),
    }
}

#[test]
fn reward_binding_accepts_bound_crablink_account_shape() {
    let binding = valid_binding();
    binding.validate().expect("valid binding should pass");

    let value = serde_json::to_value(&binding).expect("binding should serialize");
    assert_eq!(value["service_node_id"], "node_b3c9");
    assert_eq!(value["reward_recipient_account_id"], "acct_stevan");
    assert_eq!(value["reward_recipient_display_address"], "@stevan");
    assert_eq!(value["policy_hash"], cid('a').to_string());
}

#[test]
fn reward_binding_rejects_invalid_display_address_and_bad_expiry() {
    let mut bad_address = valid_binding();
    bad_address.reward_recipient_display_address = "stevan".to_string();

    assert!(matches!(
        bad_address.validate(),
        Err(
            ServiceNodeRewardBindingValidationError::InvalidDisplayAddress {
                field: "reward_recipient_display_address"
            }
        )
    ));

    let mut bad_expiry = valid_binding();
    bad_expiry.expires_at_ms = Some(bad_expiry.created_at_ms);

    assert!(matches!(
        bad_expiry.validate(),
        Err(ServiceNodeRewardBindingValidationError::InvalidOrdering {
            field: "expires_at_ms"
        })
    ));
}

#[test]
fn reward_binding_wire_rejects_unknown_fields() {
    let mut value = serde_json::to_value(valid_binding()).expect("binding should serialize");
    value
        .as_object_mut()
        .expect("binding is object")
        .insert("arbitrary_payout_address".to_string(), json!("@attacker"));

    let decoded = serde_json::from_value::<ServiceNodeRewardBindingV1>(value);
    assert!(
        decoded.is_err(),
        "reward binding DTO must reject unknown payout redirection fields"
    );
}

#[test]
fn reward_binding_rotation_requires_future_effective_epoch() {
    let rotation = valid_rotation();
    rotation.validate().expect("valid rotation should pass");

    let mut not_future = rotation;
    not_future.effective_epoch = not_future.requested_epoch;

    assert!(matches!(
        not_future.validate(),
        Err(ServiceNodeRewardBindingValidationError::InvalidOrdering {
            field: "effective_epoch"
        })
    ));
}

#[test]
fn recipient_resolution_requires_registry_resolved_account() {
    let resolution = RewardRecipientResolutionV1 {
        version: REWARD_RECIPIENT_RESOLUTION_VERSION,
        service_node_id: "node_b3c9".to_string(),
        resolved_at_epoch: 12,
        registry_root: cid('c'),
        reward_binding_root: cid('d'),
        state: RewardRecipientResolutionStateV1::Resolved,
        binding_id: Some("binding:node_b3c9:1".to_string()),
        reward_recipient_account_id: Some("acct_stevan".to_string()),
        reward_recipient_display_address: Some("@stevan".to_string()),
    };

    resolution
        .validate()
        .expect("resolved recipient should validate");

    let mut missing_account = resolution;
    missing_account.reward_recipient_account_id = None;

    assert!(matches!(
        missing_account.validate(),
        Err(
            ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                field: "reward_recipient_account_id"
            }
        )
    ));
}

#[test]
fn unresolved_resolution_cannot_smuggle_recipient_fields() {
    let resolution = RewardRecipientResolutionV1 {
        version: REWARD_RECIPIENT_RESOLUTION_VERSION,
        service_node_id: "node_b3c9".to_string(),
        resolved_at_epoch: 12,
        registry_root: cid('c'),
        reward_binding_root: cid('d'),
        state: RewardRecipientResolutionStateV1::Unbound,
        binding_id: None,
        reward_recipient_account_id: Some("acct_attacker".to_string()),
        reward_recipient_display_address: Some("@attacker".to_string()),
    };

    assert!(matches!(
        resolution.validate(),
        Err(
            ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                field: "unresolved_recipient_fields"
            }
        )
    ));
}

#[test]
fn node_reward_status_enforces_bound_and_unbound_shapes() {
    let bound = NodeRewardRecipientStatusV1 {
        version: NODE_REWARD_RECIPIENT_STATUS_VERSION,
        service_node_id: "node_b3c9".to_string(),
        state: NodeRewardRecipientStateV1::Bound,
        current_binding: Some(valid_binding()),
        pending_rotation: None,
        observed_registry_epoch: 12,
        notes: vec!["display_only_not_ledger_truth".to_string()],
    };

    bound.validate().expect("bound status should pass");

    let unbound = NodeRewardRecipientStatusV1 {
        version: NODE_REWARD_RECIPIENT_STATUS_VERSION,
        service_node_id: "node_b3c9".to_string(),
        state: NodeRewardRecipientStateV1::Unbound,
        current_binding: None,
        pending_rotation: None,
        observed_registry_epoch: 12,
        notes: Vec::new(),
    };

    unbound.validate().expect("unbound status should pass");

    let mut contradictory = unbound;
    contradictory.current_binding = Some(valid_binding());

    assert!(matches!(
        contradictory.validate(),
        Err(
            ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                field: "unbound_status"
            }
        )
    ));
}

#[test]
fn pending_rotation_status_requires_current_binding_and_rotation() {
    let status = NodeRewardRecipientStatusV1 {
        version: NODE_REWARD_RECIPIENT_STATUS_VERSION,
        service_node_id: "node_b3c9".to_string(),
        state: NodeRewardRecipientStateV1::PendingRotation,
        current_binding: Some(valid_binding()),
        pending_rotation: Some(valid_rotation()),
        observed_registry_epoch: 12,
        notes: Vec::new(),
    };

    status
        .validate()
        .expect("pending rotation status should pass");

    let mut missing_rotation = status;
    missing_rotation.pending_rotation = None;

    assert!(matches!(
        missing_rotation.validate(),
        Err(
            ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                field: "pending_rotation"
            }
        )
    ));
}
