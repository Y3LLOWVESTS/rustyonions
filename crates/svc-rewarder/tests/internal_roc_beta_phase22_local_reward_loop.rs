//! RO:WHAT — Phase 22G complete local ROC reward-loop composition proof.
//! RO:WHY — Proves real evidence reaches canonical accounting, capped planning,
//! registry resolution, quorum, wallet/ledger receipts, and independent User
//! Node replay.
//! RO:INTERACTS — ron-policy, ron-accounting, svc-rewarder, svc-registry,
//! ron-proto, ron-kms, svc-wallet, ron-ledger, and micronode.
//! RO:INVARIANTS — one canonical economics hash; no arbitrary recipient;
//! no single-node mint; exact retry is idempotent; replay conserves supply.
//! RO:SECURITY — real Ed25519 quorum signatures; self-issuance rejected;
//! tampered replay creates challenge evidence.
//! RO:TEST — cargo test -p svc-rewarder --test
//! internal_roc_beta_phase22_local_reward_loop.

#![allow(clippy::missing_panics_doc)]

use std::collections::BTreeMap;

use micronode::economic_audit::{
    build_invalid_epoch_challenge, review_epoch_transition_with_signatures,
    EpochQuorumKeyResolver as UserNodeQuorumKeyResolver, EpochReplayObservationV1,
    UserNodeEpochReviewStatusV1, USER_NODE_EPOCH_REPLAY_OBSERVATION_SCHEMA,
    USER_NODE_EPOCH_REPLAY_VERSION,
};
use ron_accounting::{
    build_accounting_epoch_snapshot, canonical_accounting_epoch_snapshot_artifact_cid,
    classify_service_evidence_batch, classify_user_verification_batch,
    AccountingEconomicsConfigBindingV1, AccountingEconomicsProfileV1, AccountingEpochWindowV1,
    EvidenceContentId, ServiceEvidenceAccountingInputV1, ServiceEvidenceAccountingKindV1,
    UserVerificationAccountingInputV1, UserVerificationEvidenceKindV1, UserVerificationResultV1,
    SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA, SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION,
    USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA, USER_VERIFICATION_ACCOUNTING_INPUT_VERSION,
};
use ron_kms::{backends::memory::MemoryKeystore, KeyId, Keystore, Signer};
use ron_ledger::replay_epoch_payout_receipts;
use ron_policy::economics::{internal_roc_economics_config_hash, load_internal_roc_economics_toml};
use ron_proto::{
    ContentId, EpochEligibilityStatusV1, EpochEligibilityV1, EpochQuorumThresholdV1,
    EpochRewardAllocationV1, InvalidEpochChallengeKindV1, RewardBindingSignatureRefV1,
    RocEpochTransitionExpectationV1, RocEpochTransitionV1, ServiceNodeQuorumV1,
    ServiceNodeRewardBindingV1, ServiceNodeSignatureV1, SignatureAlg,
    EPOCH_REWARD_ALLOCATION_SCHEMA, ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA,
    ROC_EPOCH_TRANSITION_SCHEMA, ROC_EPOCH_TRANSITION_VERSION, SERVICE_NODE_REWARD_BINDING_VERSION,
};
use serde::Serialize;
use svc_registry::rewards::{
    preview_reward_payout_authorization, RewardBindingRegistry, RewardPayoutAuthorizationError,
    RewardPayoutAuthorizationPreview, SelfIssuanceMode,
};
use svc_rewarder::{
    core::{compute_service_node_reward_plan, AmountMinor, ServiceNodeRewardPlan},
    inputs::{
        accounting_epoch_reward_planning_handoff, load_internal_roc_planning_economics_toml,
        InternalRocRewardPlanningEconomics,
    },
};
use svc_wallet::{
    epoch_execution::{
        execute_epoch_transition, prepare_epoch_payout_operations,
        service_node_signature_message_bytes, EpochTransitionExecutionRequest, QuorumKeyResolver,
    },
    errors::{WalletError, WalletResult},
    ledger::client::LocalLedgerClient,
};

