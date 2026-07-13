//! RO:WHAT — Tests registry-backed reward binding/rotation request intake.
//! RO:WHY — Phase 5C moves from local operator request state toward svc-registry-backed binding logic.
//! RO:INVARIANTS — no wallet mutation; no ledger mutation; no confirmed ROC; no payout override.

use ron_proto::{
    ContentId, NodeRewardRecipientStateV1, RewardBindingSignatureRefV1,
    RewardRecipientResolutionStateV1,
};
use svc_registry::rewards::{
    RegistryRewardBindingRequestIntake, RegistryRewardBindingRequestV1,
    RegistryRewardBindingRotationRequestV1, ResolvedRewardRecipientForBinding,
    RewardBindingRegistry, RewardBindingRequestError,
};

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

fn recipient(account_id: &str, display_address: &str) -> ResolvedRewardRecipientForBinding {
    ResolvedRewardRecipientForBinding {
        account_id: account_id.to_string(),
        display_address: display_address.to_string(),
    }
}

fn intake() -> RegistryRewardBindingRequestIntake {
    RegistryRewardBindingRequestIntake::new(RewardBindingRegistry::new(cid('c'), cid('d')))
}

fn binding_request() -> RegistryRewardBindingRequestV1 {
    RegistryRewardBindingRequestV1 {
        binding_id: "binding:node_b3c9:1".to_string(),
        service_node_id: "node_b3c9".to_string(),
        operator_account_id: "acct_stevan".to_string(),
        operator_display_address: "@stevan".to_string(),
        operator_passport_id: Some("passport:main:stevan".to_string()),
        reward_recipient: recipient("acct_stevan", "@stevan"),
        node_public_key: "node_pk_b3c9".to_string(),
        operator_signature: signature("operator"),
        node_signature: signature("node"),
        created_at_ms: 1_000,
        effective_epoch: 7,
        expires_at_ms: Some(9_999),
        rotation_nonce: "nonce_001".to_string(),
        policy_hash: cid('a'),
        evidence_payout_override: None,
    }
}

fn rotation_request() -> RegistryRewardBindingRotationRequestV1 {
    RegistryRewardBindingRotationRequestV1 {
        rotation_id: "rotation:node_b3c9:2".to_string(),
        service_node_id: "node_b3c9".to_string(),
        old_binding_id: "binding:node_b3c9:1".to_string(),
        new_reward_recipient: recipient("acct_new_operator", "@new-operator"),
        requested_at_ms: 2_000,
        requested_epoch: 10,
        effective_epoch: 12,
        rotation_nonce: "nonce_002".to_string(),
        policy_hash: cid('b'),
        operator_signature: signature("new_operator"),
        node_signature: signature("node_rotation"),
        old_operator_signature: Some(signature("old_operator")),
        evidence_payout_override: None,
    }
}

#[test]
fn binding_request_inserts_registry_binding_and_resolves_recipient_without_mutation() {
    let mut intake = intake();

    let receipt = intake
        .submit_binding_request(binding_request())
        .expect("binding request should be accepted");

    assert_eq!(receipt.status, "binding accepted by registry intake");
    assert_eq!(receipt.service_node_id, "node_b3c9");
    assert_eq!(receipt.request_id, "binding:node_b3c9:1");
    assert_eq!(receipt.reward_recipient_account_id, "acct_stevan");
    assert_eq!(receipt.reward_recipient_display_address, "@stevan");
    assert!(!receipt.wallet_mutation);
    assert!(!receipt.ledger_mutation);
    assert_eq!(receipt.confirmed_roc, None);

    let resolution = intake.registry().resolve("node_b3c9", 7, 1_500);
    resolution
        .validate()
        .expect("registry resolution should validate");
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
fn binding_request_rejects_evidence_payout_override_before_registry_insert() {
    let mut intake = intake();
    let mut request = binding_request();
    request.evidence_payout_override = Some("@attacker".to_string());

    let err = intake
        .submit_binding_request(request)
        .expect_err("payout override must be rejected");

    assert!(matches!(
        err,
        RewardBindingRequestError::EvidencePayoutOverrideForbidden
    ));

    let resolution = intake.registry().resolve("node_b3c9", 7, 1_500);
    resolution
        .validate()
        .expect("unbound resolution should validate");
    assert_eq!(resolution.state, RewardRecipientResolutionStateV1::Unbound);
    assert!(resolution.reward_recipient_account_id.is_none());
    assert!(resolution.reward_recipient_display_address.is_none());
}

#[test]
fn rotation_request_schedules_pending_rotation_without_mutation_or_finality() {
    let mut intake = intake();
    intake
        .submit_binding_request(binding_request())
        .expect("initial binding should be accepted");

    let receipt = intake
        .submit_rotation_request(rotation_request())
        .expect("rotation request should be accepted");

    assert_eq!(receipt.status, "rotation accepted by registry intake");
    assert_eq!(receipt.service_node_id, "node_b3c9");
    assert_eq!(receipt.request_id, "rotation:node_b3c9:2");
    assert_eq!(receipt.reward_recipient_account_id, "acct_new_operator");
    assert_eq!(receipt.reward_recipient_display_address, "@new-operator");
    assert!(!receipt.wallet_mutation);
    assert!(!receipt.ledger_mutation);
    assert_eq!(receipt.confirmed_roc, None);

    let status = intake.registry().status("node_b3c9", 11, 2_500);
    status.validate().expect("pending status should validate");
    assert_eq!(status.state, NodeRewardRecipientStateV1::PendingRotation);
    assert!(status.pending_rotation.is_some());

    let before_due = intake.registry().resolve("node_b3c9", 11, 2_500);
    assert_eq!(
        before_due.reward_recipient_account_id.as_deref(),
        Some("acct_stevan")
    );

    let applied = intake
        .registry_mut()
        .apply_due_rotation("node_b3c9", 12, 3_000)
        .expect("due rotation should apply");
    assert!(applied);

    let after_due = intake.registry().resolve("node_b3c9", 12, 3_000);
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
}

#[test]
fn rotation_request_rejects_evidence_payout_override_before_registry_schedule() {
    let mut intake = intake();
    intake
        .submit_binding_request(binding_request())
        .expect("initial binding should be accepted");

    let mut request = rotation_request();
    request.evidence_payout_override = Some("@attacker".to_string());

    let err = intake
        .submit_rotation_request(request)
        .expect_err("rotation payout override must be rejected");

    assert!(matches!(
        err,
        RewardBindingRequestError::EvidencePayoutOverrideForbidden
    ));

    let status = intake.registry().status("node_b3c9", 11, 2_500);
    status.validate().expect("bound status should validate");
    assert_eq!(status.state, NodeRewardRecipientStateV1::Bound);
    assert!(status.pending_rotation.is_none());
}

#[test]
fn rotation_request_for_unbound_node_fails_through_registry_rules() {
    let mut intake = intake();

    let err = intake
        .submit_rotation_request(rotation_request())
        .expect_err("unbound rotation should be rejected");

    assert!(matches!(err, RewardBindingRequestError::Registry(_)));

    let status = intake.registry().status("node_b3c9", 11, 2_500);
    status.validate().expect("unbound status should validate");
    assert_eq!(status.state, NodeRewardRecipientStateV1::Unbound);
}
