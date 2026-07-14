//! RO:WHAT — Phase 18 cross-crate Sybil-resistance scenarios.
//!
//! RO:WHY — Lifecycle state must produce consistent denial or constraint across
//! registry transition, weighted quorum review, and deterministic reward planning.
//!
//! RO:INTERACTS — `ron-policy` lifecycle evaluation, `svc-registry` canonical
//! descriptors and quorum review, and `svc-rewarder` economics-bound planning.
//!
//! RO:INVARIANTS — candidate/probation/quarantine cannot gain quorum authority;
//! probation rewards remain capped; fresh identities cannot dominate weight;
//! no wallet, ledger, receipt, mint, burn, payout, or finality mutation.
//!
//! RO:TEST — this integration target.

#![allow(clippy::missing_panics_doc)]

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
use svc_registry::{
    eligibility::{ServiceNodeEligibilityRegistry, ServiceNodeHistoryRootsUpdate},
    governance::quorum::{
        review_service_node_quorum_weight, ServiceNodeQuorumWeightError,
        ServiceNodeQuorumWeightPolicyV1, SERVICE_NODE_QUORUM_WEIGHT_POLICY_VERSION,
    },
    rewards::RewardBindingRegistry,
};
use svc_rewarder::{
    core::{
        compute_service_node_reward_plan_with_eligibility, AmountMinor,
        ServiceNodeEligibilityRewardPlan, ServiceNodeRewardCandidate,
        ServiceNodeRewardEvidenceClass, ServiceNodeRewardPlanInput,
    },
    inputs::{load_internal_roc_planning_economics_toml, InternalRocRewardPlanningEconomics},
};

const CURRENT_EPOCH: u64 = 22;
const QUORUM_BPS: u16 = 6_667;
const OBSERVED_AT_MS: u64 = 2_000;

const CANONICAL_ECONOMICS: &[u8] = include_bytes!("../../../configs/roc-economics.toml");

#[derive(Debug, Clone)]
struct NodeSpec {
    service_node_id: String,
    registered_at_epoch: u64,
    probation_at_epoch: Option<u64>,
    eligible_at_epoch: Option<u64>,
    quarantine_at_epoch: Option<u64>,
}

fn mature(index: usize) -> NodeSpec {
    NodeSpec {
        service_node_id: format!("service_node:mature_{index:02}"),
        registered_at_epoch: 1,
        probation_at_epoch: Some(2),
        eligible_at_epoch: Some(4),
        quarantine_at_epoch: None,
    }
}

fn fresh(index: usize) -> NodeSpec {
    NodeSpec {
        service_node_id: format!("service_node:fresh_{index:02}"),
        registered_at_epoch: 19,
        probation_at_epoch: Some(20),
        eligible_at_epoch: Some(22),
        quarantine_at_epoch: None,
    }
}

fn probation(index: usize) -> NodeSpec {
    NodeSpec {
        service_node_id: format!("service_node:probation_{index:02}"),
        registered_at_epoch: 19,
        probation_at_epoch: Some(20),
        eligible_at_epoch: None,
        quarantine_at_epoch: None,
    }
}

fn candidate(index: usize) -> NodeSpec {
    NodeSpec {
        service_node_id: format!("service_node:candidate_{index:02}"),
        registered_at_epoch: 19,
        probation_at_epoch: None,
        eligible_at_epoch: None,
        quarantine_at_epoch: None,
    }
}

fn quarantined(index: usize) -> NodeSpec {
    NodeSpec {
        service_node_id: format!("service_node:quarantined_{index:02}"),
        registered_at_epoch: 1,
        probation_at_epoch: Some(2),
        eligible_at_epoch: Some(4),
        quarantine_at_epoch: Some(6),
    }
}

fn slug(service_node_id: &str) -> &str {
    service_node_id
        .strip_prefix("service_node:")
        .expect("fixture Service Node ID should be canonical")
}

fn binding_id(service_node_id: &str) -> String {
    format!("binding:{}:1", slug(service_node_id))
}

fn registry_entry_id(service_node_id: &str) -> String {
    format!("registry:{}", slug(service_node_id))
}

fn key_id(service_node_id: &str) -> String {
    format!("key:{}", slug(service_node_id))
}

fn b3(label: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in label.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    format!("b3:{hash:016x}{hash:016x}{hash:016x}{hash:016x}")
}