const CANONICAL_ECONOMICS: &[u8] = include_bytes!("../../../configs/roc-economics.toml");

const CHAIN_ID: &str = "rustyonions-dev";
const EPOCH_ID: &str = "epoch:22";
const EPOCH_NUMBER: u64 = 22;

const ALPHA_NODE: &str = "service_node:alpha";

const BETA_NODE: &str = "service_node:beta";

const GAMMA_NODE: &str = "service_node:gamma";

const ALPHA_ACCOUNT: &str = "acct_phase22_alpha";

fn confirmed_roc_projection_path(raw_path: &str) -> &str {
    let path = raw_path.trim();

    assert!(
        !path.is_empty(),
        "PHASE22_CONFIRMED_ROC_PROJECTION_PATH must not be empty",
    );

    path
}

#[test]
#[should_panic(expected = "PHASE22_CONFIRMED_ROC_PROJECTION_PATH must not be empty")]
fn confirmed_roc_projection_rejects_empty_export_path() {
    let _ = confirmed_roc_projection_path(" \t ");
}

fn confirmed_roc_projection_ledger_root(raw_root: &str) -> String {
    let root = raw_root.trim();

    assert!(
        root.len() == 64
            && root
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')),
        "Phase 22 replay ledger root must be 64 lowercase hex",
    );

    format!("b3:{root}")
}

#[test]
fn confirmed_roc_projection_ledger_root_uses_canonical_b3_form() {
    let raw_root = "a".repeat(64);

    assert_eq!(
        confirmed_roc_projection_ledger_root(&raw_root),
        format!("b3:{raw_root}"),
    );
}

#[test]
#[should_panic(expected = "Phase 22 replay ledger root must be 64 lowercase hex")]
fn confirmed_roc_projection_ledger_root_rejects_prefixed_input() {
    let raw_root = format!("b3:{}", "a".repeat(64));

    let _ = confirmed_roc_projection_ledger_root(&raw_root);
}

struct QuorumKeys {
    keys: BTreeMap<String, KeyId>,
}

impl QuorumKeyResolver for QuorumKeys {
    fn resolve_key(&self, key_ref: &str) -> WalletResult<KeyId> {
        self.keys.get(key_ref).cloned().ok_or_else(|| {
            WalletError::forbidden(format!("unknown Phase 22 quorum key reference {key_ref}"))
        })
    }
}

impl UserNodeQuorumKeyResolver for QuorumKeys {
    fn resolve_key(&self, key_ref: &str) -> Option<KeyId> {
        self.keys.get(key_ref).cloned()
    }
}

fn content_id_from_bytes(domain: &str, bytes: &[u8]) -> ContentId {
    let mut hasher = blake3::Hasher::new();

    hasher.update(domain.as_bytes());
    hasher.update(&[0]);
    hasher.update(bytes);

    ContentId::parse(&format!("b3:{}", hasher.finalize().to_hex(),))
        .expect("generated content ID must be canonical")
}

fn content_id_from_value<T>(domain: &str, value: &T) -> ContentId
where
    T: Serialize,
{
    let bytes = serde_json::to_vec(value).expect("Phase 22 identity input must serialize");

    content_id_from_bytes(domain, &bytes)
}

fn evidence_content_id(content_id: &ContentId) -> EvidenceContentId {
    content_id
        .as_str()
        .parse()
        .expect("canonical evidence content ID")
}

