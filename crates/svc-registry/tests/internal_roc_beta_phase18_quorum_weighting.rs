//! RO:WHAT — Phase 18 fresh-node cap and weighted quorum tests.
//!
//! RO:WHY — Proves newly created identities cannot convert population count into
//! controlling quorum authority.
//!
//! RO:INTERACTS — `svc-registry` lifecycle custody and weighted governance review,
//! `ron-policy` lifecycle transitions, and `ron-proto` Phase 15 quorum DTOs.
//!
//! RO:INVARIANTS — only eligible registry identities count; fresh weight is capped;
//! candidates and quarantined nodes reject; no manual trust or economic authority.

use ron_policy::{
    ServiceNodeEligibilityObservationV1, ServiceNodeEligibilityPolicyV1,
    ServiceNodeHistoryThresholdsV1, SERVICE_NODE_ELIGIBILITY_POLICY_VERSION,
};
use ron_proto::{
    ContentId, EpochEligibilityStatusV1, EpochEligibilityV1, EpochQuorumThresholdV1,
    RewardBindingSignatureRefV1, ServiceNodeEligibilityStateV1, ServiceNodeIdentityDescriptorV1,
    ServiceNodeQuorumV1, ServiceNodeRewardBindingV1, ServiceNodeSignatureV1, SignatureAlg,
    SERVICE_NODE_QUORUM_SCHEMA, SERVICE_NODE_QUORUM_VERSION, SERVICE_NODE_REWARD_BINDING_VERSION,
};
use serde_json::Value;
use svc_registry::{
    eligibility::{ServiceNodeEligibilityRegistry, ServiceNodeHistoryRootsUpdate},
    governance::quorum::{
        review_service_node_quorum_weight, ServiceNodeQuorumWeightClassV1,
        ServiceNodeQuorumWeightError, ServiceNodeQuorumWeightPolicyV1,
        SERVICE_NODE_QUORUM_WEIGHT_POLICY_VERSION,
    },
    rewards::RewardBindingRegistry,
};

const CURRENT_EPOCH: u64 = 22;
const QUORUM_BPS: u16 = 6_667;

#[derive(Debug, Clone)]
struct NodeSpec {
    service_node_id: String,
    registered_at_epoch: u64,
    eligible_at_epoch: Option<u64>,
    quarantine_at_epoch: Option<u64>,
}

fn mature(index: usize) -> NodeSpec {
    NodeSpec {
        service_node_id: format!("service_node:mature_{index:02}"),
        registered_at_epoch: 1,
        eligible_at_epoch: Some(4),
        quarantine_at_epoch: None,
    }
}

fn fresh(index: usize) -> NodeSpec {
    NodeSpec {
        service_node_id: format!("service_node:fresh_{index:02}"),
        registered_at_epoch: 19,
        eligible_at_epoch: Some(22),
        quarantine_at_epoch: None,
    }
}

fn candidate(index: usize) -> NodeSpec {
    NodeSpec {
        service_node_id: format!("service_node:candidate_{index:02}"),
        registered_at_epoch: 19,
        eligible_at_epoch: None,
        quarantine_at_epoch: None,
    }
}

fn quarantined(index: usize) -> NodeSpec {
    NodeSpec {
        service_node_id: format!("service_node:quarantined_{index:02}"),
        registered_at_epoch: 1,
        eligible_at_epoch: Some(4),
        quarantine_at_epoch: Some(6),
    }
}

fn cid(label: &str) -> ContentId {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in label.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    format!("b3:{hash:016x}{hash:016x}{hash:016x}{hash:016x}")
        .parse()
        .expect("fixture CID should parse")
}

fn signature_ref(label: &str) -> RewardBindingSignatureRefV1 {
    RewardBindingSignatureRefV1 {
        alg: "ed25519".to_string(),
        public_key_ref: format!("key:{label}"),
        signature: format!("signature-{label}"),
    }
}

