use ron_proto::{
    ContentId, RewardRecipientResolutionStateV1, RewardRecipientResolutionV1,
    REWARD_RECIPIENT_RESOLUTION_VERSION,
};
use svc_registry::rewards::{
    preview_reward_payout_authorization, RewardPayoutAuthorizationError,
    RewardPayoutAuthorizationPreview, SelfIssuanceMode,
};

fn cid(byte: char) -> ContentId {
    let raw = format!("b3:{}", byte.to_string().repeat(64));
    ContentId::parse(&raw).expect("test cid should parse")
}

fn resolved_for(node_id: &str) -> RewardRecipientResolutionV1 {
    RewardRecipientResolutionV1 {
        version: REWARD_RECIPIENT_RESOLUTION_VERSION,
        service_node_id: node_id.to_string(),
        resolved_at_epoch: 7,
        registry_root: cid('a'),
        reward_binding_root: cid('b'),
        state: RewardRecipientResolutionStateV1::Resolved,
        binding_id: Some(format!("binding:{node_id}:1")),
        reward_recipient_account_id: Some("acct_stevan".to_string()),
        reward_recipient_display_address: Some("@stevan".to_string()),
    }
}

fn unbound_for(node_id: &str) -> RewardRecipientResolutionV1 {
    RewardRecipientResolutionV1 {
        version: REWARD_RECIPIENT_RESOLUTION_VERSION,
        service_node_id: node_id.to_string(),
        resolved_at_epoch: 7,
        registry_root: cid('a'),
        reward_binding_root: cid('b'),
        state: RewardRecipientResolutionStateV1::Unbound,
        binding_id: None,
        reward_recipient_account_id: None,
        reward_recipient_display_address: None,
    }
}

#[test]
fn independent_issuer_can_preview_resolved_bound_recipient() {
    let resolution = resolved_for("node_beneficiary");

    let result = preview_reward_payout_authorization(RewardPayoutAuthorizationPreview {
        issuer_service_node_id: "node_issuer",
        beneficiary_service_node_id: "node_beneficiary",
        recipient_resolution: &resolution,
        evidence_payout_override: None,
        self_issuance_mode: SelfIssuanceMode::RejectByDefault,
    });

    assert!(result.is_ok());
}

#[test]
fn evidence_payout_override_is_rejected_even_when_registry_resolves() {
    let resolution = resolved_for("node_beneficiary");

    let result = preview_reward_payout_authorization(RewardPayoutAuthorizationPreview {
        issuer_service_node_id: "node_issuer",
        beneficiary_service_node_id: "node_beneficiary",
        recipient_resolution: &resolution,
        evidence_payout_override: Some("@attacker"),
        self_issuance_mode: SelfIssuanceMode::RejectByDefault,
    });

    assert_eq!(
        result,
        Err(RewardPayoutAuthorizationError::EvidencePayoutOverrideForbidden)
    );
}

#[test]
fn unresolved_recipient_is_rejected_before_any_payout_preview() {
    let resolution = unbound_for("node_beneficiary");

    let result = preview_reward_payout_authorization(RewardPayoutAuthorizationPreview {
        issuer_service_node_id: "node_issuer",
        beneficiary_service_node_id: "node_beneficiary",
        recipient_resolution: &resolution,
        evidence_payout_override: None,
        self_issuance_mode: SelfIssuanceMode::RejectByDefault,
    });

    assert_eq!(
        result,
        Err(RewardPayoutAuthorizationError::RecipientNotResolved)
    );
}

#[test]
fn beneficiary_mismatch_is_rejected() {
    let resolution = resolved_for("node_other");

    let result = preview_reward_payout_authorization(RewardPayoutAuthorizationPreview {
        issuer_service_node_id: "node_issuer",
        beneficiary_service_node_id: "node_beneficiary",
        recipient_resolution: &resolution,
        evidence_payout_override: None,
        self_issuance_mode: SelfIssuanceMode::RejectByDefault,
    });

    assert_eq!(
        result,
        Err(RewardPayoutAuthorizationError::BeneficiaryMismatch)
    );
}

#[test]
fn self_issued_payout_to_bound_recipient_is_rejected_by_default() {
    let resolution = resolved_for("node_self");

    let result = preview_reward_payout_authorization(RewardPayoutAuthorizationPreview {
        issuer_service_node_id: "node_self",
        beneficiary_service_node_id: "node_self",
        recipient_resolution: &resolution,
        evidence_payout_override: None,
        self_issuance_mode: SelfIssuanceMode::RejectByDefault,
    });

    assert_eq!(
        result,
        Err(RewardPayoutAuthorizationError::SelfIssuedPayoutRejected)
    );
}

#[test]
fn self_issuance_fixture_mode_still_requires_feature_gate() {
    let resolution = resolved_for("node_self");

    let result = preview_reward_payout_authorization(RewardPayoutAuthorizationPreview {
        issuer_service_node_id: "node_self",
        beneficiary_service_node_id: "node_self",
        recipient_resolution: &resolution,
        evidence_payout_override: None,
        self_issuance_mode: SelfIssuanceMode::AllowTestFixtureOnly,
    });

    if cfg!(feature = "test-self-issuance-fixtures") {
        assert!(result.is_ok());
    } else {
        assert_eq!(
            result,
            Err(RewardPayoutAuthorizationError::SelfIssuedPayoutRejected)
        );
    }
}
