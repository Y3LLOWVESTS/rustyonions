//! RO:WHAT — Phase 18B tests for registry-owned Service Node identity and objective history roots.
//! RO:WHY — Proves fresh identities cannot enter as trusted/eligible nodes or bypass their canonical reward binding.
//! RO:INTERACTS — svc-registry eligibility custody, reward-binding registry, and ron-proto Phase 18 descriptors.
//! RO:INVARIANTS — candidate-only registration; exact reward binding; monotonic history; no lifecycle or economic mutation.
//! RO:TEST — this file.

use ron_proto::{
    ContentId, RewardBindingSignatureRefV1, ServiceNodeEligibilityStateV1,
    ServiceNodeIdentityDescriptorV1, ServiceNodeRewardBindingV1,
    SERVICE_NODE_REWARD_BINDING_VERSION,
};
use svc_registry::{
    eligibility::{
        ServiceNodeEligibilityRegistry, ServiceNodeEligibilityRegistryError,
        ServiceNodeHistoryRootsUpdate,
    },
    rewards::RewardBindingRegistry,
};

const REGISTERED_EPOCH: u64 = 18;
const OBSERVED_AT_MS: u64 = 2_000;

fn cid(label: &str) -> ContentId {
    // Test-only deterministic material. ContentId validation requires canonical
    // b3 syntax here; these fixtures do not claim to be hashes of stored bytes.
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in label.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    format!("b3:{hash:016x}{hash:016x}{hash:016x}{hash:016x}")
        .parse()
        .expect("fixture CID should parse")
}

fn signature(label: &str) -> RewardBindingSignatureRefV1 {
    RewardBindingSignatureRefV1 {
        alg: "ed25519".to_string(),
        public_key_ref: format!("key:{label}"),
        signature: format!("signature-{label}"),
    }
}

fn binding(service_node_id: &str, binding_id: &str, nonce: &str) -> ServiceNodeRewardBindingV1 {
    ServiceNodeRewardBindingV1 {
        version: SERVICE_NODE_REWARD_BINDING_VERSION,
        binding_id: binding_id.to_string(),
        service_node_id: service_node_id.to_string(),
        operator_account_id: "acct_operator".to_string(),
        operator_display_address: "@operator".to_string(),
        operator_passport_id: Some("passport:operator".to_string()),
        reward_recipient_account_id: "acct_operator".to_string(),
        reward_recipient_display_address: "@operator".to_string(),
        node_public_key: format!("key:{service_node_id}"),
        operator_signature: signature("operator"),
        node_signature: signature(service_node_id),
        created_at_ms: 1_000,
        effective_epoch: REGISTERED_EPOCH,
        expires_at_ms: None,
        rotation_nonce: nonce.to_string(),
        policy_hash: cid("binding-policy"),
    }
}

fn reward_bindings_for(
    service_node_id: &str,
    binding_id: &str,
    nonce: &str,
) -> RewardBindingRegistry {
    let mut registry = RewardBindingRegistry::new(cid("registry-root"), cid("binding-root"));

    registry
        .insert_binding(binding(service_node_id, binding_id, nonce))
        .expect("valid reward binding should insert");

    registry
}

fn descriptor(
    service_node_id: &str,
    registry_entry_id: &str,
    binding_id: &str,
) -> ServiceNodeIdentityDescriptorV1 {
    ServiceNodeIdentityDescriptorV1::new_candidate(
        service_node_id.to_string(),
        registry_entry_id.to_string(),
        binding_id.to_string(),
        format!("key:{service_node_id}"),
        REGISTERED_EPOCH,
        cid(&format!("{service_node_id}-service-history-0")),
        cid(&format!("{service_node_id}-challenge-history-0")),
    )
    .expect("candidate descriptor should validate")
}

#[test]
fn candidate_registration_requires_exact_active_reward_binding() {
    let service_node_id = "service_node:node_01";
    let binding_id = "binding:node_01:1";
    let reward_bindings = reward_bindings_for(service_node_id, binding_id, "nonce_node_01_1");
    let candidate = descriptor(service_node_id, "registry:node_01", binding_id);

    let mut registry = ServiceNodeEligibilityRegistry::new();
    registry
        .register_candidate(candidate.clone(), &reward_bindings, OBSERVED_AT_MS)
        .expect("bound candidate should register");

    assert_eq!(registry.len(), 1);
    assert_eq!(registry.descriptor(service_node_id), Some(&candidate));
    assert_eq!(
        registry.history_effective_epoch(service_node_id),
        Some(REGISTERED_EPOCH)
    );
    assert_eq!(
        registry
            .descriptor(service_node_id)
            .expect("descriptor should exist")
            .state,
        ServiceNodeEligibilityStateV1::Candidate
    );
}