fn binding(spec: &NodeSpec) -> ServiceNodeRewardBindingV1 {
    ServiceNodeRewardBindingV1 {
        version: SERVICE_NODE_REWARD_BINDING_VERSION,
        binding_id: format!("binding:{}", spec.service_node_id),
        service_node_id: spec.service_node_id.clone(),
        operator_account_id: "acct_operator".to_string(),
        operator_display_address: "@operator".to_string(),
        operator_passport_id: None,
        reward_recipient_account_id: "acct_operator".to_string(),
        reward_recipient_display_address: "@operator".to_string(),
        node_public_key: format!("key:{}", spec.service_node_id),
        operator_signature: signature_ref(&format!("operator:{}", spec.service_node_id)),
        node_signature: signature_ref(&spec.service_node_id),
        created_at_ms: 1_000,
        effective_epoch: spec.registered_at_epoch,
        expires_at_ms: None,
        rotation_nonce: format!("nonce:{}", spec.service_node_id),
        policy_hash: cid(&format!("binding-policy:{}", spec.service_node_id)),
    }
}

fn candidate_descriptor(spec: &NodeSpec) -> ServiceNodeIdentityDescriptorV1 {
    ServiceNodeIdentityDescriptorV1::new_candidate(
        spec.service_node_id.clone(),
        format!("registry:{}", spec.service_node_id),
        format!("binding:{}", spec.service_node_id),
        format!("key:{}", spec.service_node_id),
        spec.registered_at_epoch,
        cid(&format!("service-history-0:{}", spec.service_node_id)),
        cid(&format!("challenge-history-0:{}", spec.service_node_id)),
    )
    .expect("candidate descriptor should validate")
}

fn lifecycle_policy() -> ServiceNodeEligibilityPolicyV1 {
    ServiceNodeEligibilityPolicyV1 {
        version: SERVICE_NODE_ELIGIBILITY_POLICY_VERSION,
        candidate_to_probation: ServiceNodeHistoryThresholdsV1 {
            minimum_state_age_epochs: 1,
            minimum_successful_service_events: 1,
            minimum_distinct_requesters: 1,
            minimum_distinct_provider_peers: 1,
        },
        probation_to_eligible: ServiceNodeHistoryThresholdsV1 {
            minimum_state_age_epochs: 1,
            minimum_successful_service_events: 2,
            minimum_distinct_requesters: 2,
            minimum_distinct_provider_peers: 2,
        },
        maximum_failure_rate_bps: 2_000,
        degrade_upheld_challenge_count: 1,
        quarantine_upheld_challenge_count: 2,
        block_upheld_challenge_count: 4,
    }
}

fn observation(
    descriptor: &ServiceNodeIdentityDescriptorV1,
    evaluation_epoch: u64,
) -> ServiceNodeEligibilityObservationV1 {
    ServiceNodeEligibilityObservationV1 {
        service_node_id: descriptor.service_node_id.clone(),
        evaluation_epoch,
        service_history_root: descriptor.service_history_root.clone(),
        challenge_history_root: descriptor.challenge_history_root.clone(),
        successful_service_events_in_state: 2,
        failed_service_events_in_state: 0,
        distinct_requesters_in_state: 2,
        distinct_provider_peers_in_state: 2,
        upheld_challenges_in_state: 0,
        rejected_challenges_in_state: 0,
        pending_challenges_in_state: 0,
    }
}

fn promote(registry: &mut ServiceNodeEligibilityRegistry, spec: &NodeSpec, eligible_at_epoch: u64) {
    let candidate = registry
        .descriptor(&spec.service_node_id)
        .expect("candidate should exist")
        .clone();

    registry
        .evaluate_and_apply_policy(
            lifecycle_policy(),
            &observation(&candidate, spec.registered_at_epoch + 1),
        )
        .expect("candidate should enter probation");

    let probation = registry
        .descriptor(&spec.service_node_id)
        .expect("probation should exist")
        .clone();

    registry
        .update_history_roots(ServiceNodeHistoryRootsUpdate {
            service_node_id: spec.service_node_id.clone(),
            expected_service_history_root: probation.service_history_root,
            service_history_root: cid(&format!("service-history-1:{}", spec.service_node_id)),
            expected_challenge_history_root: probation.challenge_history_root,
            challenge_history_root: cid(&format!("challenge-history-1:{}", spec.service_node_id)),
            effective_epoch: spec.registered_at_epoch + 2,
        })
        .expect("history roots should advance");

    let probation = registry
        .descriptor(&spec.service_node_id)
        .expect("probation should remain")
        .clone();

    registry
        .evaluate_and_apply_policy(
            lifecycle_policy(),
            &observation(&probation, eligible_at_epoch),
        )
        .expect("probation should become eligible");
}

