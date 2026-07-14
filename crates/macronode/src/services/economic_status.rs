//! RO:WHAT — Process-local projection of canonical QuickChain economic stages.
//! RO:WHY — The operator plane needs truthful accounting, reward-plan,
//!          transition, wallet-execution, and ledger-receipt visibility without
//!          becoming economic authority.
//! RO:INTERACTS — ron-proto accounting/reward/transition DTOs and ron-ledger
//!                canonical epoch payout receipts.
//! RO:INVARIANTS —
//!   - Every supplied canonical DTO validates before publication.
//!   - Snapshot, plan, transition, and payout receipt references match exactly.
//!   - Receipt replay rejects duplicate operations, duplicate idempotency keys,
//!     invalid sequence order, arithmetic overflow, and conservation failure.
//!   - Receipt totals and counts match the accepted epoch transition.
//!   - Rejected updates never replace previously accepted projection truth.
//!   - No wallet call, ledger mutation, payout submission, balance mutation,
//!     finality, anchoring, mint, burn, or ROX settlement occurs here.
//!
//! RO:TEST — Focused tests preserve exact cross-stage references and canonical
//!           receipt replay while rejecting mismatched payout material.

#![forbid(unsafe_code)]

use std::{error::Error, fmt, sync::Mutex};

use ron_ledger::{replay_epoch_payout_receipts, EpochPayoutReceiptV1, EpochPayoutReplaySummaryV1};
use ron_proto::{
    QuickChainAccountingSnapshotReferenceV1, QuickChainRewardPlanReferenceV1, RocEpochTransitionV1,
};

/// One validated operator-visible economic pipeline snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomicPipelineSnapshot {
    /// Canonical sealed accounting-snapshot reference.
    pub accounting_snapshot: QuickChainAccountingSnapshotReferenceV1,

    /// Canonical non-mutating reward-plan reference, when produced.
    pub reward_plan: Option<QuickChainRewardPlanReferenceV1>,

    /// Canonical quorum-reviewed epoch-transition material.
    pub epoch_transition: Option<RocEpochTransitionV1>,

    /// Canonical accepted epoch-payout receipts produced through svc-wallet
    /// and durably recorded by ron-ledger.
    pub epoch_payout_receipts: Option<Vec<EpochPayoutReceiptV1>>,

    /// Deterministic replay summary derived from the accepted receipts.
    ///
    /// Recipient balances remain process-local and are never projected through
    /// the operator API.
    pub epoch_payout_replay: Option<EpochPayoutReplaySummaryV1>,
}

/// Rejection while replacing process-local economic projection truth.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EconomicPipelineStatusError {
    /// The accounting-snapshot reference failed canonical validation.
    InvalidAccountingSnapshot(String),

    /// The reward-plan reference failed canonical validation.
    InvalidRewardPlan(String),

    /// The epoch transition failed canonical validation.
    InvalidEpochTransition(String),

    /// Canonical epoch-payout receipt replay failed.
    InvalidEpochPayoutReplay(String),

    /// An epoch transition was supplied without its reward plan.
    MissingRewardPlan,

    /// Payout receipts were supplied without an accepted transition.
    MissingEpochTransition,

    /// An explicitly supplied receipt collection was empty.
    EmptyEpochPayoutReceipts,

    /// Two canonical stages disagreed about a required reference.
    ReferenceMismatch {
        /// Stable field label describing the mismatched reference.
        field: &'static str,
    },

    /// A downstream stage claims to predate its required input.
    TimestampOrder {
        /// Stable timestamp relationship label.
        field: &'static str,
    },

    /// Receipt count did not match transition allocation count.
    ReceiptCountMismatch {
        /// Allocation count declared by the transition.
        expected: usize,
        /// Accepted receipt count.
        actual: usize,
    },

    /// Replayed issued supply did not match the transition total.
    IssuedTotalMismatch {
        /// Transition-declared total.
        expected: String,
        /// Replayed accepted-receipt total.
        actual: String,
    },
}

