//! RO:WHAT — Phase 16A cryptographic quorum and atomic epoch payout tests.
//! RO:WHY — Prove that only exact recipient-bound, root-bound, quorum-signed
//! operations enter svc-wallet and append-only ron-ledger truth.
//! RO:INTERACTS — svc-wallet epoch_execution, ron-kms, ron-proto Phase 15,
//! ron-ledger payout evidence and replay.
//! RO:INVARIANTS — no single-node mint, no recipient redirection, no duplicate
//! issuance, economics hash preserved, deterministic replay and conservation.
//! RO:SECURITY — tests use real Ed25519 signing and verification.
//! RO:TEST — cargo test -p svc-wallet --test internal_roc_beta_phase16_quorum_execution.

use std::collections::BTreeMap;

use ron_kms::{backends::memory::MemoryKeystore, KeyId, Keystore, Signer};
use ron_ledger::replay_epoch_payout_receipts;
use ron_proto::{
    ContentId, EpochEligibilityStatusV1, EpochEligibilityV1, EpochQuorumThresholdV1,
    EpochRewardAllocationV1, RewardRecipientResolutionStateV1, RewardRecipientResolutionV1,
    RocEpochTransitionExpectationV1, RocEpochTransitionV1, ServiceNodeQuorumV1,
    ServiceNodeSignatureV1, SignatureAlg, EPOCH_REWARD_ALLOCATION_SCHEMA,
    REWARD_RECIPIENT_RESOLUTION_VERSION, ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA,
    ROC_EPOCH_TRANSITION_SCHEMA, ROC_EPOCH_TRANSITION_VERSION,
};
use svc_wallet::{
    epoch_execution::{
        execute_epoch_transition, prepare_epoch_payout_operations,
        service_node_signature_message_bytes, EpochTransitionExecutionRequest, QuorumKeyResolver,
    },
    errors::{WalletError, WalletResult},
    ledger::client::LocalLedgerClient,
};

const ALPHA_NODE: &str = "service_node:alpha";
const BETA_NODE: &str = "service_node:beta";
const GAMMA_NODE: &str = "service_node:gamma";

const ALPHA_ACCOUNT: &str = "acct_phase16_alpha";
const BETA_ACCOUNT: &str = "acct_phase16_beta";

struct TestKeyResolver {
    keys: BTreeMap<String, KeyId>,
}

impl QuorumKeyResolver for TestKeyResolver {
    fn resolve_key(&self, key_ref: &str) -> WalletResult<KeyId> {
        self.keys.get(key_ref).cloned().ok_or_else(|| {
            WalletError::forbidden(format!("unknown test quorum key reference {key_ref}"))
        })
    }
}

struct Fixture {
    kms: MemoryKeystore,
    resolver: TestKeyResolver,
    transition: RocEpochTransitionV1,
    expectation: RocEpochTransitionExpectationV1,
    resolutions: Vec<RewardRecipientResolutionV1>,
}

fn cid(character: char) -> ContentId {
    ContentId::parse(&format!("b3:{}", character.to_string().repeat(64)))
        .expect("fixture content id must parse")
}

fn eligibility(
    service_node_id: &str,
    registry_entry_id: &str,
    reward_binding_id: &str,
    key_id: &str,
) -> EpochEligibilityV1 {
    EpochEligibilityV1 {
        version: 1,
        service_node_id: service_node_id.to_owned(),
        registry_entry_id: registry_entry_id.to_owned(),
        reward_binding_id: reward_binding_id.to_owned(),
        key_id: key_id.to_owned(),
        status: EpochEligibilityStatusV1::Eligible,
    }
}

fn threshold() -> EpochQuorumThresholdV1 {
    EpochQuorumThresholdV1 {
        version: 1,
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
        version: 1,
        chain_id: "rustyonions-dev".to_owned(),
        epoch_id: "epoch:16".to_owned(),
        service_node_id: service_node_id.to_owned(),
        key_id: logical_key_ref.to_owned(),
        algorithm: SignatureAlg::Ed25519,
        transition_hash: transition_hash.clone(),
        signature_wire: "00".repeat(64),
    };

    let message =
        service_node_signature_message_bytes(&signature).expect("signature message must encode");

    signature.signature_wire = hex::encode(
        kms.sign(concrete_key, &message)
            .expect("test quorum key must sign"),
    );

    signature
}

fn allocation(
    allocation_id: &str,
    reward_plan_allocation_id: &str,
    service_node_id: &str,
    amount_minor_units: &str,
) -> EpochRewardAllocationV1 {
    EpochRewardAllocationV1 {
        schema: EPOCH_REWARD_ALLOCATION_SCHEMA.to_owned(),
        version: ROC_EPOCH_TRANSITION_VERSION,
        allocation_id: allocation_id.to_owned(),
        reward_plan_allocation_id: reward_plan_allocation_id.to_owned(),
        service_node_id: service_node_id.to_owned(),
        source_pool: "node_delivery".to_owned(),
        amount_minor_units: amount_minor_units.to_owned(),
    }
}

