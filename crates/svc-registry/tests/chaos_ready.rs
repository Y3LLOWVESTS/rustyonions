use ron_proto::{ContentId, RewardRecipientResolutionStateV1};
use svc_registry::rewards::RewardBindingRegistry;

fn cid(byte: char) -> ContentId {
    let raw = format!("b3:{}", byte.to_string().repeat(64));
    ContentId::parse(&raw).expect("test cid should parse")
}

#[test]
fn reward_binding_resolution_fails_closed_when_node_is_unknown() {
    let registry = RewardBindingRegistry::new(cid('e'), cid('f'));

    let resolution = registry.resolve("node_missing_after_churn", 42, 42);
    resolution
        .validate()
        .expect("missing node resolution should still be a valid fail-closed DTO");

    assert_eq!(resolution.state, RewardRecipientResolutionStateV1::Unbound);
    assert!(resolution.reward_recipient_account_id.is_none());
    assert!(resolution.reward_recipient_display_address.is_none());
}