#[test]
fn unbound_service_node_cannot_enter_eligibility_registry() {
    let service_node_id = "service_node:node_01";
    let candidate = descriptor(service_node_id, "registry:node_01", "binding:node_01:1");
    let reward_bindings = RewardBindingRegistry::new(cid("registry-root"), cid("binding-root"));

    let error = ServiceNodeEligibilityRegistry::new()
        .register_candidate(candidate, &reward_bindings, OBSERVED_AT_MS)
        .expect_err("unbound node must reject");

    assert!(matches!(
        error,
        ServiceNodeEligibilityRegistryError::RewardBindingUnresolved { .. }
    ));
}

#[test]
fn descriptor_cannot_reference_a_different_reward_binding() {
    let service_node_id = "service_node:node_01";
    let reward_bindings = reward_bindings_for(
        service_node_id,
        "binding:node_01:actual",
        "nonce_node_01_actual",
    );
    let candidate = descriptor(
        service_node_id,
        "registry:node_01",
        "binding:node_01:smuggled",
    );

    let error = ServiceNodeEligibilityRegistry::new()
        .register_candidate(candidate, &reward_bindings, OBSERVED_AT_MS)
        .expect_err("mismatched binding must reject");

    assert!(matches!(
        error,
        ServiceNodeEligibilityRegistryError::RewardBindingMismatch { .. }
    ));
}

#[test]
fn new_registry_identity_cannot_enter_as_eligible_or_manually_trusted() {
    let service_node_id = "service_node:node_01";
    let binding_id = "binding:node_01:1";
    let reward_bindings = reward_bindings_for(service_node_id, binding_id, "nonce_node_01_1");
    let mut non_candidate = descriptor(service_node_id, "registry:node_01", binding_id);
    non_candidate.state = ServiceNodeEligibilityStateV1::Eligible;

    let error = ServiceNodeEligibilityRegistry::new()
        .register_candidate(non_candidate, &reward_bindings, OBSERVED_AT_MS)
        .expect_err("fresh eligible identity must reject");

    assert!(matches!(
        error,
        ServiceNodeEligibilityRegistryError::InitialStateMustBeCandidate {
            actual: ServiceNodeEligibilityStateV1::Eligible
        }
    ));
}

#[test]
fn duplicate_service_node_and_registry_entry_ids_are_rejected() {
    let first_node = "service_node:node_01";
    let first_binding = "binding:node_01:1";
    let first_bindings = reward_bindings_for(first_node, first_binding, "nonce_node_01_1");

    let mut registry = ServiceNodeEligibilityRegistry::new();
    registry
        .register_candidate(
            descriptor(first_node, "registry:shared", first_binding),
            &first_bindings,
            OBSERVED_AT_MS,
        )
        .expect("first candidate should register");

    let duplicate_node = registry
        .register_candidate(
            descriptor(first_node, "registry:node_01_other", first_binding),
            &first_bindings,
            OBSERVED_AT_MS,
        )
        .expect_err("duplicate Service Node identity must reject");

    assert!(matches!(
        duplicate_node,
        ServiceNodeEligibilityRegistryError::DuplicateServiceNodeId { .. }
    ));

    let second_node = "service_node:node_02";
    let second_binding = "binding:node_02:1";
    let second_bindings = reward_bindings_for(second_node, second_binding, "nonce_node_02_1");

    let duplicate_entry = registry
        .register_candidate(
            descriptor(second_node, "registry:shared", second_binding),
            &second_bindings,
            OBSERVED_AT_MS,
        )
        .expect_err("duplicate registry entry must reject");

    assert!(matches!(
        duplicate_entry,
        ServiceNodeEligibilityRegistryError::DuplicateRegistryEntryId { .. }
    ));
}