fn resolution(
    service_node_id: &str,
    binding_id: &str,
    account_id: &str,
    display_address: &str,
    registry_root: &ContentId,
    reward_binding_root: &ContentId,
) -> RewardRecipientResolutionV1 {
    RewardRecipientResolutionV1 {
        version: REWARD_RECIPIENT_RESOLUTION_VERSION,
        service_node_id: service_node_id.to_owned(),
        resolved_at_epoch: 16,
        registry_root: registry_root.clone(),
        reward_binding_root: reward_binding_root.clone(),
        state: RewardRecipientResolutionStateV1::Resolved,
        binding_id: Some(binding_id.to_owned()),
        reward_recipient_account_id: Some(account_id.to_owned()),
        reward_recipient_display_address: Some(display_address.to_owned()),
    }
}

fn fixture() -> Fixture {
    let kms = ron_kms::memory_keystore();

    let alpha_key = kms
        .create_ed25519("service-node", "alpha")
        .expect("alpha key must be created");
    let beta_key = kms
        .create_ed25519("service-node", "beta")
        .expect("beta key must be created");
    let gamma_key = kms
        .create_ed25519("service-node", "gamma")
        .expect("gamma key must be created");

    let logical_alpha = "key:alpha";
    let logical_beta = "key:beta";
    let logical_gamma = "key:gamma";

    let resolver = TestKeyResolver {
        keys: BTreeMap::from([
            (logical_alpha.to_owned(), alpha_key.clone()),
            (logical_beta.to_owned(), beta_key.clone()),
            (logical_gamma.to_owned(), gamma_key),
        ]),
    };

    let transition_hash = cid('8');

    let eligibilities = vec![
        eligibility(ALPHA_NODE, "registry:alpha", "binding:alpha", logical_alpha),
        eligibility(BETA_NODE, "registry:beta", "binding:beta", logical_beta),
        eligibility(GAMMA_NODE, "registry:gamma", "binding:gamma", logical_gamma),
    ];

    let quorum = ServiceNodeQuorumV1 {
        schema: "ron.service_node.quorum.v1".to_owned(),
        version: 1,
        chain_id: "rustyonions-dev".to_owned(),
        epoch_id: "epoch:16".to_owned(),
        transition_hash: transition_hash.clone(),
        threshold: threshold(),
        eligibilities: eligibilities.clone(),
        signatures: vec![
            signed_signature(
                &kms,
                &alpha_key,
                logical_alpha,
                ALPHA_NODE,
                &transition_hash,
            ),
            signed_signature(&kms, &beta_key, logical_beta, BETA_NODE, &transition_hash),
        ],
    };

    let transition = RocEpochTransitionV1 {
        schema: ROC_EPOCH_TRANSITION_SCHEMA.to_owned(),
        version: ROC_EPOCH_TRANSITION_VERSION,
        chain_id: "rustyonions-dev".to_owned(),
        epoch_id: "epoch:16".to_owned(),
        transition_hash,
        accounting_snapshot_hash: cid('1'),
        reward_plan_hash: cid('2'),
        policy_hash: cid('3'),
        economics_config_hash: cid('4'),
        registry_root: cid('5'),
        reward_binding_root: cid('6'),
        evidence_root: cid('7'),
        reward_cap_minor_units: "1000".to_owned(),
        reward_total_minor_units: "1000".to_owned(),
        allocations: vec![
            allocation(
                "allocation:alpha",
                "reward_plan_allocation:alpha",
                ALPHA_NODE,
                "400",
            ),
            allocation(
                "allocation:beta",
                "reward_plan_allocation:beta",
                BETA_NODE,
                "600",
            ),
        ],
        quorum,
        produced_at_ms: 1_789_000_000_000,
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
        threshold: transition.quorum.threshold.clone(),
        eligibilities,
    };

    let resolutions = vec![
        resolution(
            ALPHA_NODE,
            "binding:alpha",
            ALPHA_ACCOUNT,
            "@alpha",
            &transition.registry_root,
            &transition.reward_binding_root,
        ),
        resolution(
            BETA_NODE,
            "binding:beta",
            BETA_ACCOUNT,
            "@beta",
            &transition.registry_root,
            &transition.reward_binding_root,
        ),
    ];

    Fixture {
        kms,
        resolver,
        transition,
        expectation,
        resolutions,
    }
}

