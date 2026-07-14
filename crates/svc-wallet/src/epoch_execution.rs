//! RO:WHAT — Phase 16 quorum-approved ROC epoch payout preparation and execution.
//! RO:WHY — ECON/SEC: only structurally valid and cryptographically verified
//! epoch transitions may enter the existing svc-wallet/ron-ledger mutation path.
//! RO:INTERACTS — ron-proto Phase 15 DTOs, ron-kms, LocalLedgerClient,
//! ron-ledger epoch payout evidence, registry recipient resolutions.
//! RO:INVARIANTS — exact roots, exact prepared operations, valid signatures,
//! canonical recipients, atomic ledger batch, no unilateral local mint.
//! RO:METRICS — route/service wrappers may count reject categories later.
//! RO:CONFIG — consumes accepted economics_config_hash; owns no reward rates.
//! RO:SECURITY — all signatures are verified before ledger mutation.
//! RO:TEST — internal_roc_beta_phase16_quorum_execution.rs.

use std::collections::{BTreeMap, BTreeSet};

use ron_kms::{Alg, KeyId, Verifier};
use ron_ledger::{
    EpochPayoutOperationV1, EpochPayoutReceiptV1, EPOCH_PAYOUT_OPERATION_SCHEMA,
    EPOCH_PAYOUT_VERSION,
};
use ron_proto::{
    service_node_signature_message_bytes as canonical_signature_message_bytes,
    EpochRewardAllocationV1, RewardRecipientResolutionStateV1, RewardRecipientResolutionV1,
    RocEpochTransitionExpectationV1, RocEpochTransitionV1, ServiceNodeSignatureV1, SignatureAlg,
};
use serde::Serialize;

use crate::{
    config::WalletConfig,
    errors::{WalletError, WalletResult},
    ledger::client::LocalLedgerClient,
};
use ron_ledger::Storage;

/// Resolves a Phase 15 logical quorum key reference into a concrete KMS key ID.
pub trait QuorumKeyResolver {
    /// Resolve one logical key reference.
    fn resolve_key(&self, key_ref: &str) -> WalletResult<KeyId>;
}

/// Return the exact canonical message bytes verified for one quorum signature.
///
/// `signature_wire` is intentionally excluded from its own signing preimage.
pub fn service_node_signature_message_bytes(
    signature: &ServiceNodeSignatureV1,
) -> WalletResult<Vec<u8>> {
    if signature.algorithm != SignatureAlg::Ed25519 {
        return Err(WalletError::forbidden(
            "Phase 16 currently accepts Ed25519 quorum signatures only",
        ));
    }

    canonical_signature_message_bytes(signature).map_err(WalletError::from)
}

/// Prepare the exact recipient-bound operations expected from a transition.
///
/// Preparation does not mutate the ledger and does not verify cryptographic
/// signatures. `execute_epoch_transition` performs those gates before commit.
pub fn prepare_epoch_payout_operations(
    transition: &RocEpochTransitionV1,
    resolutions: &[RewardRecipientResolutionV1],
) -> WalletResult<Vec<EpochPayoutOperationV1>> {
    transition.validate().map_err(|error| {
        WalletError::forbidden(format!("invalid Phase 15 epoch transition: {error}"))
    })?;

    let resolution_map = validate_resolutions(transition, resolutions)?;
    let mut operations = Vec::with_capacity(transition.allocations.len());
    let mut operation_ids = BTreeSet::new();
    let mut idempotency_keys = BTreeSet::new();

    for allocation in &transition.allocations {
        let resolution = resolution_map
            .get(allocation.service_node_id.as_str())
            .ok_or_else(|| {
                WalletError::forbidden(format!(
                    "missing reward recipient resolution for {}",
                    allocation.service_node_id
                ))
            })?;

        let recipient_account_id = resolution
            .reward_recipient_account_id
            .as_deref()
            .ok_or_else(|| {
                WalletError::forbidden("resolved reward recipient is missing canonical account id")
            })?;

        let operation = build_operation(transition, allocation, recipient_account_id)?;

        if !operation_ids.insert(operation.operation_id.clone()) {
            return Err(WalletError::idempotency_conflict(
                "duplicate Phase 16 operation_id",
            ));
        }

        if !idempotency_keys.insert(operation.idempotency_key.clone()) {
            return Err(WalletError::idempotency_conflict(
                "duplicate Phase 16 idempotency_key",
            ));
        }

        operations.push(operation);
    }

    Ok(operations)
}