fn quarantine(
    registry: &mut ServiceNodeEligibilityRegistry,
    spec: &NodeSpec,
    quarantine_at_epoch: u64,
) {
    let eligible = registry
        .descriptor(&spec.service_node_id)
        .expect("eligible descriptor should exist")
        .clone();

    registry
        .update_history_roots(ServiceNodeHistoryRootsUpdate {
            service_node_id: spec.service_node_id.clone(),
            expected_service_history_root: eligible.service_history_root,
            service_history_root: cid(&format!(
                "service-history-quarantine:{}",
                spec.service_node_id
            )),
            expected_challenge_history_root: eligible.challenge_history_root,
            challenge_history_root: cid(&format!(
                "challenge-history-quarantine:{}",
                spec.service_node_id
            )),
            effective_epoch: quarantine_at_epoch - 1,
        })
        .expect("challenge history roots should advance");

    let eligible = registry
        .descriptor(&spec.service_node_id)
        .expect("eligible descriptor should remain")
        .clone();

    let mut challenged = observation(&eligible, quarantine_at_epoch);
    challenged.upheld_challenges_in_state = 2;

    registry
        .evaluate_and_apply_policy(lifecycle_policy(), &challenged)
        .expect("upheld challenges should quarantine");
}

fn registry(specs: &[NodeSpec]) -> ServiceNodeEligibilityRegistry {
    let mut reward_bindings = RewardBindingRegistry::new(cid("registry-root"), cid("binding-root"));

    for spec in specs {
        reward_bindings
            .insert_binding(binding(spec))
            .expect("reward binding should insert");
    }

    let mut registry = ServiceNodeEligibilityRegistry::new();

    for spec in specs {
        registry
            .register_candidate(candidate_descriptor(spec), &reward_bindings, 2_000)
            .expect("candidate should register");
    }

    for spec in specs {
        if let Some(eligible_at_epoch) = spec.eligible_at_epoch {
            promote(&mut registry, spec, eligible_at_epoch);
        }

        if let Some(quarantine_at_epoch) = spec.quarantine_at_epoch {
            quarantine(&mut registry, spec, quarantine_at_epoch);
        }
    }

    registry
}

fn weight_policy() -> ServiceNodeQuorumWeightPolicyV1 {
    ServiceNodeQuorumWeightPolicyV1 {
        version: SERVICE_NODE_QUORUM_WEIGHT_POLICY_VERSION,
        minimum_mature_registration_age_epochs: 5,
        minimum_mature_eligibility_age_epochs: 5,
        mature_weight_units: 100,
        fresh_weight_units: 25,
        maximum_fresh_weight_bps: 2_500,
    }
}

fn threshold(member_count: usize) -> EpochQuorumThresholdV1 {
    let eligible_service_nodes = u16::try_from(member_count).expect("fixture count should fit u16");

    let weighted = u32::from(eligible_service_nodes) * u32::from(QUORUM_BPS);

    let required_signatures =
        u16::try_from(weighted.div_ceil(10_000).max(2)).expect("fixture threshold should fit u16");

    EpochQuorumThresholdV1 {
        version: SERVICE_NODE_QUORUM_VERSION,
        eligible_service_nodes,
        quorum_bps: QUORUM_BPS,
        minimum_signatures: 2,
        required_signatures,
    }
}