#[test]
fn valid_quorum_transition_executes_atomically_and_replays() {
    let fixture = fixture();
    let config = svc_wallet::config::WalletConfig::default();
    let client = LocalLedgerClient::in_memory().expect("wallet ledger must initialize");

    let operations = prepare_epoch_payout_operations(&fixture.transition, &fixture.resolutions)
        .expect("valid operations must prepare");

    let receipts = execute_epoch_transition(
        &client,
        &config,
        EpochTransitionExecutionRequest::new(
            &fixture.transition,
            &fixture.expectation,
            &fixture.resolutions,
            &operations,
            &fixture.kms,
            &fixture.resolver,
        ),
    )
    .expect("valid quorum transition must execute");

    assert_eq!(receipts.len(), 2);
    assert_eq!(
        client
            .balance(&config, ALPHA_ACCOUNT)
            .expect("alpha balance")
            .amount_minor
            .get(),
        400
    );
    assert_eq!(
        client
            .balance(&config, BETA_ACCOUNT)
            .expect("beta balance")
            .amount_minor
            .get(),
        600
    );

    for receipt in &receipts {
        assert_eq!(
            receipt.operation.economics_config_hash,
            fixture.transition.economics_config_hash
        );

        let stored = client
            .epoch_payout_record(&receipt.operation.operation_id)
            .expect("ledger record lookup must succeed")
            .expect("accepted payout evidence must be stored");

        assert_eq!(stored, receipt.operation);
    }

    let replay = replay_epoch_payout_receipts(&receipts).expect("accepted receipts must replay");

    assert_eq!(replay.receipt_count, 2);
    assert_eq!(replay.total_issued_minor, 1000);
    assert_eq!(replay.balances[ALPHA_ACCOUNT], 400);
    assert_eq!(replay.balances[BETA_ACCOUNT], 600);
    assert_eq!(
        replay.balances.values().copied().sum::<u128>(),
        replay.total_issued_minor
    );
}

#[test]
fn exact_transition_retry_returns_identical_receipts_without_double_issue() {
    let fixture = fixture();
    let config = svc_wallet::config::WalletConfig::default();
    let client = LocalLedgerClient::in_memory().expect("wallet ledger must initialize");

    let operations = prepare_epoch_payout_operations(&fixture.transition, &fixture.resolutions)
        .expect("valid operations must prepare");

    let first = execute_epoch_transition(
        &client,
        &config,
        EpochTransitionExecutionRequest::new(
            &fixture.transition,
            &fixture.expectation,
            &fixture.resolutions,
            &operations,
            &fixture.kms,
            &fixture.resolver,
        ),
    )
    .expect("first execution must succeed");

    let second = execute_epoch_transition(
        &client,
        &config,
        EpochTransitionExecutionRequest::new(
            &fixture.transition,
            &fixture.expectation,
            &fixture.resolutions,
            &operations,
            &fixture.kms,
            &fixture.resolver,
        ),
    )
    .expect("exact retry must replay");

    assert_eq!(first, second);
    assert_eq!(
        client
            .balance(&config, ALPHA_ACCOUNT)
            .expect("alpha balance")
            .amount_minor
            .get(),
        400
    );
    assert_eq!(
        client
            .balance(&config, BETA_ACCOUNT)
            .expect("beta balance")
            .amount_minor
            .get(),
        600
    );
}

#[test]
fn invalid_quorum_signature_rejects_before_ledger_mutation() {
    let mut fixture = fixture();
    let config = svc_wallet::config::WalletConfig::default();
    let client = LocalLedgerClient::in_memory().expect("wallet ledger must initialize");

    let signature = &mut fixture.transition.quorum.signatures[0].signature_wire;
    let replacement = if &signature[..2] == "00" { "01" } else { "00" };
    signature.replace_range(..2, replacement);

    let operations = prepare_epoch_payout_operations(&fixture.transition, &fixture.resolutions)
        .expect("structurally valid operations still prepare");

    assert!(execute_epoch_transition(
        &client,
        &config,
        EpochTransitionExecutionRequest::new(
            &fixture.transition,
            &fixture.expectation,
            &fixture.resolutions,
            &operations,
            &fixture.kms,
            &fixture.resolver,
        ),
    )
    .is_err());

    assert_eq!(
        client
            .balance(&config, ALPHA_ACCOUNT)
            .expect("alpha balance")
            .amount_minor
            .get(),
        0
    );
    assert_eq!(
        client
            .balance(&config, BETA_ACCOUNT)
            .expect("beta balance")
            .amount_minor
            .get(),
        0
    );
}

#[test]
fn caller_prepared_recipient_mismatch_is_rejected() {
    let fixture = fixture();
    let config = svc_wallet::config::WalletConfig::default();
    let client = LocalLedgerClient::in_memory().expect("wallet ledger must initialize");

    let mut operations = prepare_epoch_payout_operations(&fixture.transition, &fixture.resolutions)
        .expect("valid operations must prepare");

    operations[0].recipient_account_id = "acct_attacker".to_owned();

    assert!(execute_epoch_transition(
        &client,
        &config,
        EpochTransitionExecutionRequest::new(
            &fixture.transition,
            &fixture.expectation,
            &fixture.resolutions,
            &operations,
            &fixture.kms,
            &fixture.resolver,
        ),
    )
    .is_err());

    assert_eq!(
        client
            .balance(&config, "acct_attacker")
            .expect("attacker balance")
            .amount_minor
            .get(),
        0
    );
}