/// Reviewed inputs required to execute one Phase 16 epoch transition.
///
/// This request groups immutable transition, expectation, registry-resolution,
/// prepared-operation, signature-verification, and key-resolution inputs. It
/// owns no mutation authority and performs no work until passed to
/// [`execute_epoch_transition`].
pub struct EpochTransitionExecutionRequest<'a, V, R> {
    transition: &'a RocEpochTransitionV1,
    expected: &'a RocEpochTransitionExpectationV1,
    resolutions: &'a [RewardRecipientResolutionV1],
    prepared_operations: &'a [EpochPayoutOperationV1],
    verifier: &'a V,
    key_resolver: &'a R,
}

impl<'a, V, R> EpochTransitionExecutionRequest<'a, V, R> {
    /// Group reviewed Phase 16 execution inputs without performing mutation.
    pub const fn new(
        transition: &'a RocEpochTransitionV1,
        expected: &'a RocEpochTransitionExpectationV1,
        resolutions: &'a [RewardRecipientResolutionV1],
        prepared_operations: &'a [EpochPayoutOperationV1],
        verifier: &'a V,
        key_resolver: &'a R,
    ) -> Self {
        Self {
            transition,
            expected,
            resolutions,
            prepared_operations,
            verifier,
            key_resolver,
        }
    }
}

/// Verify and atomically execute one accepted Phase 15 transition.
///
/// Prepared operations are compared byte-for-byte with wallet-derived expected
/// operations so a caller cannot redirect recipients, alter amounts, replace
/// roots, or substitute a different quorum set.
pub fn execute_epoch_transition<S, V, R>(
    client: &LocalLedgerClient<S>,
    config: &WalletConfig,
    request: EpochTransitionExecutionRequest<'_, V, R>,
) -> WalletResult<Vec<EpochPayoutReceiptV1>>
where
    S: Storage,
    V: Verifier,
    R: QuorumKeyResolver,
{
    request
        .transition
        .validate_against(request.expected)
        .map_err(|error| {
            WalletError::forbidden(format!(
                "epoch transition does not match reviewed inputs: {error}"
            ))
        })?;

    verify_quorum_signatures(request.transition, request.verifier, request.key_resolver)?;

    let expected_operations =
        prepare_epoch_payout_operations(request.transition, request.resolutions)?;

    if expected_operations != request.prepared_operations {
        return Err(WalletError::forbidden(
            "prepared Phase 16 payout operations do not match transition and registry truth",
        ));
    }

    client.execute_epoch_payout_batch(config, request.prepared_operations)
}

fn verify_quorum_signatures<V, R>(
    transition: &RocEpochTransitionV1,
    verifier: &V,
    key_resolver: &R,
) -> WalletResult<()>
where
    V: Verifier,
    R: QuorumKeyResolver,
{
    for signature in &transition.quorum.signatures {
        if signature.algorithm != SignatureAlg::Ed25519 {
            return Err(WalletError::forbidden(
                "unsupported quorum signature algorithm",
            ));
        }

        let key_id = key_resolver.resolve_key(&signature.key_id)?;

        if key_id.alg != Alg::Ed25519 {
            return Err(WalletError::forbidden("resolved quorum key is not Ed25519"));
        }

        if signature.signature_wire.len() != 128
            || !signature
                .signature_wire
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(WalletError::forbidden(
                "quorum signature must be 64-byte lowercase hex",
            ));
        }

        let signature_bytes = hex::decode(&signature.signature_wire)
            .map_err(|_| WalletError::forbidden("quorum signature hex could not be decoded"))?;

        let message = service_node_signature_message_bytes(signature)?;

        let verified = verifier
            .verify(&key_id, &message, &signature_bytes)
            .map_err(|_| {
                WalletError::upstream("quorum signature verification dependency failed")
            })?;

        if !verified {
            return Err(WalletError::forbidden(
                "quorum signature verification failed",
            ));
        }
    }

    Ok(())
}