fn cid(label: &str) -> ContentId {
    b3(label).parse().expect("fixture ContentId should parse")
}

fn signature_ref(label: &str) -> RewardBindingSignatureRefV1 {
    RewardBindingSignatureRefV1 {
        alg: "ed25519".to_string(),
        public_key_ref: format!("key:{label}"),
        signature: format!("signature-{label}"),
    }
}

fn reward_binding(spec: &NodeSpec) -> ServiceNodeRewardBindingV1 {
    ServiceNodeRewardBindingV1 {
        version: SERVICE_NODE_REWARD_BINDING_VERSION,
        binding_id: binding_id(&spec.service_node_id),
        service_node_id: spec.service_node_id.clone(),
        operator_account_id: "acct_operator".to_string(),
        operator_display_address: "@operator".to_string(),
        operator_passport_id: Some("passport:main:operator".to_string()),
        reward_recipient_account_id: "acct_operator".to_string(),
        reward_recipient_display_address: "@operator".to_string(),
        node_public_key: key_id(&spec.service_node_id),
        operator_signature: signature_ref(&format!("operator-{}", slug(&spec.service_node_id))),
        node_signature: signature_ref(&format!("node-{}", slug(&spec.service_node_id))),
        created_at_ms: 1_000,
        effective_epoch: spec.registered_at_epoch,
        expires_at_ms: None,
        rotation_nonce: format!("nonce_{}_1", slug(&spec.service_node_id)),
        policy_hash: cid(&format!("binding-policy:{}", spec.service_node_id)),
    }
}

