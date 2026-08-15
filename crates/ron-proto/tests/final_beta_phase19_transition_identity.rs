//! RO:WHAT — Focused FINAL_BETA Phase 19 tests for canonical epoch-transition identity.
//! RO:WHY — Service Nodes need one exact pre-sign bytes contract before live quorum signing is enabled.
//! RO:INTERACTS — ron-proto epoch expectations, allocations, eligibility, and historical transition domains.
//! RO:INVARIANTS — exact legacy bytes preserved; no hash/signature/timestamp authority enters the identity.
//! RO:SECURITY — validation only; no wallet, ledger, payout, receipt, mint, or finality authority.
//! RO:TEST — this file.

use ron_proto::{
    ContentId, EpochEligibilityStatusV1, EpochEligibilityV1, EpochQuorumThresholdV1,
    EpochRewardAllocationV1, RocEpochTransitionExpectationV1, RocEpochTransitionIdentityV1,
    EPOCH_REWARD_ALLOCATION_SCHEMA, ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA,
    ROC_EPOCH_TRANSITION_HASH_DOMAIN, ROC_EPOCH_TRANSITION_IDENTITY_DOMAIN,
    ROC_EPOCH_TRANSITION_VERSION,
};
use serde::Serialize;

fn cid(character: char) -> ContentId {
    format!("b3:{}", character.to_string().repeat(64))
        .parse()
        .expect("test content id must parse")
}

fn eligibility(
    service_node_id: &str,
    registry_entry_id: &str,
    reward_binding_id: &str,
    key_id: &str,
) -> EpochEligibilityV1 {
    EpochEligibilityV1 {
        version: ROC_EPOCH_TRANSITION_VERSION,
        service_node_id: service_node_id.to_owned(),
        registry_entry_id: registry_entry_id.to_owned(),
        reward_binding_id: reward_binding_id.to_owned(),
        key_id: key_id.to_owned(),
        status: EpochEligibilityStatusV1::Eligible,
    }
}

fn threshold() -> EpochQuorumThresholdV1 {
    EpochQuorumThresholdV1 {
        version: ROC_EPOCH_TRANSITION_VERSION,
        eligible_service_nodes: 3,
        quorum_bps: 6_666,
        minimum_signatures: 2,
        required_signatures: 2,
    }
}

fn valid_expectation() -> RocEpochTransitionExpectationV1 {
    RocEpochTransitionExpectationV1 {
        schema: ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA.to_owned(),
        version: ROC_EPOCH_TRANSITION_VERSION,
        chain_id: "rustyonions-dev".to_owned(),
        epoch_id: "epoch:19".to_owned(),
        accounting_snapshot_hash: cid('a'),
        reward_plan_hash: cid('b'),
        policy_hash: cid('c'),
        economics_config_hash: cid('d'),
        registry_root: cid('e'),
        reward_binding_root: cid('f'),
        evidence_root: cid('1'),
        reward_cap_minor_units: "1000".to_owned(),
        threshold: threshold(),
        eligibilities: vec![
            eligibility(
                "service_node:alpha",
                "registry:alpha",
                "binding:alpha",
                "key:alpha",
            ),
            eligibility(
                "service_node:beta",
                "registry:beta",
                "binding:beta",
                "key:beta",
            ),
            eligibility(
                "service_node:gamma",
                "registry:gamma",
                "binding:gamma",
                "key:gamma",
            ),
        ],
    }
}

fn allocation(
    allocation_id: &str,
    reward_plan_allocation_id: &str,
    service_node_id: &str,
    source_pool: &str,
    amount_minor_units: &str,
) -> EpochRewardAllocationV1 {
    EpochRewardAllocationV1 {
        schema: EPOCH_REWARD_ALLOCATION_SCHEMA.to_owned(),
        version: ROC_EPOCH_TRANSITION_VERSION,
        allocation_id: allocation_id.to_owned(),
        reward_plan_allocation_id: reward_plan_allocation_id.to_owned(),
        service_node_id: service_node_id.to_owned(),
        source_pool: source_pool.to_owned(),
        amount_minor_units: amount_minor_units.to_owned(),
    }
}

fn allocations() -> Vec<EpochRewardAllocationV1> {
    vec![
        allocation(
            "allocation:alpha",
            "reward_plan_allocation:alpha",
            "service_node:alpha",
            "node_delivery",
            "400",
        ),
        allocation(
            "allocation:beta",
            "reward_plan_allocation:beta",
            "service_node:beta",
            "node_delivery",
            "600",
        ),
    ]
}