#[test]
fn objective_history_roots_advance_without_promoting_candidate_state() {
    let service_node_id = "service_node:node_01";
    let binding_id = "binding:node_01:1";
    let reward_bindings = reward_bindings_for(service_node_id, binding_id, "nonce_node_01_1");
    let candidate = descriptor(service_node_id, "registry:node_01", binding_id);

    let original_service_root = candidate.service_history_root.clone();
    let original_challenge_root = candidate.challenge_history_root.clone();
    let original_key_id = candidate.key_id.clone();

    let mut registry = ServiceNodeEligibilityRegistry::new();
    registry
        .register_candidate(candidate, &reward_bindings, OBSERVED_AT_MS)
        .expect("candidate should register");

    let updated = registry
        .update_history_roots(ServiceNodeHistoryRootsUpdate {
            service_node_id: service_node_id.to_string(),
            expected_service_history_root: original_service_root,
            service_history_root: cid("service-history-1"),
            expected_challenge_history_root: original_challenge_root,
            challenge_history_root: cid("challenge-history-1"),
            effective_epoch: 19,
        })
        .expect("later objective roots should update");

    assert_eq!(updated.state, ServiceNodeEligibilityStateV1::Candidate);
    assert_eq!(updated.reward_binding_id, binding_id);
    assert_eq!(updated.key_id, original_key_id);
    assert_eq!(registry.history_effective_epoch(service_node_id), Some(19));
    assert!(!updated.state.counts_toward_quorum());
}

#[test]
fn stale_history_roots_and_epoch_regression_are_rejected() {
    let service_node_id = "service_node:node_01";
    let binding_id = "binding:node_01:1";
    let reward_bindings = reward_bindings_for(service_node_id, binding_id, "nonce_node_01_1");
    let candidate = descriptor(service_node_id, "registry:node_01", binding_id);

    let original_service_root = candidate.service_history_root.clone();
    let original_challenge_root = candidate.challenge_history_root.clone();

    let mut registry = ServiceNodeEligibilityRegistry::new();
    registry
        .register_candidate(candidate, &reward_bindings, OBSERVED_AT_MS)
        .expect("candidate should register");

    registry
        .update_history_roots(ServiceNodeHistoryRootsUpdate {
            service_node_id: service_node_id.to_string(),
            expected_service_history_root: original_service_root.clone(),
            service_history_root: cid("service-history-1"),
            expected_challenge_history_root: original_challenge_root.clone(),
            challenge_history_root: cid("challenge-history-1"),
            effective_epoch: 19,
        })
        .expect("first history update should succeed");

    let stale = registry
        .update_history_roots(ServiceNodeHistoryRootsUpdate {
            service_node_id: service_node_id.to_string(),
            expected_service_history_root: original_service_root,
            service_history_root: cid("service-history-2"),
            expected_challenge_history_root: original_challenge_root,
            challenge_history_root: cid("challenge-history-2"),
            effective_epoch: 20,
        })
        .expect_err("stale expected roots must reject");

    assert!(matches!(
        stale,
        ServiceNodeEligibilityRegistryError::ServiceHistoryRootMismatch { .. }
    ));

    let current = registry
        .descriptor(service_node_id)
        .expect("descriptor should remain registered")
        .clone();

    let regression = registry
        .update_history_roots(ServiceNodeHistoryRootsUpdate {
            service_node_id: service_node_id.to_string(),
            expected_service_history_root: current.service_history_root.clone(),
            service_history_root: cid("service-history-2"),
            expected_challenge_history_root: current.challenge_history_root.clone(),
            challenge_history_root: cid("challenge-history-2"),
            effective_epoch: 19,
        })
        .expect_err("same or older history epoch must reject");

    assert!(matches!(
        regression,
        ServiceNodeEligibilityRegistryError::HistoryEpochRegression {
            current_epoch: 19,
            proposed_epoch: 19,
            ..
        }
    ));
}

#[test]
fn unchanged_history_roots_are_rejected() {
    let service_node_id = "service_node:node_01";
    let binding_id = "binding:node_01:1";
    let reward_bindings = reward_bindings_for(service_node_id, binding_id, "nonce_node_01_1");
    let candidate = descriptor(service_node_id, "registry:node_01", binding_id);

    let service_root = candidate.service_history_root.clone();
    let challenge_root = candidate.challenge_history_root.clone();

    let mut registry = ServiceNodeEligibilityRegistry::new();
    registry
        .register_candidate(candidate, &reward_bindings, OBSERVED_AT_MS)
        .expect("candidate should register");

    let error = registry
        .update_history_roots(ServiceNodeHistoryRootsUpdate {
            service_node_id: service_node_id.to_string(),
            expected_service_history_root: service_root.clone(),
            service_history_root: service_root,
            expected_challenge_history_root: challenge_root.clone(),
            challenge_history_root: challenge_root,
            effective_epoch: 19,
        })
        .expect_err("no-op root update must reject");

    assert!(matches!(
        error,
        ServiceNodeEligibilityRegistryError::HistoryRootsUnchanged { .. }
    ));
}