fn validate_resolutions<'a>(
    transition: &RocEpochTransitionV1,
    resolutions: &'a [RewardRecipientResolutionV1],
) -> WalletResult<BTreeMap<&'a str, &'a RewardRecipientResolutionV1>> {
    let epoch_number = transition
        .epoch_id
        .strip_prefix("epoch:")
        .ok_or_else(|| {
            WalletError::bad_request("Phase 16 epoch_id must use canonical epoch:<number> form")
        })?
        .parse::<u64>()
        .map_err(|_| {
            WalletError::bad_request("Phase 16 epoch_id suffix must be an unsigned integer")
        })?;

    let mut map = BTreeMap::new();

    for resolution in resolutions {
        resolution.validate().map_err(|error| {
            WalletError::forbidden(format!("invalid reward recipient resolution: {error}"))
        })?;

        if resolution.state != RewardRecipientResolutionStateV1::Resolved {
            return Err(WalletError::forbidden(
                "reward recipient resolution is not resolved",
            ));
        }

        if resolution.resolved_at_epoch != epoch_number {
            return Err(WalletError::forbidden(
                "reward recipient resolution epoch mismatch",
            ));
        }

        if resolution.registry_root != transition.registry_root {
            return Err(WalletError::forbidden(
                "reward recipient registry root mismatch",
            ));
        }

        if resolution.reward_binding_root != transition.reward_binding_root {
            return Err(WalletError::forbidden(
                "reward recipient binding root mismatch",
            ));
        }

        let eligibility = transition
            .quorum
            .eligibilities
            .iter()
            .find(|eligibility| eligibility.service_node_id == resolution.service_node_id)
            .ok_or_else(|| {
                WalletError::forbidden(
                    "reward recipient resolution targets an ineligible service node",
                )
            })?;

        let binding_id = resolution.binding_id.as_deref().ok_or_else(|| {
            WalletError::forbidden("resolved reward recipient is missing binding_id")
        })?;

        if binding_id != eligibility.reward_binding_id {
            return Err(WalletError::forbidden(
                "reward recipient binding id mismatch",
            ));
        }

        if map
            .insert(resolution.service_node_id.as_str(), resolution)
            .is_some()
        {
            return Err(WalletError::forbidden(
                "duplicate reward recipient resolution",
            ));
        }
    }

    Ok(map)
}

fn build_operation(
    transition: &RocEpochTransitionV1,
    allocation: &EpochRewardAllocationV1,
    recipient_account_id: &str,
) -> WalletResult<EpochPayoutOperationV1> {
    #[derive(Serialize)]
    struct OperationIdentity<'a> {
        domain: &'static str,
        epoch_id: &'a str,
        transition_hash: &'a ron_proto::ContentId,
        allocation_id: &'a str,
        reward_plan_allocation_id: &'a str,
        service_node_id: &'a str,
        recipient_account_id: &'a str,
        amount_minor: &'a str,
    }

    let encoded = serde_json::to_vec(&OperationIdentity {
        domain: "rustyonions.epoch-payout-operation.v1",
        epoch_id: &transition.epoch_id,
        transition_hash: &transition.transition_hash,
        allocation_id: &allocation.allocation_id,
        reward_plan_allocation_id: &allocation.reward_plan_allocation_id,
        service_node_id: &allocation.service_node_id,
        recipient_account_id,
        amount_minor: &allocation.amount_minor_units,
    })
    .map_err(WalletError::from)?;

    let digest = blake3::hash(&encoded).to_hex().to_string();

    let operation = EpochPayoutOperationV1 {
        schema: EPOCH_PAYOUT_OPERATION_SCHEMA.to_owned(),
        version: EPOCH_PAYOUT_VERSION,
        operation_id: format!("epoch_payout:{}", &digest[..32]),
        idempotency_key: format!("epoch-payout-{digest}"),
        epoch_id: transition.epoch_id.clone(),
        source_pool: allocation.source_pool.clone(),
        service_node_id: Some(allocation.service_node_id.clone()),
        recipient_account_id: recipient_account_id.to_owned(),
        amount_minor: allocation.amount_minor_units.clone(),
        transition_hash: transition.transition_hash.clone(),
        policy_approval_hash: transition.policy_hash.clone(),
        economics_config_hash: transition.economics_config_hash.clone(),
        reward_plan_hash: transition.reward_plan_hash.clone(),
        accounting_snapshot_hash: transition.accounting_snapshot_hash.clone(),
        registry_hash: transition.registry_root.clone(),
        reward_binding_hash: transition.reward_binding_root.clone(),
        evidence_root: transition.evidence_root.clone(),
        quorum_signature_set: transition.quorum.clone(),
        submitted_at_ms: transition.produced_at_ms,
    };

    operation.validate().map_err(|error| {
        WalletError::bad_request(format!(
            "prepared epoch payout operation is invalid: {error}"
        ))
    })?;

    Ok(operation)
}