impl fmt::Display for EconomicPipelineStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAccountingSnapshot(reason) => {
                write!(formatter, "invalid accounting snapshot reference: {reason}")
            }
            Self::InvalidRewardPlan(reason) => {
                write!(formatter, "invalid reward plan reference: {reason}")
            }
            Self::InvalidEpochTransition(reason) => {
                write!(formatter, "invalid epoch transition: {reason}")
            }
            Self::InvalidEpochPayoutReplay(reason) => {
                write!(formatter, "invalid epoch payout receipt replay: {reason}")
            }
            Self::MissingRewardPlan => write!(
                formatter,
                "epoch transition requires a canonical reward plan reference"
            ),
            Self::MissingEpochTransition => write!(
                formatter,
                "epoch payout receipts require a canonical epoch transition"
            ),
            Self::EmptyEpochPayoutReceipts => write!(
                formatter,
                "reported epoch payout receipts must not be empty"
            ),
            Self::ReferenceMismatch { field } => {
                write!(formatter, "economic pipeline reference mismatch: {field}")
            }
            Self::TimestampOrder { field } => write!(
                formatter,
                "economic pipeline timestamp order failed: {field}"
            ),
            Self::ReceiptCountMismatch { expected, actual } => write!(
                formatter,
                "epoch payout receipt count mismatch: expected {expected}, got {actual}"
            ),
            Self::IssuedTotalMismatch { expected, actual } => write!(
                formatter,
                "epoch payout issued-total mismatch: expected {expected}, got {actual}"
            ),
        }
    }
}

impl Error for EconomicPipelineStatusError {}

/// Validated process-local economic status cache.
///
/// The mutex protects one small cloneable snapshot. It never guards wallet
/// execution, signature verification, ledger mutation, or finality.
#[derive(Debug, Default)]
pub struct EconomicPipelineStatusStore {
    current: Mutex<Option<EconomicPipelineSnapshot>>,
}