fn quorum(
    registry: &ServiceNodeEligibilityRegistry,
    claimed_node_ids: &[String],
    signer_node_ids: &[String],
) -> ServiceNodeQuorumV1 {
    let transition_hash = cid("phase18-weighted-transition");

    let mut eligibilities = claimed_node_ids
        .iter()
        .map(|service_node_id| {
            let descriptor = registry
                .descriptor(service_node_id)
                .expect("claimed descriptor should exist");

            EpochEligibilityV1 {
                version: SERVICE_NODE_QUORUM_VERSION,
                service_node_id: descriptor.service_node_id.clone(),
                registry_entry_id: descriptor.registry_entry_id.clone(),
                reward_binding_id: descriptor.reward_binding_id.clone(),
                key_id: descriptor.key_id.clone(),
                status: EpochEligibilityStatusV1::Eligible,
            }
        })
        .collect::<Vec<_>>();

    eligibilities.sort_by(|left, right| left.service_node_id.cmp(&right.service_node_id));

    let mut signatures = signer_node_ids
        .iter()
        .map(|service_node_id| {
            let descriptor = registry
                .descriptor(service_node_id)
                .expect("signer descriptor should exist");

            ServiceNodeSignatureV1 {
                version: SERVICE_NODE_QUORUM_VERSION,
                chain_id: "rustyonions-dev".to_string(),
                epoch_id: format!("epoch:{CURRENT_EPOCH}"),
                service_node_id: descriptor.service_node_id.clone(),
                key_id: descriptor.key_id.clone(),
                algorithm: SignatureAlg::Ed25519,
                transition_hash: transition_hash.clone(),
                signature_wire: format!("signature:{service_node_id}"),
            }
        })
        .collect::<Vec<_>>();

    signatures.sort_by(|left, right| left.service_node_id.cmp(&right.service_node_id));

    ServiceNodeQuorumV1 {
        schema: SERVICE_NODE_QUORUM_SCHEMA.to_string(),
        version: SERVICE_NODE_QUORUM_VERSION,
        chain_id: "rustyonions-dev".to_string(),
        epoch_id: format!("epoch:{CURRENT_EPOCH}"),
        transition_hash,
        threshold: threshold(eligibilities.len()),
        eligibilities,
        signatures,
    }
}

fn ids(specs: &[NodeSpec]) -> Vec<String> {
    specs
        .iter()
        .map(|spec| spec.service_node_id.clone())
        .collect()
}

#[test]
fn two_of_three_mature_signatures_fail_existing_structural_quorum() {
    let specs = vec![mature(0), mature(1), mature(2)];
    let registry = registry(&specs);
    let claimed = ids(&specs);
    let signers = claimed[..2].to_vec();
    let quorum = quorum(&registry, &claimed, &signers);

    let error =
        review_service_node_quorum_weight(&registry, &quorum, CURRENT_EPOCH, weight_policy())
            .expect_err("two signatures must fail the existing structural threshold");

    assert!(matches!(
        error,
        ServiceNodeQuorumWeightError::StructuralQuorum(_)
    ));
}

#[test]
fn all_three_mature_signatures_reach_weighted_quorum() {
    let specs = vec![mature(0), mature(1), mature(2)];
    let registry = registry(&specs);
    let claimed = ids(&specs);
    let quorum = quorum(&registry, &claimed, &claimed);

    let review =
        review_service_node_quorum_weight(&registry, &quorum, CURRENT_EPOCH, weight_policy())
            .expect("all mature signatures should pass");

    assert!(review.reaches_weighted_quorum());
    assert_eq!(review.actual_signed_weight, 300);
    assert!(!review.authorizes_economic_mutation());
}

#[test]
fn candidate_claimed_as_eligible_is_rejected() {
    let specs = vec![mature(0), mature(1), candidate(0)];
    let registry = registry(&specs);
    let claimed = ids(&specs);
    let quorum = quorum(&registry, &claimed, &claimed);

    let error =
        review_service_node_quorum_weight(&registry, &quorum, CURRENT_EPOCH, weight_policy())
            .expect_err("candidate must not count toward quorum");

    assert!(matches!(
        error,
        ServiceNodeQuorumWeightError::LifecycleIneligible {
            state: ServiceNodeEligibilityStateV1::Candidate,
            ..
        }
    ));
}

#[test]
fn quarantined_signer_is_rejected_before_weight_is_counted() {
    let specs = vec![mature(0), mature(1), quarantined(0)];
    let registry = registry(&specs);
    let claimed = ids(&specs);
    let quorum = quorum(&registry, &claimed, &claimed);

    let error =
        review_service_node_quorum_weight(&registry, &quorum, CURRENT_EPOCH, weight_policy())
            .expect_err("quarantined signer must reject");

    assert!(matches!(
        error,
        ServiceNodeQuorumWeightError::LifecycleIneligible {
            state: ServiceNodeEligibilityStateV1::Quarantined,
            ..
        }
    ));
}