fn valid_identity() -> RocEpochTransitionIdentityV1 {
    RocEpochTransitionIdentityV1::from_expectation_and_allocations(
        &valid_expectation(),
        "1000",
        allocations(),
    )
}

#[derive(Serialize)]
struct LegacyTransitionIdentity<'a> {
    domain: &'a str,
    chain_id: &'a str,
    epoch_id: &'a str,
    accounting_snapshot_hash: &'a ContentId,
    reward_plan_hash: &'a ContentId,
    policy_hash: &'a ContentId,
    economics_config_hash: &'a ContentId,
    registry_root: &'a ContentId,
    reward_binding_root: &'a ContentId,
    evidence_root: &'a ContentId,
    reward_cap_minor_units: &'a str,
    reward_total_minor_units: &'a str,
    allocations: &'a [EpochRewardAllocationV1],
    threshold: &'a EpochQuorumThresholdV1,
    eligibilities: &'a [EpochEligibilityV1],
}

#[test]
fn phase19_shared_identity_preserves_exact_legacy_json_bytes() {
    let identity = valid_identity();

    identity
        .validate()
        .expect("valid pre-sign identity must validate");

    let legacy = LegacyTransitionIdentity {
        domain: ROC_EPOCH_TRANSITION_IDENTITY_DOMAIN,
        chain_id: &identity.chain_id,
        epoch_id: &identity.epoch_id,
        accounting_snapshot_hash: &identity.accounting_snapshot_hash,
        reward_plan_hash: &identity.reward_plan_hash,
        policy_hash: &identity.policy_hash,
        economics_config_hash: &identity.economics_config_hash,
        registry_root: &identity.registry_root,
        reward_binding_root: &identity.reward_binding_root,
        evidence_root: &identity.evidence_root,
        reward_cap_minor_units: &identity.reward_cap_minor_units,
        reward_total_minor_units: &identity.reward_total_minor_units,
        allocations: &identity.allocations,
        threshold: &identity.threshold,
        eligibilities: &identity.eligibilities,
    };

    let shared_bytes = serde_json::to_vec(&identity).expect("shared identity must serialize");

    let legacy_bytes = serde_json::to_vec(&legacy).expect("legacy identity must serialize");

    assert_eq!(shared_bytes, legacy_bytes);
}

#[test]
fn phase19_historical_transition_domains_are_locked_for_compatibility() {
    assert_eq!(
        ROC_EPOCH_TRANSITION_IDENTITY_DOMAIN,
        "rustyonions.phase22.epoch-transition.v1"
    );

    assert_eq!(
        ROC_EPOCH_TRANSITION_HASH_DOMAIN,
        "phase22.epoch-transition.v1"
    );
}

#[test]
fn phase19_identity_excludes_post_review_authority_fields() {
    let identity = valid_identity();

    let value = serde_json::to_value(identity).expect("identity must serialize");

    let object = value
        .as_object()
        .expect("identity must serialize as an object");

    assert!(!object.contains_key("transition_hash"));
    assert!(!object.contains_key("signatures"));
    assert!(!object.contains_key("produced_at_ms"));
    assert!(!object.contains_key("recipient_account_id"));
    assert!(!object.contains_key("wallet_mutation"));
    assert!(!object.contains_key("ledger_mutation"));
    assert!(!object.contains_key("finality"));
}

#[test]
fn phase19_identity_rejects_wrong_domain() {
    let mut identity = valid_identity();

    identity.domain = "rustyonions.phase19.unreviewed-transition.v1".to_owned();

    assert!(identity.validate().is_err());
}

#[test]
fn phase19_identity_reuses_reviewed_quorum_threshold_validation() {
    let mut identity = valid_identity();

    identity.threshold.required_signatures = 3;

    assert!(identity.validate().is_err());
}

#[test]
fn phase19_identity_rejects_allocation_sum_drift() {
    let mut identity = valid_identity();

    identity.reward_total_minor_units = "999".to_owned();

    assert!(identity.validate().is_err());
}

#[test]
fn phase19_identity_rejects_duplicate_allocation_identity() {
    let mut identity = valid_identity();

    identity.allocations[1].allocation_id = identity.allocations[0].allocation_id.clone();

    assert!(identity.validate().is_err());
}

#[test]
fn phase19_identity_rejects_noncanonical_allocation_order() {
    let mut identity = valid_identity();

    identity.allocations.swap(0, 1);

    assert!(identity.validate().is_err());
}

#[test]
fn phase19_identity_rejects_allocation_for_unreviewed_service_node() {
    let mut identity = valid_identity();

    identity.allocations[1].service_node_id = "service_node:delta".to_owned();

    assert!(identity.validate().is_err());
}