impl EconomicPipelineStatusStore {
    /// Replace operator-visible economic status after canonical validation.
    #[allow(dead_code)]
    pub fn replace(
        &self,
        accounting_snapshot: QuickChainAccountingSnapshotReferenceV1,
        reward_plan: Option<QuickChainRewardPlanReferenceV1>,
        epoch_transition: Option<RocEpochTransitionV1>,
        epoch_payout_receipts: Option<Vec<EpochPayoutReceiptV1>>,
    ) -> Result<(), EconomicPipelineStatusError> {
        accounting_snapshot.validate().map_err(|error| {
            EconomicPipelineStatusError::InvalidAccountingSnapshot(error.to_string())
        })?;

        if let Some(plan) = reward_plan.as_ref() {
            plan.validate().map_err(|error| {
                EconomicPipelineStatusError::InvalidRewardPlan(error.to_string())
            })?;

            if plan.chain_id != accounting_snapshot.chain_id {
                return Err(EconomicPipelineStatusError::ReferenceMismatch {
                    field: "reward_plan.chain_id/accounting_snapshot.chain_id",
                });
            }

            if plan.snapshot_id != accounting_snapshot.snapshot_id {
                return Err(EconomicPipelineStatusError::ReferenceMismatch {
                    field: "reward_plan.snapshot_id/accounting_snapshot.snapshot_id",
                });
            }

            if plan.snapshot_root != accounting_snapshot.snapshot_root {
                return Err(EconomicPipelineStatusError::ReferenceMismatch {
                    field: "reward_plan.snapshot_root/accounting_snapshot.snapshot_root",
                });
            }

            if plan.produced_at_ms < accounting_snapshot.sealed_at_ms {
                return Err(EconomicPipelineStatusError::TimestampOrder {
                    field: "reward_plan.produced_at_ms >= accounting_snapshot.sealed_at_ms",
                });
            }
        }

        if let Some(transition) = epoch_transition.as_ref() {
            let plan = reward_plan
                .as_ref()
                .ok_or(EconomicPipelineStatusError::MissingRewardPlan)?;

            transition.validate().map_err(|error| {
                EconomicPipelineStatusError::InvalidEpochTransition(error.to_string())
            })?;

            if transition.chain_id != accounting_snapshot.chain_id
                || transition.chain_id != plan.chain_id
            {
                return Err(EconomicPipelineStatusError::ReferenceMismatch {
                    field: "epoch_transition.chain_id/upstream.chain_id",
                });
            }

            if transition.accounting_snapshot_hash != accounting_snapshot.snapshot_root {
                return Err(EconomicPipelineStatusError::ReferenceMismatch {
                    field: "epoch_transition.accounting_snapshot_hash",
                });
            }

            if transition.reward_plan_hash != plan.plan_root {
                return Err(EconomicPipelineStatusError::ReferenceMismatch {
                    field: "epoch_transition.reward_plan_hash",
                });
            }

            if transition.produced_at_ms < plan.produced_at_ms {
                return Err(EconomicPipelineStatusError::TimestampOrder {
                    field: "epoch_transition.produced_at_ms >= reward_plan.produced_at_ms",
                });
            }
        }

        let epoch_payout_replay = if let Some(receipts) = epoch_payout_receipts.as_ref() {
            if receipts.is_empty() {
                return Err(EconomicPipelineStatusError::EmptyEpochPayoutReceipts);
            }

            let transition = epoch_transition
                .as_ref()
                .ok_or(EconomicPipelineStatusError::MissingEpochTransition)?;

            let replay = replay_epoch_payout_receipts(receipts).map_err(|error| {
                EconomicPipelineStatusError::InvalidEpochPayoutReplay(error.to_string())
            })?;

            if replay.receipt_count != transition.allocations.len() {
                return Err(EconomicPipelineStatusError::ReceiptCountMismatch {
                    expected: transition.allocations.len(),
                    actual: replay.receipt_count,
                });
            }

            let replayed_total = replay.total_issued_minor.to_string();

            if replayed_total != transition.reward_total_minor_units {
                return Err(EconomicPipelineStatusError::IssuedTotalMismatch {
                    expected: transition.reward_total_minor_units.clone(),
                    actual: replayed_total,
                });
            }

            for receipt in receipts {
                let operation = &receipt.operation;

                if operation.epoch_id != transition.epoch_id {
                    return Err(EconomicPipelineStatusError::ReferenceMismatch {
                        field: "epoch_payout.operation.epoch_id",
                    });
                }

                if operation.transition_hash != transition.transition_hash {
                    return Err(EconomicPipelineStatusError::ReferenceMismatch {
                        field: "epoch_payout.operation.transition_hash",
                    });
                }

                if operation.policy_approval_hash != transition.policy_hash {
                    return Err(EconomicPipelineStatusError::ReferenceMismatch {
                        field: "epoch_payout.operation.policy_approval_hash",
                    });
                }

                if operation.economics_config_hash != transition.economics_config_hash {
                    return Err(EconomicPipelineStatusError::ReferenceMismatch {
                        field: "epoch_payout.operation.economics_config_hash",
                    });
                }

                if operation.reward_plan_hash != transition.reward_plan_hash {
                    return Err(EconomicPipelineStatusError::ReferenceMismatch {
                        field: "epoch_payout.operation.reward_plan_hash",
                    });
                }

                if operation.accounting_snapshot_hash != transition.accounting_snapshot_hash {
                    return Err(EconomicPipelineStatusError::ReferenceMismatch {
                        field: "epoch_payout.operation.accounting_snapshot_hash",
                    });
                }

                if operation.registry_hash != transition.registry_root {
                    return Err(EconomicPipelineStatusError::ReferenceMismatch {
                        field: "epoch_payout.operation.registry_hash",
                    });
                }

                if operation.reward_binding_hash != transition.reward_binding_root {
                    return Err(EconomicPipelineStatusError::ReferenceMismatch {
                        field: "epoch_payout.operation.reward_binding_hash",
                    });
                }

                if operation.evidence_root != transition.evidence_root {
                    return Err(EconomicPipelineStatusError::ReferenceMismatch {
                        field: "epoch_payout.operation.evidence_root",
                    });
                }

                if operation.quorum_signature_set != transition.quorum {
                    return Err(EconomicPipelineStatusError::ReferenceMismatch {
                        field: "epoch_payout.operation.quorum_signature_set",
                    });
                }

                if operation.submitted_at_ms != transition.produced_at_ms
                    || receipt.accepted_at_ms != transition.produced_at_ms
                {
                    return Err(EconomicPipelineStatusError::TimestampOrder {
                        field:
                            "epoch_payout accepted timestamp equals transition produced timestamp",
                    });
                }

                if receipt.ledger_root != replay.last_ledger_root {
                    return Err(EconomicPipelineStatusError::ReferenceMismatch {
                        field: "epoch_payout.receipt.ledger_root",
                    });
                }
            }

            Some(replay)
        } else {
            None
        };

        *self
            .current
            .lock()
            .expect("economic pipeline status mutex poisoned") = Some(EconomicPipelineSnapshot {
            accounting_snapshot,
            reward_plan,
            epoch_transition,
            epoch_payout_receipts,
            epoch_payout_replay,
        });

        Ok(())
    }