#[test]
fn many_fresh_signatures_cannot_control_existing_mature_quorum() {
    let mut specs = vec![mature(0), mature(1)];

    for index in 0..20 {
        specs.push(fresh(index));
    }

    let registry = registry(&specs);
    let claimed = ids(&specs);

    let fresh_signers = specs
        .iter()
        .filter(|spec| spec.service_node_id.contains(":fresh_"))
        .map(|spec| spec.service_node_id.clone())
        .collect::<Vec<_>>();

    assert!(
        fresh_signers.len() >= usize::from(threshold(claimed.len()).required_signatures),
        "fresh population must pass the old structural count threshold"
    );

    let quorum = quorum(&registry, &claimed, &fresh_signers);

    let error =
        review_service_node_quorum_weight(&registry, &quorum, CURRENT_EPOCH, weight_policy())
            .expect_err("fresh-only signatures must fail weighted quorum");

    assert_eq!(
        error,
        ServiceNodeQuorumWeightError::InsufficientWeightedQuorum {
            required: 178,
            actual: 66,
        }
    );
}

#[test]
fn aggregate_fresh_weight_is_capped_as_a_strict_minority() {
    let mut specs = vec![mature(0), mature(1)];

    for index in 0..20 {
        specs.push(fresh(index));
    }

    let registry = registry(&specs);
    let claimed = ids(&specs);
    let quorum = quorum(&registry, &claimed, &claimed);

    let review =
        review_service_node_quorum_weight(&registry, &quorum, CURRENT_EPOCH, weight_policy())
            .expect("mature plus fresh signatures should pass");

    assert_eq!(review.raw_mature_weight, 200);
    assert_eq!(review.raw_fresh_weight, 500);
    assert_eq!(review.maximum_counted_fresh_weight, 66);
    assert_eq!(review.counted_fresh_weight, 66);
    assert_eq!(review.total_counted_weight, 266);
    assert_eq!(review.actual_signed_weight, 266);
    assert!(review.reaches_weighted_quorum());

    assert_eq!(
        review
            .members
            .iter()
            .filter(|member| {
                member.weight_class == ServiceNodeQuorumWeightClassV1::FreshEligible
            })
            .count(),
        20
    );
}

#[test]
fn registry_insertion_order_does_not_change_weight_review() {
    let forward = vec![mature(0), mature(1), mature(2), fresh(0)];
    let mut reverse = forward.clone();
    reverse.reverse();

    let forward_registry = registry(&forward);
    let reverse_registry = registry(&reverse);

    let claimed = ids(&forward);
    let quorum = quorum(&forward_registry, &claimed, &claimed);

    let forward_review = review_service_node_quorum_weight(
        &forward_registry,
        &quorum,
        CURRENT_EPOCH,
        weight_policy(),
    )
    .expect("forward registry review should pass");

    let reverse_review = review_service_node_quorum_weight(
        &reverse_registry,
        &quorum,
        CURRENT_EPOCH,
        weight_policy(),
    )
    .expect("reverse registry review should pass");

    assert_eq!(forward_review, reverse_review);
}

#[test]
fn unsafe_or_founder_weight_policy_fields_are_rejected() {
    let mut unsafe_policy = weight_policy();
    unsafe_policy.maximum_fresh_weight_bps = 5_000;

    assert!(matches!(
        unsafe_policy.validate(),
        Err(ServiceNodeQuorumWeightError::InvalidPolicy {
            field: "maximum_fresh_weight_bps",
            ..
        })
    ));

    let mut encoded = serde_json::to_value(weight_policy()).expect("policy should serialize");

    encoded
        .as_object_mut()
        .expect("policy should encode as object")
        .insert("founder_weight_units".to_string(), Value::from(1_000));

    assert!(
        serde_json::from_value::<ServiceNodeQuorumWeightPolicyV1>(encoded).is_err(),
        "manual founder weighting must not exist"
    );
}