fn service_input(content_id: &ContentId) -> ServiceEvidenceAccountingInputV1 {
    ServiceEvidenceAccountingInputV1 {
        schema: SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA.to_owned(),

        version: SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION,

        sequence: 1,

        kind: ServiceEvidenceAccountingKindV1::Delivery,

        proof_id: "service:evidence:phase22:0001".to_owned(),

        service_node_id: ALPHA_NODE.to_owned(),

        witness_node_id: "user_node:phase22_verifier".to_owned(),

        related_actor_ids: Vec::new(),

        content_id: evidence_content_id(content_id),

        observed_at_ms: 1_100,

        signature_verified: true,
        evidence_only: true,

        accounting_accepted: false,
        reward_eligible: false,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

fn user_input(content_id: &ContentId) -> UserVerificationAccountingInputV1 {
    UserVerificationAccountingInputV1 {
        schema: USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA.to_owned(),

        version: USER_VERIFICATION_ACCOUNTING_INPUT_VERSION,

        sequence: 1,

        evidence_id: "user:verification:phase22:0001".to_owned(),

        user_node_id: "user_node:phase22_verifier".to_owned(),

        subject_ref: "oap_object:phase22:0001".to_owned(),

        verification_kind: UserVerificationEvidenceKindV1::B3IntegrityChallenge,

        observed_at_ms: 1_200,

        input_digest: evidence_content_id(content_id),

        result: UserVerificationResultV1::VerifiedValid,

        failure_reason: None,

        nonce: "nonce:phase22:0001".to_owned(),

        idempotency_key: "idempotency:phase22:0001".to_owned(),

        capability_id: "capability:phase22:verification".to_owned(),

        privacy_route_id: Some("relay:phase22g".to_owned()),

        attestation_ref: "attestation:phase22:0001".to_owned(),

        local_attestation_verified: true,
        evidence_only: true,

        accounting_accepted: false,
        reward_eligible: false,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

fn signature_ref(label: &str) -> RewardBindingSignatureRefV1 {
    RewardBindingSignatureRefV1 {
        alg: "ed25519".to_owned(),

        public_key_ref: format!("key:binding:{label}"),

        signature: format!("signature-{label}"),
    }
}

fn binding(
    service_node_id: &str,
    label: &str,
    account_id: &str,
    policy_hash: &ContentId,
) -> ServiceNodeRewardBindingV1 {
    ServiceNodeRewardBindingV1 {
        version: SERVICE_NODE_REWARD_BINDING_VERSION,

        binding_id: format!("binding:phase22:{label}"),

        service_node_id: service_node_id.to_owned(),

        operator_account_id: format!("operator_account_phase22_{label}"),

        operator_display_address: format!("@{label}"),

        operator_passport_id: Some(format!("passport:phase22:{label}")),

        reward_recipient_account_id: account_id.to_owned(),

        reward_recipient_display_address: format!("@{label}"),

        node_public_key: format!("node-public-key-phase22-{label}"),

        operator_signature: signature_ref(&format!("operator-{label}")),

        node_signature: signature_ref(&format!("node-{label}")),

        created_at_ms: 1_000,
        effective_epoch: EPOCH_NUMBER,
        expires_at_ms: None,

        rotation_nonce: format!("rotation_nonce_phase22_{label}"),

        policy_hash: policy_hash.clone(),
    }
}

fn eligibility(label: &str, service_node_id: &str) -> EpochEligibilityV1 {
    EpochEligibilityV1 {
        version: ROC_EPOCH_TRANSITION_VERSION,

        service_node_id: service_node_id.to_owned(),

        registry_entry_id: format!("registry:phase22:{label}"),

        reward_binding_id: format!("binding:phase22:{label}"),

        key_id: format!("key:phase22:{label}"),

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

fn signed_signature(
    kms: &MemoryKeystore,
    concrete_key: &KeyId,
    logical_key_ref: &str,
    service_node_id: &str,
    transition_hash: &ContentId,
) -> ServiceNodeSignatureV1 {
    let mut signature = ServiceNodeSignatureV1 {
        version: ROC_EPOCH_TRANSITION_VERSION,

        chain_id: CHAIN_ID.to_owned(),

        epoch_id: EPOCH_ID.to_owned(),

        service_node_id: service_node_id.to_owned(),

        key_id: logical_key_ref.to_owned(),

        algorithm: SignatureAlg::Ed25519,

        transition_hash: transition_hash.clone(),

        signature_wire: "00".repeat(64),
    };

    let message = service_node_signature_message_bytes(&signature)
        .expect("Phase 22 signature message must encode");

    signature.signature_wire = hex::encode(
        kms.sign(concrete_key, &message)
            .expect("Phase 22 quorum key must sign"),
    );

    signature
}

fn transition_allocations(plan: &ServiceNodeRewardPlan) -> Vec<EpochRewardAllocationV1> {
    plan.allocations
        .iter()
        .enumerate()
        .map(|(index, allocation)| EpochRewardAllocationV1 {
            schema: EPOCH_REWARD_ALLOCATION_SCHEMA.to_owned(),

            version: ROC_EPOCH_TRANSITION_VERSION,

            allocation_id: format!("allocation:phase22:{index:04}"),

            reward_plan_allocation_id: format!("reward_plan_allocation:phase22:{index:04}"),

            service_node_id: allocation.service_node_id.clone(),

            source_pool: allocation.reward_category.clone(),

            amount_minor_units: allocation.amount_minor_units.get().to_string(),
        })
        .collect()
}

#[derive(Serialize)]
struct TransitionIdentity<'a> {
    domain: &'static str,
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
fn complete_local_reward_loop_executes_replays_and_challenges_tampering() {
    let policy_config = load_internal_roc_economics_toml(CANONICAL_ECONOMICS)
        .expect("ron-policy must validate canonical economics");

    let policy_economics_hash = internal_roc_economics_config_hash(&policy_config)
        .expect("ron-policy must hash canonical economics");

    let economics: InternalRocRewardPlanningEconomics =
        load_internal_roc_planning_economics_toml(CANONICAL_ECONOMICS)
            .expect("svc-rewarder must project ron-policy economics");

    assert_eq!(economics.economics_config_hash, policy_economics_hash,);

    assert!(economics.bridge_inert);
    assert!(economics.staking_inert);

    let object_id = content_id_from_bytes("phase22.service-object.v1", b"abc");

    let service_evidence = service_input(&object_id);

    let user_evidence = user_input(&object_id);

    let service_batch = classify_service_evidence_batch(std::slice::from_ref(&service_evidence))
        .expect("Service Node evidence must classify");

    let user_batch = classify_user_verification_batch(std::slice::from_ref(&user_evidence))
        .expect("User Node verification must classify");

    let economics_binding = AccountingEconomicsConfigBindingV1::from_validated_model(
        economics.schema.clone(),
        economics.version,
        AccountingEconomicsProfileV1::Canonical,
        &economics.economics_config_hash,
    )
    .expect("accounting economics binding");

    let snapshot = build_accounting_epoch_snapshot(
        AccountingEpochWindowV1 {
            epoch_id: EPOCH_ID.to_owned(),

            starts_at_ms: 1_000,
            ends_at_ms: 2_000,
            produced_at_ms: 2_100,
        },
        &economics_binding,
        Some(&service_batch),
        Some(&user_batch),
    )
    .expect("canonical accounting snapshot");

    let snapshot_cid = canonical_accounting_epoch_snapshot_artifact_cid(&snapshot)
        .expect("accounting snapshot CID");

    let accounting_snapshot_hash =
        ContentId::parse(&snapshot_cid).expect("accounting snapshot content ID");

    let policy_hash = content_id_from_value(
        "phase22.reward-policy.v1",
        &(policy_economics_hash.as_str(), EPOCH_ID),
    );

    let reward_policy = economics
        .to_reward_policy("policy:phase22:reward-loop", policy_hash.as_str())
        .expect("config-bound reward policy");

    let first_handoff = accounting_epoch_reward_planning_handoff(
        &snapshot,
        &reward_policy,
        AmountMinor(1_000_000),
        &economics,
    )
    .expect("accounting to rewarder handoff");

    let second_handoff = accounting_epoch_reward_planning_handoff(
        &snapshot,
        &reward_policy,
        AmountMinor(1_000_000),
        &economics,
    )
    .expect("deterministic accounting handoff");

    assert_eq!(first_handoff, second_handoff,);

    assert!(first_handoff.planning_only);
    assert!(!first_handoff.payout_authority);
    assert!(!first_handoff.wallet_mutation);
    assert!(!first_handoff.ledger_mutation);

    let plan_input = first_handoff
        .service_node_plan_input
        .clone()
        .expect("proof-eligible Service Node plan input");

    let first_plan =
        compute_service_node_reward_plan(plan_input.clone(), &economics).expect("reward plan");

    let second_plan = compute_service_node_reward_plan(plan_input, &economics)
        .expect("deterministic reward plan");

    assert_eq!(first_plan, second_plan);

    assert_eq!(first_plan.allocations.len(), 1,);

    assert_eq!(first_plan.allocations[0].service_node_id, ALPHA_NODE,);

    assert!(first_plan.planning_only);

    assert!(first_plan.registry_resolution_required);

    assert!(!first_plan.payout_authority);
    assert!(!first_plan.payout_executed);
    assert!(!first_plan.wallet_mutation);
    assert!(!first_plan.ledger_mutation);
    assert!(!first_plan.receipt_created);
    assert!(!first_plan.balance_truth);

    let registry_root = content_id_from_value(
        "phase22.registry-root.v1",
        &(EPOCH_ID, "service-node-registry"),
    );

    let reward_binding_root = content_id_from_value(
        "phase22.reward-binding-root.v1",
        &(EPOCH_ID, "reward-bindings"),
    );

    let mut reward_bindings =
        RewardBindingRegistry::new(registry_root.clone(), reward_binding_root.clone());

    reward_bindings
        .insert_binding(binding(ALPHA_NODE, "alpha", ALPHA_ACCOUNT, &policy_hash))
        .expect("alpha reward binding");

    reward_bindings
        .insert_binding(binding(
            BETA_NODE,
            "beta",
            "acct_phase22_beta",
            &policy_hash,
        ))
        .expect("beta reward binding");

    reward_bindings
        .insert_binding(binding(
            GAMMA_NODE,
            "gamma",
            "acct_phase22_gamma",
            &policy_hash,
        ))
        .expect("gamma reward binding");

    let alpha_resolution = reward_bindings.resolve(ALPHA_NODE, EPOCH_NUMBER, 2_200);

    alpha_resolution
        .validate()
        .expect("registry-derived alpha resolution");

    preview_reward_payout_authorization(RewardPayoutAuthorizationPreview {
        issuer_service_node_id: BETA_NODE,

        beneficiary_service_node_id: ALPHA_NODE,

        recipient_resolution: &alpha_resolution,

        evidence_payout_override: None,

        self_issuance_mode: SelfIssuanceMode::RejectByDefault,
    })
    .expect("different eligible node may approve registry-bound beneficiary");

    assert_eq!(
        preview_reward_payout_authorization(RewardPayoutAuthorizationPreview {
            issuer_service_node_id: ALPHA_NODE,

            beneficiary_service_node_id: ALPHA_NODE,

            recipient_resolution: &alpha_resolution,

            evidence_payout_override: None,

            self_issuance_mode: SelfIssuanceMode::RejectByDefault,
        },),
        Err(RewardPayoutAuthorizationError::SelfIssuedPayoutRejected,),
    );

    let allocations = transition_allocations(&first_plan);

    let reward_total = allocations
        .iter()
        .map(|allocation| {
            allocation
                .amount_minor_units
                .parse::<u128>()
                .expect("planned allocation amount")
        })
        .sum::<u128>();

    assert_eq!(reward_total, first_plan.totals.allocated_minor_units.get(),);

    let reward_plan_hash = ContentId::parse(&first_plan.plan_id).expect("reward plan content ID");

    let economics_config_hash =
        ContentId::parse(&economics.economics_config_hash).expect("economics content ID");

    let evidence_root = content_id_from_value(
        "phase22.evidence-root.v1",
        &(service_evidence, user_evidence),
    );

    let eligibilities = vec![
        eligibility("alpha", ALPHA_NODE),
        eligibility("beta", BETA_NODE),
        eligibility("gamma", GAMMA_NODE),
    ];

    let quorum_threshold = threshold();

    let reward_cap_minor_units = economics.epoch_pool_cap_minor.get().to_string();

    let reward_total_minor_units = reward_total.to_string();

    let transition_hash = content_id_from_value(
        "phase22.epoch-transition.v1",
        &TransitionIdentity {
            domain: "rustyonions.phase22.epoch-transition.v1",

            chain_id: CHAIN_ID,
            epoch_id: EPOCH_ID,

            accounting_snapshot_hash: &accounting_snapshot_hash,

            reward_plan_hash: &reward_plan_hash,

            policy_hash: &policy_hash,

            economics_config_hash: &economics_config_hash,

            registry_root: &registry_root,

            reward_binding_root: &reward_binding_root,

            evidence_root: &evidence_root,

            reward_cap_minor_units: &reward_cap_minor_units,

            reward_total_minor_units: &reward_total_minor_units,

            allocations: &allocations,

            threshold: &quorum_threshold,

            eligibilities: &eligibilities,
        },
    );

    let kms = ron_kms::memory_keystore();

    let alpha_key = kms
        .create_ed25519("service-node", "phase22-alpha")
        .expect("alpha quorum key");

    let beta_key = kms
        .create_ed25519("service-node", "phase22-beta")
        .expect("beta quorum key");

    let gamma_key = kms
        .create_ed25519("service-node", "phase22-gamma")
        .expect("gamma quorum key");

    let keys = QuorumKeys {
        keys: BTreeMap::from([
            ("key:phase22:alpha".to_owned(), alpha_key.clone()),
            ("key:phase22:beta".to_owned(), beta_key.clone()),
            ("key:phase22:gamma".to_owned(), gamma_key),
        ]),
    };

    let quorum = ServiceNodeQuorumV1 {
        schema: "ron.service_node.quorum.v1".to_owned(),

        version: ROC_EPOCH_TRANSITION_VERSION,

        chain_id: CHAIN_ID.to_owned(),

        epoch_id: EPOCH_ID.to_owned(),

        transition_hash: transition_hash.clone(),

        threshold: quorum_threshold.clone(),

        eligibilities: eligibilities.clone(),

        signatures: vec![
            signed_signature(
                &kms,
                &alpha_key,
                "key:phase22:alpha",
                ALPHA_NODE,
                &transition_hash,
            ),
            signed_signature(
                &kms,
                &beta_key,
                "key:phase22:beta",
                BETA_NODE,
                &transition_hash,
            ),
        ],
    };

    let transition = RocEpochTransitionV1 {
        schema: ROC_EPOCH_TRANSITION_SCHEMA.to_owned(),

        version: ROC_EPOCH_TRANSITION_VERSION,

        chain_id: CHAIN_ID.to_owned(),

        epoch_id: EPOCH_ID.to_owned(),

        transition_hash,

        accounting_snapshot_hash,
        reward_plan_hash,
        policy_hash,
        economics_config_hash,
        registry_root,
        reward_binding_root,
        evidence_root,

        reward_cap_minor_units,
        reward_total_minor_units,

        allocations,
        quorum,

        produced_at_ms: 2_200,
    };

    let expectation = RocEpochTransitionExpectationV1 {
        schema: ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA.to_owned(),

        version: ROC_EPOCH_TRANSITION_VERSION,

        chain_id: transition.chain_id.clone(),

        epoch_id: transition.epoch_id.clone(),

        accounting_snapshot_hash: transition.accounting_snapshot_hash.clone(),

        reward_plan_hash: transition.reward_plan_hash.clone(),

        policy_hash: transition.policy_hash.clone(),

        economics_config_hash: transition.economics_config_hash.clone(),

        registry_root: transition.registry_root.clone(),

        reward_binding_root: transition.reward_binding_root.clone(),

        evidence_root: transition.evidence_root.clone(),

        reward_cap_minor_units: transition.reward_cap_minor_units.clone(),

        threshold: quorum_threshold,

        eligibilities,
    };

    transition
        .validate_against(&expectation)
        .expect("two-of-three quorum transition must validate");

    let resolutions = vec![alpha_resolution];

    let operations = prepare_epoch_payout_operations(&transition, &resolutions)
        .expect("registry-bound payout operation");

    let mut single_node_transition = transition.clone();

    single_node_transition.quorum.signatures.truncate(1);

    let config = svc_wallet::config::WalletConfig::default();

    let single_node_ledger = LocalLedgerClient::in_memory().expect("single-node rejection ledger");

    assert!(execute_epoch_transition(
        &single_node_ledger,
        &config,
        EpochTransitionExecutionRequest::new(
            &single_node_transition,
            &expectation,
            &resolutions,
            &operations,
            &kms,
            &keys,
        ),
    )
    .is_err());

    assert_eq!(
        single_node_ledger
            .balance(&config, ALPHA_ACCOUNT,)
            .expect("single-node rejected balance",)
            .amount_minor
            .get(),
        0,
    );

    let ledger = LocalLedgerClient::in_memory().expect("local Phase 22 ledger");

    let first_receipts = execute_epoch_transition(
        &ledger,
        &config,
        EpochTransitionExecutionRequest::new(
            &transition,
            &expectation,
            &resolutions,
            &operations,
            &kms,
            &keys,
        ),
    )
    .expect("quorum transition must execute through svc-wallet");

    let retry_receipts = execute_epoch_transition(
        &ledger,
        &config,
        EpochTransitionExecutionRequest::new(
            &transition,
            &expectation,
            &resolutions,
            &operations,
            &kms,
            &keys,
        ),
    )
    .expect("exact transition retry must replay");

    assert_eq!(first_receipts, retry_receipts,);

    assert_eq!(first_receipts.len(), 1,);

    assert_eq!(
        first_receipts[0].operation.recipient_account_id,
        ALPHA_ACCOUNT,
    );

    assert_eq!(
        first_receipts[0].operation.amount_minor,
        reward_total.to_string(),
    );

    assert_eq!(
        first_receipts[0].operation.economics_config_hash,
        transition.economics_config_hash,
    );

    assert_eq!(
        ledger
            .balance(&config, ALPHA_ACCOUNT,)
            .expect("confirmed alpha balance",)
            .amount_minor
            .get(),
        reward_total,
    );

    let replay =
        replay_epoch_payout_receipts(&first_receipts).expect("ledger receipts must replay");

    assert_eq!(replay.receipt_count, 1,);

    assert_eq!(replay.total_issued_minor, reward_total,);

    assert_eq!(replay.balances[ALPHA_ACCOUNT], reward_total,);

    assert_eq!(
        replay.balances.values().copied().sum::<u128>(),
        replay.total_issued_minor,
    );

    let observation = EpochReplayObservationV1 {
        schema: USER_NODE_EPOCH_REPLAY_OBSERVATION_SCHEMA.to_owned(),

        version: USER_NODE_EPOCH_REPLAY_VERSION,

        accounting_snapshot_hash: expectation.accounting_snapshot_hash.clone(),

        reward_plan_hash: expectation.reward_plan_hash.clone(),

        policy_hash: expectation.policy_hash.clone(),

        economics_config_hash: expectation.economics_config_hash.clone(),

        registry_root: expectation.registry_root.clone(),

        reward_binding_root: expectation.reward_binding_root.clone(),

        applied_allocation_ids: transition
            .allocations
            .iter()
            .map(|allocation| allocation.allocation_id.clone())
            .collect(),

        replayed_reward_total_minor_units: replay.total_issued_minor.to_string(),

        supply_before_minor_units: "0".to_owned(),

        supply_after_minor_units: replay.total_issued_minor.to_string(),
    };

    let review = review_epoch_transition_with_signatures(
        &transition,
        &expectation,
        &observation,
        &kms,
        &keys,
    );

    assert_eq!(review.status(), UserNodeEpochReviewStatusV1::Accepted,);

    assert!(review.is_accepted());
    assert!(review.findings().is_empty());
    assert!(!review.wallet_mutation());
    assert!(!review.ledger_mutation());
    assert!(!review.challenge_submitted());

    if let Ok(raw_path) = std::env::var("PHASE22_CONFIRMED_ROC_PROJECTION_PATH") {
        let path = confirmed_roc_projection_path(&raw_path);

        let operation_ids = first_receipts
            .iter()
            .map(|receipt| receipt.operation.operation_id.clone())
            .collect::<Vec<_>>();

        let last_ledger_root = confirmed_roc_projection_ledger_root(&replay.last_ledger_root);

        let projection = serde_json::json!({
            "schema":
                "crablink.phase22.confirmed-roc-projection.v1",

            "version": 1,

            "epochId":
                EPOCH_ID,

            "accountId":
                ALPHA_ACCOUNT,

            "confirmedRocMinorUnits":
                replay.total_issued_minor.to_string(),

            "source":
                "wallet_ledger_receipt_only",

            "receiptCount":
                replay.receipt_count,

            "lastLedgerSeq":
                replay.last_ledger_seq,

            "lastLedgerRoot":
                last_ledger_root,

            "transitionHash":
                transition.transition_hash.as_str(),

            "economicsConfigHash":
                transition.economics_config_hash.as_str(),

            "walletReceiptConfirmed":
                true,

            "ledgerReplayConfirmed":
                true,

            "userNodeReplayAccepted":
                review.is_accepted(),

            "pendingEvidenceOnly":
                false,

            "displayOnly":
                true,

            "clientWalletMutation":
                false,

            "clientLedgerMutation":
                false,

            "clientFinalityAuthority":
                false,

            "operationIds":
                operation_ids,
        });

        let bytes = serde_json::to_vec_pretty(&projection)
            .expect("confirmed ROC projection must serialize");

        std::fs::write(path, bytes).unwrap_or_else(|error| {
            panic!("confirmed ROC projection write failed at {path}: {error}")
        });
    }

    let mut tampered_observation = observation;

    tampered_observation.reward_plan_hash =
        content_id_from_bytes("phase22.tampered-reward-plan.v1", b"tampered");

    let rejected = review_epoch_transition_with_signatures(
        &transition,
        &expectation,
        &tampered_observation,
        &kms,
        &keys,
    );

    assert!(!rejected.is_accepted());
    assert!(!rejected.wallet_mutation());
    assert!(!rejected.ledger_mutation());

    let challenge =
        build_invalid_epoch_challenge(&transition, &rejected, "user_node:phase22_auditor", 2_300)
            .expect("tampered replay must produce canonical challenge evidence");

    challenge
        .validate()
        .expect("Phase 22 challenge must validate");

    assert_eq!(
        challenge.challenge_kind,
        InvalidEpochChallengeKindV1::InvalidRewardPlan,
    );

    println!(
        "Phase 22G passed: canonical evidence produced a capped registry-bound two-of-three quorum payout, durable idempotent ledger receipt, accepted User Node replay, and deterministic tamper challenge; self-issuance and single-node minting were rejected."
    );
}
