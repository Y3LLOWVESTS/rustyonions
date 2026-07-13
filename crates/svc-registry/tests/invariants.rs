use ron_proto::{
    ContentId, RewardRecipientResolutionStateV1, RewardRecipientResolutionV1,
    REWARD_RECIPIENT_RESOLUTION_VERSION,
};
use svc_registry::rewards::RewardBindingRegistry;

fn cid(byte: char) -> ContentId {
    let raw = format!("b3:{}", byte.to_string().repeat(64));
    ContentId::parse(&raw).expect("test cid should parse")
}

#[test]
fn reward_resolution_unbound_shape_has_no_recipient_fields() {
    let registry = RewardBindingRegistry::new(cid('a'), cid('b'));

    let resolution = registry.resolve("node_unbound", 1, 1);
    resolution
        .validate()
        .expect("unbound resolution should remain a valid DTO");

    assert_eq!(resolution.version, REWARD_RECIPIENT_RESOLUTION_VERSION);
    assert_eq!(resolution.state, RewardRecipientResolutionStateV1::Unbound);
    assert_eq!(resolution.service_node_id, "node_unbound");
    assert!(resolution.binding_id.is_none());
    assert!(resolution.reward_recipient_account_id.is_none());
    assert!(resolution.reward_recipient_display_address.is_none());
}

#[test]
fn reward_resolution_dto_rejects_unbound_recipient_smuggling() {
    let resolution = RewardRecipientResolutionV1 {
        version: REWARD_RECIPIENT_RESOLUTION_VERSION,
        service_node_id: "node_unbound".to_string(),
        resolved_at_epoch: 1,
        registry_root: cid('a'),
        reward_binding_root: cid('b'),
        state: RewardRecipientResolutionStateV1::Unbound,
        binding_id: None,
        reward_recipient_account_id: Some("acct_attacker".to_string()),
        reward_recipient_display_address: Some("@attacker".to_string()),
    };

    assert!(
        resolution.validate().is_err(),
        "unbound resolution must not carry recipient fields"
    );
}