#[test]
fn economics_config_hash_mismatch_rejects_before_execution() {
    let mut fixture = fixture();
    let config = svc_wallet::config::WalletConfig::default();
    let client = LocalLedgerClient::in_memory().expect("wallet ledger must initialize");

    let operations = prepare_epoch_payout_operations(&fixture.transition, &fixture.resolutions)
        .expect("valid operations must prepare");

    fixture.expectation.economics_config_hash = cid('9');

    assert!(execute_epoch_transition(
        &client,
        &config,
        EpochTransitionExecutionRequest::new(
            &fixture.transition,
            &fixture.expectation,
            &fixture.resolutions,
            &operations,
            &fixture.kms,
            &fixture.resolver,
        ),
    )
    .is_err());

    assert_eq!(
        client
            .balance(&config, ALPHA_ACCOUNT)
            .expect("alpha balance")
            .amount_minor
            .get(),
        0
    );
}

#[test]
fn single_service_node_quorum_cannot_prepare_payouts() {
    let mut fixture = fixture();

    fixture.transition.quorum.eligibilities.truncate(1);
    fixture.transition.quorum.signatures.truncate(1);
    fixture.transition.quorum.threshold.eligible_service_nodes = 1;
    fixture.transition.quorum.threshold.minimum_signatures = 1;
    fixture.transition.quorum.threshold.required_signatures = 1;
    fixture.transition.quorum.threshold.quorum_bps = 10_000;

    assert!(
        prepare_epoch_payout_operations(&fixture.transition, &fixture.resolutions,).is_err(),
        "single-node quorum must fail before wallet mutation"
    );
}

#[test]
fn operation_wire_requires_economics_config_hash() {
    let fixture = fixture();

    let operations = prepare_epoch_payout_operations(&fixture.transition, &fixture.resolutions)
        .expect("valid operations must prepare");

    let mut encoded = serde_json::to_value(&operations[0]).expect("operation must serialize");

    encoded
        .as_object_mut()
        .expect("operation must be an object")
        .remove("economics_config_hash");

    assert!(
        serde_json::from_value::<ron_ledger::EpochPayoutOperationV1>(encoded).is_err(),
        "economics_config_hash must be required in ledger operation evidence"
    );
}

#[test]
fn policy_hash_mismatch_rejects_before_execution() {
    let mut fixture = fixture();
    let config = svc_wallet::config::WalletConfig::default();
    let client = LocalLedgerClient::in_memory().expect("wallet ledger must initialize");

    let operations = prepare_epoch_payout_operations(&fixture.transition, &fixture.resolutions)
        .expect("valid operations must prepare");

    fixture.expectation.policy_hash = cid('9');

    let result = execute_epoch_transition(
        &client,
        &config,
        EpochTransitionExecutionRequest::new(
            &fixture.transition,
            &fixture.expectation,
            &fixture.resolutions,
            &operations,
            &fixture.kms,
            &fixture.resolver,
        ),
    );

    assert!(
        result.is_err(),
        "policy hash mismatch must reject before wallet mutation"
    );

    assert_eq!(
        client
            .balance(&config, ALPHA_ACCOUNT)
            .expect("alpha balance")
            .amount_minor
            .get(),
        0
    );

    assert_eq!(
        client
            .balance(&config, BETA_ACCOUNT)
            .expect("beta balance")
            .amount_minor
            .get(),
        0
    );
}

#[test]
fn registry_root_mismatch_rejects_recipient_resolution() {
    let mut fixture = fixture();

    fixture.resolutions[0].registry_root = cid('9');

    assert!(
        prepare_epoch_payout_operations(&fixture.transition, &fixture.resolutions,).is_err(),
        "recipient resolution with the wrong registry root must reject"
    );
}

#[test]
fn reward_binding_root_mismatch_rejects_recipient_resolution() {
    let mut fixture = fixture();

    fixture.resolutions[0].reward_binding_root = cid('9');

    assert!(
        prepare_epoch_payout_operations(&fixture.transition, &fixture.resolutions,).is_err(),
        "recipient resolution with the wrong reward-binding root must reject"
    );
}

#[test]
fn recipient_binding_id_mismatch_rejects_before_execution() {
    let mut fixture = fixture();

    fixture.resolutions[0].binding_id = Some("binding:attacker".to_owned());

    assert!(
        prepare_epoch_payout_operations(&fixture.transition, &fixture.resolutions,).is_err(),
        "recipient resolution must match the eligible node binding id"
    );
}