    /// Clear only process-local projection truth.
    #[allow(dead_code)]
    pub fn clear(&self) {
        *self
            .current
            .lock()
            .expect("economic pipeline status mutex poisoned") = None;
    }

    /// Clone the validated status snapshot for read-only projection.
    #[must_use]
    pub fn snapshot(&self) -> Option<EconomicPipelineSnapshot> {
        self.current
            .lock()
            .expect("economic pipeline status mutex poisoned")
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use ron_ledger::{
        EpochPayoutOperationV1, EpochPayoutReceiptV1, EPOCH_PAYOUT_OPERATION_SCHEMA,
        EPOCH_PAYOUT_VERSION,
    };
    use ron_proto::{
        ContentId, EpochEligibilityStatusV1, EpochEligibilityV1, EpochQuorumThresholdV1,
        EpochRewardAllocationV1, QuickChainAccountingSnapshotReferenceV1, QuickChainEventClassV1,
        QuickChainRewardPlanReferenceV1, RocEpochTransitionV1, ServiceNodeQuorumV1,
        ServiceNodeSignatureV1, SignatureAlg, EPOCH_REWARD_ALLOCATION_SCHEMA,
        QUICKCHAIN_ACCOUNTING_SNAPSHOT_REFERENCE_SCHEMA, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_REWARD_PLAN_REFERENCE_SCHEMA, ROC_EPOCH_TRANSITION_SCHEMA,
        ROC_EPOCH_TRANSITION_VERSION,
    };

    use super::{EconomicPipelineStatusError, EconomicPipelineStatusStore};

    fn cid(character: char) -> ContentId {
        format!("b3:{}", character.to_string().repeat(64))
            .parse()
            .expect("fixture content ID must parse")
    }

    fn accounting_snapshot() -> QuickChainAccountingSnapshotReferenceV1 {
        QuickChainAccountingSnapshotReferenceV1 {
            schema: QUICKCHAIN_ACCOUNTING_SNAPSHOT_REFERENCE_SCHEMA.to_string(),
            version: QUICKCHAIN_DTO_VERSION,
            chain_id: "rustyonions-dev".to_string(),
            snapshot_id: "snapshot:epoch:20".to_string(),
            snapshot_root: cid('1'),
            window_started_at_ms: 1_000,
            window_ended_at_ms: 2_000,
            sealed_at_ms: 3_000,
            source_event_count: 1,
            economic_receipt_count: 0,
            metering_count: 0,
            proof_eligible_count: 1,
            ad_budgeted_count: 0,
            analytics_only_count: 0,
        }
    }

    fn reward_plan() -> QuickChainRewardPlanReferenceV1 {
        QuickChainRewardPlanReferenceV1 {
            schema: QUICKCHAIN_REWARD_PLAN_REFERENCE_SCHEMA.to_string(),
            version: QUICKCHAIN_DTO_VERSION,
            chain_id: "rustyonions-dev".to_string(),
            plan_id: "reward-plan:epoch:20".to_string(),
            plan_root: cid('2'),
            snapshot_id: "snapshot:epoch:20".to_string(),
            snapshot_root: cid('1'),
            source_event_class: QuickChainEventClassV1::ProofEligible,
            planned_total_minor: "1000".to_string(),
            payout_candidate_count: 1,
            capped_by_policy: true,
            verification_ref: Some("verification:epoch:20".to_string()),
            funding_budget_ref: None,
            produced_at_ms: 4_000,
        }
    }

    fn eligibility(
        service_node_id: &str,
        registry_entry_id: &str,
        reward_binding_id: &str,
        key_id: &str,
    ) -> EpochEligibilityV1 {
        EpochEligibilityV1 {
            version: ROC_EPOCH_TRANSITION_VERSION,
            service_node_id: service_node_id.to_string(),
            registry_entry_id: registry_entry_id.to_string(),
            reward_binding_id: reward_binding_id.to_string(),
            key_id: key_id.to_string(),
            status: EpochEligibilityStatusV1::Eligible,
        }
    }

    fn signature(
        service_node_id: &str,
        key_id: &str,
        transition_hash: &ContentId,
    ) -> ServiceNodeSignatureV1 {
        ServiceNodeSignatureV1 {
            version: ROC_EPOCH_TRANSITION_VERSION,
            chain_id: "rustyonions-dev".to_string(),
            epoch_id: "epoch:20".to_string(),
            service_node_id: service_node_id.to_string(),
            key_id: key_id.to_string(),
            algorithm: SignatureAlg::Ed25519,
            transition_hash: transition_hash.clone(),
            signature_wire: format!("signature:{service_node_id}"),
        }
    }

    fn epoch_transition() -> RocEpochTransitionV1 {
        let transition_hash = cid('8');

        let eligibilities = vec![
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
        ];

        let signatures = vec![
            signature("service_node:alpha", "key:alpha", &transition_hash),
            signature("service_node:beta", "key:beta", &transition_hash),
        ];

        RocEpochTransitionV1 {
            schema: ROC_EPOCH_TRANSITION_SCHEMA.to_string(),
            version: ROC_EPOCH_TRANSITION_VERSION,
            chain_id: "rustyonions-dev".to_string(),
            epoch_id: "epoch:20".to_string(),
            transition_hash: transition_hash.clone(),
            accounting_snapshot_hash: cid('1'),
            reward_plan_hash: cid('2'),
            policy_hash: cid('3'),
            economics_config_hash: cid('4'),
            registry_root: cid('5'),
            reward_binding_root: cid('6'),
            evidence_root: cid('7'),
            reward_cap_minor_units: "1000".to_string(),
            reward_total_minor_units: "1000".to_string(),
            allocations: vec![EpochRewardAllocationV1 {
                schema: EPOCH_REWARD_ALLOCATION_SCHEMA.to_string(),
                version: ROC_EPOCH_TRANSITION_VERSION,
                allocation_id: "allocation:alpha".to_string(),
                reward_plan_allocation_id: "reward-plan-allocation:alpha".to_string(),
                service_node_id: "service_node:alpha".to_string(),
                source_pool: "node_delivery".to_string(),
                amount_minor_units: "1000".to_string(),
            }],
            quorum: ServiceNodeQuorumV1 {
                schema: "ron.service_node.quorum.v1".to_string(),
                version: ROC_EPOCH_TRANSITION_VERSION,
                chain_id: "rustyonions-dev".to_string(),
                epoch_id: "epoch:20".to_string(),
                transition_hash,
                threshold: EpochQuorumThresholdV1 {
                    version: ROC_EPOCH_TRANSITION_VERSION,
                    eligible_service_nodes: 2,
                    quorum_bps: 10_000,
                    minimum_signatures: 2,
                    required_signatures: 2,
                },
                eligibilities,
                signatures,
            },
            produced_at_ms: 5_000,
        }
    }

    fn payout_operation(transition: &RocEpochTransitionV1) -> EpochPayoutOperationV1 {
        EpochPayoutOperationV1 {
            schema: EPOCH_PAYOUT_OPERATION_SCHEMA.to_string(),
            version: EPOCH_PAYOUT_VERSION,
            operation_id: "epoch_payout:phase20-alpha".to_string(),
            idempotency_key: "epoch-payout-phase20-alpha".to_string(),
            epoch_id: transition.epoch_id.clone(),
            source_pool: "node_delivery".to_string(),
            service_node_id: Some("service_node:alpha".to_string()),
            recipient_account_id: "acct_phase20_alpha".to_string(),
            amount_minor: "1000".to_string(),
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
        }
    }

    fn payout_receipt(transition: &RocEpochTransitionV1) -> EpochPayoutReceiptV1 {
        EpochPayoutReceiptV1::new(
            payout_operation(transition),
            41,
            "a".repeat(64),
            transition.produced_at_ms,
        )
        .expect("canonical payout receipt fixture must validate")
    }

    #[test]
    fn canonical_economic_pipeline_preserves_exact_roots_and_non_authority() {
        let store = EconomicPipelineStatusStore::default();

        store
            .replace(
                accounting_snapshot(),
                Some(reward_plan()),
                Some(epoch_transition()),
                None,
            )
            .expect("matching canonical pipeline should validate");

        let snapshot = store.snapshot().expect("economic snapshot");

        assert_eq!(snapshot.accounting_snapshot.snapshot_root, cid('1'));
        assert_eq!(
            snapshot
                .reward_plan
                .as_ref()
                .expect("reward plan")
                .plan_root,
            cid('2')
        );
        assert!(snapshot.epoch_payout_receipts.is_none());
        assert!(snapshot.epoch_payout_replay.is_none());

        let mut mismatched = epoch_transition();
        mismatched.reward_plan_hash = cid('9');

        let error = store
            .replace(
                accounting_snapshot(),
                Some(reward_plan()),
                Some(mismatched),
                None,
            )
            .expect_err("mismatched plan root must be rejected");

        assert_eq!(
            error,
            EconomicPipelineStatusError::ReferenceMismatch {
                field: "epoch_transition.reward_plan_hash",
            }
        );

        assert_eq!(
            store
                .snapshot()
                .expect("prior snapshot remains")
                .epoch_transition
                .expect("prior transition")
                .reward_plan_hash,
            cid('2')
        );
    }

    #[test]
    fn canonical_economic_pipeline_accepts_durable_epoch_payout_receipts_without_finality() {
        let store = EconomicPipelineStatusStore::default();
        let transition = epoch_transition();
        let receipt = payout_receipt(&transition);

        store
            .replace(
                accounting_snapshot(),
                Some(reward_plan()),
                Some(transition.clone()),
                Some(vec![receipt.clone()]),
            )
            .expect("canonical payout receipt must validate");

        let snapshot = store.snapshot().expect("economic snapshot");
        let receipts = snapshot.epoch_payout_receipts.expect("accepted receipts");
        let replay = snapshot
            .epoch_payout_replay
            .expect("accepted receipt replay");

        assert_eq!(receipts, vec![receipt]);
        assert_eq!(replay.receipt_count, 1);
        assert_eq!(replay.total_issued_minor, 1000);
        assert_eq!(replay.last_ledger_seq, 41);
        assert_eq!(replay.last_ledger_root, "a".repeat(64));
        assert_eq!(replay.balances["acct_phase20_alpha"], 1000);

        let mut mismatched_operation = payout_operation(&transition);
        mismatched_operation.reward_plan_hash = cid('9');

        let mismatched_receipt = EpochPayoutReceiptV1::new(
            mismatched_operation,
            42,
            "b".repeat(64),
            transition.produced_at_ms,
        )
        .expect("internally valid mismatched receipt fixture");

        let error = store
            .replace(
                accounting_snapshot(),
                Some(reward_plan()),
                Some(transition),
                Some(vec![mismatched_receipt]),
            )
            .expect_err("receipt with wrong plan binding must reject");

        assert_eq!(
            error,
            EconomicPipelineStatusError::ReferenceMismatch {
                field: "epoch_payout.operation.reward_plan_hash",
            }
        );

        assert_eq!(
            store
                .snapshot()
                .expect("prior snapshot remains")
                .epoch_payout_replay
                .expect("prior replay remains")
                .last_ledger_seq,
            41
        );
    }
}