fn candidate_descriptor(spec: &NodeSpec) -> ServiceNodeIdentityDescriptorV1 {
    ServiceNodeIdentityDescriptorV1::new_candidate(
        spec.service_node_id.clone(),
        registry_entry_id(&spec.service_node_id),
        binding_id(&spec.service_node_id),
        key_id(&spec.service_node_id),
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

fn advance_history(
    registry: &mut ServiceNodeEligibilityRegistry,
    service_node_id: &str,
    effective_epoch: u64,
    suffix: &str,
) {
    let current = registry
        .descriptor(service_node_id)
        .expect("descriptor should exist")
        .clone();

    registry
        .update_history_roots(ServiceNodeHistoryRootsUpdate {
            service_node_id: service_node_id.to_string(),
            expected_service_history_root: current.service_history_root,
            service_history_root: cid(&format!("service-history-{suffix}:{service_node_id}")),
            expected_challenge_history_root: current.challenge_history_root,
            challenge_history_root: cid(&format!("challenge-history-{suffix}:{service_node_id}")),
            effective_epoch,
        })
        .expect("history roots should advance");
}

fn apply_requested_lifecycle(registry: &mut ServiceNodeEligibilityRegistry, spec: &NodeSpec) {
    if let Some(probation_at_epoch) = spec.probation_at_epoch {
        let descriptor = registry
            .descriptor(&spec.service_node_id)
            .expect("candidate descriptor should exist")
            .clone();

        registry
            .evaluate_and_apply_policy(
                lifecycle_policy(),
                &observation(&descriptor, probation_at_epoch),
            )
            .expect("candidate should enter probation");
    }

    if let Some(eligible_at_epoch) = spec.eligible_at_epoch {
        let probation_at_epoch = spec
            .probation_at_epoch
            .expect("eligible fixture must first enter probation");

        advance_history(
            registry,
            &spec.service_node_id,
            probation_at_epoch + 1,
            "eligible",
        );

        let descriptor = registry
            .descriptor(&spec.service_node_id)
            .expect("probation descriptor should exist")
            .clone();

        registry
            .evaluate_and_apply_policy(
                lifecycle_policy(),
                &observation(&descriptor, eligible_at_epoch),
            )
            .expect("probation should become eligible");
    }

    if let Some(quarantine_at_epoch) = spec.quarantine_at_epoch {
        advance_history(
            registry,
            &spec.service_node_id,
            quarantine_at_epoch - 1,
            "quarantine",
        );

        let descriptor = registry
            .descriptor(&spec.service_node_id)
            .expect("eligible descriptor should exist")
            .clone();

        let mut challenged = observation(&descriptor, quarantine_at_epoch);

        challenged.upheld_challenges_in_state = 2;

        registry
            .evaluate_and_apply_policy(lifecycle_policy(), &challenged)
            .expect("upheld challenges should quarantine");
    }
}

fn build_registry(specs: &[NodeSpec]) -> ServiceNodeEligibilityRegistry {
    let mut reward_bindings =
        RewardBindingRegistry::new(cid("registry-root"), cid("reward-binding-root"));

    for spec in specs {
        reward_bindings
            .insert_binding(reward_binding(spec))
            .expect("reward binding should insert");
    }

    let mut registry = ServiceNodeEligibilityRegistry::new();

    for spec in specs {
        registry
            .register_candidate(candidate_descriptor(spec), &reward_bindings, OBSERVED_AT_MS)
            .expect("candidate should register");
    }

    for spec in specs {
        apply_requested_lifecycle(&mut registry, spec);
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
    let eligible_service_nodes =
        u16::try_from(member_count).expect("fixture member count should fit u16");

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
    let transition_hash = cid("phase18-cross-crate-transition");

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

fn economics() -> InternalRocRewardPlanningEconomics {
    load_internal_roc_planning_economics_toml(CANONICAL_ECONOMICS)
        .expect("canonical economics should load")
}

fn reward_candidate(service_node_id: &str, content_seed: &str) -> ServiceNodeRewardCandidate {
    ServiceNodeRewardCandidate {
        service_node_id: service_node_id.to_string(),
        evidence_class: ServiceNodeRewardEvidenceClass::Delivery,
        content_id: b3(content_seed),
        evidence_count: 1,
        eligible_score: 100,
        evidence_verified: true,
        accounting_accepted: true,
        policy_gate_passed: true,
        challenge_required: false,
        challenge_accepted: false,
    }
}

fn reward_input(
    economics: &InternalRocRewardPlanningEconomics,
    service_node_ids: &[String],
) -> ServiceNodeRewardPlanInput {
    ServiceNodeRewardPlanInput {
        epoch_id: "epoch:phase18:cross-crate".to_string(),
        accounting_snapshot_cid: b3("accounting-snapshot"),
        economics_config_hash: economics.economics_config_hash.clone(),
        policy_hash: b3("reward-policy"),
        pool_minor_units: AmountMinor(1_000_000),
        candidates: service_node_ids
            .iter()
            .enumerate()
            .map(|(index, service_node_id)| {
                reward_candidate(service_node_id, &format!("content-{index}"))
            })
            .collect(),
    }
}

fn descriptors(
    registry: &ServiceNodeEligibilityRegistry,
    service_node_ids: &[String],
) -> Vec<ServiceNodeIdentityDescriptorV1> {
    service_node_ids
        .iter()
        .map(|service_node_id| {
            registry
                .descriptor(service_node_id)
                .expect("descriptor should exist")
                .clone()
        })
        .collect()
}

fn node_total(result: &ServiceNodeEligibilityRewardPlan, service_node_id: &str) -> AmountMinor {
    result
        .plan
        .allocations
        .iter()
        .filter(|allocation| allocation.service_node_id == service_node_id)
        .fold(AmountMinor::ZERO, |total, allocation| {
            total
                .checked_add(allocation.amount_minor_units)
                .expect("allocation total should not overflow")
        })
}

fn ids(specs: &[NodeSpec]) -> Vec<String> {
    specs
        .iter()
        .map(|spec| spec.service_node_id.clone())
        .collect()
}

#[test]
fn probation_is_excluded_from_quorum_and_capped_in_rewards() {
    let specs = vec![mature(0), mature(1), mature(2), probation(0)];

    let registry = build_registry(&specs);
    let all_ids = ids(&specs);

    let invalid_quorum = quorum(&registry, &all_ids, &all_ids);

    let error = review_service_node_quorum_weight(
        &registry,
        &invalid_quorum,
        CURRENT_EPOCH,
        weight_policy(),
    )
    .expect_err("probation signer must not count toward quorum");

    assert!(matches!(
        error,
        ServiceNodeQuorumWeightError::LifecycleIneligible {
            state: ServiceNodeEligibilityStateV1::Probation,
            ..
        }
    ));

    let mature_ids = all_ids[..3].to_vec();
    let valid_quorum = quorum(&registry, &mature_ids, &mature_ids);

    let quorum_review =
        review_service_node_quorum_weight(&registry, &valid_quorum, CURRENT_EPOCH, weight_policy())
            .expect("mature registry quorum should pass");

    assert!(quorum_review.reaches_weighted_quorum());
    assert!(!quorum_review.authorizes_economic_mutation());

    let reward_ids = vec![
        specs[3].service_node_id.clone(),
        specs[0].service_node_id.clone(),
    ];

    let economics = economics();

    let plan = compute_service_node_reward_plan_with_eligibility(
        reward_input(&economics, &reward_ids),
        descriptors(&registry, &reward_ids),
        &economics,
    )
    .expect("probation and eligible reward review should compute");

    assert_eq!(
        node_total(&plan, &specs[3].service_node_id),
        economics.probation_reward_cap_minor_per_node_per_epoch
    );

    assert_eq!(
        node_total(&plan, &specs[0].service_node_id),
        economics.max_reward_minor_per_account_per_epoch
    );

    assert!(plan.planning_only);
    assert!(plan.registry_attestation_required);
    assert!(!plan.payout_authority);
    assert!(!plan.payout_executed);
    assert!(!plan.wallet_mutation);
    assert!(!plan.ledger_mutation);
    assert!(!plan.receipt_created);
    assert!(!plan.balance_truth);
}

#[test]
fn candidate_cannot_gain_quorum_or_reward_authority() {
    let specs = vec![mature(0), mature(1), mature(2), candidate(0)];

    let registry = build_registry(&specs);
    let all_ids = ids(&specs);

    let invalid_quorum = quorum(&registry, &all_ids, &all_ids);

    let quorum_error = review_service_node_quorum_weight(
        &registry,
        &invalid_quorum,
        CURRENT_EPOCH,
        weight_policy(),
    )
    .expect_err("candidate signer must reject");

    assert!(matches!(
        quorum_error,
        ServiceNodeQuorumWeightError::LifecycleIneligible {
            state: ServiceNodeEligibilityStateV1::Candidate,
            ..
        }
    ));

    let candidate_id = vec![specs[3].service_node_id.clone()];
    let economics = economics();

    let reward_error = compute_service_node_reward_plan_with_eligibility(
        reward_input(&economics, &candidate_id),
        descriptors(&registry, &candidate_id),
        &economics,
    )
    .expect_err("candidate reward planning must reject");

    assert!(reward_error
        .to_string()
        .contains("cannot enter reward planning"));
}

#[test]
fn quarantine_blocks_both_quorum_and_reward_planning() {
    let specs = vec![mature(0), mature(1), quarantined(0)];

    let registry = build_registry(&specs);
    let all_ids = ids(&specs);

    let invalid_quorum = quorum(&registry, &all_ids, &all_ids);

    let quorum_error = review_service_node_quorum_weight(
        &registry,
        &invalid_quorum,
        CURRENT_EPOCH,
        weight_policy(),
    )
    .expect_err("quarantined signer must reject");

    assert!(matches!(
        quorum_error,
        ServiceNodeQuorumWeightError::LifecycleIneligible {
            state: ServiceNodeEligibilityStateV1::Quarantined,
            ..
        }
    ));

    let quarantined_id = vec![specs[2].service_node_id.clone()];
    let economics = economics();

    let reward_error = compute_service_node_reward_plan_with_eligibility(
        reward_input(&economics, &quarantined_id),
        descriptors(&registry, &quarantined_id),
        &economics,
    )
    .expect_err("quarantined reward planning must reject");

    assert!(reward_error
        .to_string()
        .contains("cannot enter reward planning"));
}

#[test]
fn many_fresh_nodes_pass_count_but_cannot_control_weight() {
    let mut specs = vec![mature(0), mature(1)];

    for index in 0..20 {
        specs.push(fresh(index));
    }

    let registry = build_registry(&specs);
    let all_ids = ids(&specs);

    let fresh_ids = specs
        .iter()
        .filter(|spec| spec.service_node_id.contains(":fresh_"))
        .map(|spec| spec.service_node_id.clone())
        .collect::<Vec<_>>();

    assert!(
        fresh_ids.len() >= usize::from(threshold(all_ids.len()).required_signatures),
        "fresh identities must pass the old count-only threshold"
    );

    let fresh_only_quorum = quorum(&registry, &all_ids, &fresh_ids);

    let error = review_service_node_quorum_weight(
        &registry,
        &fresh_only_quorum,
        CURRENT_EPOCH,
        weight_policy(),
    )
    .expect_err("fresh signatures must not control weighted quorum");

    assert!(matches!(
        error,
        ServiceNodeQuorumWeightError::InsufficientWeightedQuorum { .. }
    ));
}
